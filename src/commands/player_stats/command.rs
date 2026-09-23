use poise::serenity_prelude as serenity;
use poise::serenity_prelude::CreateSelectMenuKind;
use poise::CreateReply;
use serenity::{
    builder::{CreateActionRow, CreateSelectMenu, CreateSelectMenuOption},
    builder::{EditAttachments, EditInteractionResponse, EditMessage},
    Error,
};

use crate::commands::register::utils::get_user_link;
use crate::commands::shared::logs::send_log;
use crate::commands::shared::player_alias::ALIAS_LOOKUP_MAP;
use crate::commands::{
    player_stats::utils::{
        create_lucksack_player_embed, create_lucksack_replay_image, empty_lucksack_summary,
        format_lucksack_ld_monsters_emojis, format_lucksack_top_monsters,
        get_lucksack_player_ld5_box, get_lucksack_player_matches, get_lucksack_player_picks,
        get_lucksack_player_summary, get_lucksack_seasons, get_rank_emojis_for_bracket,
        parse_discord_mention_to_id, search_players_lucksack, LucksackPlayerSummary,
        LucksackSearchPlayer, LucksackSeasonEntry,
    },
    shared::{
        embed_error_handling::{create_embed_error, schedule_message_deletion},
        logs::get_server_name,
        models::LoggerDocument,
    },
};
use crate::Data;

const REPLAY_PAGE_SIZE: usize = 6;
const SEASON_SELECT_CUSTOM_ID: &str = "player_stats_season_select";
const PLAYER_STATS_LOADING_REPLAY_GIF_URL: &str = "https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExczN3N3YxcjAzc3g5bWpqY2VleXA2MHN0bm9rcDVvaG00MGZrbHoweSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/2WjpfxAI5MvC9Nl8U7/giphy.gif";
const LUCKSACK_MAINTENANCE_MSG: &str = "Lucksack is under maintenance, please come back later or join the [Lucksack Discord server](https://discord.gg/teuQCDzTSp) to check the status of the website.";

fn is_maintenance_error(e: &str) -> bool {
    e.contains("502")
}

struct ResolvedPlayer<'a> {
    player_id: i64,
    reply_handle: Option<poise::ReplyHandle<'a>>,
}

/// 📂 Displays the RTA stats of the given player.
///
/// Usage: /get_player_stats
#[poise::command(slash_command)]
pub async fn get_player_stats(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Player name"] player_name: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let resolved = match resolve_player_id(&ctx, &player_name).await? {
        Some(r) => r,
        None => return Ok(()),
    };

    let result = show_player_stats(&ctx, resolved.player_id, resolved.reply_handle).await;

    send_log(LoggerDocument::new(
        &ctx.author().name,
        "get_player_stats",
        &get_server_name(&ctx).await?,
        result.is_ok(),
        chrono::Utc::now().timestamp(),
    ))
    .await?;

    result
}

async fn resolve_player_id<'a>(
    ctx: &'a poise::ApplicationContext<'a, Data, Error>,
    player_name: &str,
) -> Result<Option<ResolvedPlayer<'a>>, Error> {
    // Discord mention
    if let Some(discord_id) = parse_discord_mention_to_id(player_name) {
        let doc_opt = get_user_link(discord_id)
            .await
            .map_err(|e| Error::from(std::io::Error::other(format!("DB error: {e}"))))?;

        let Some(doc) = doc_opt else {
            ctx.say("❌ This Discord user has no linked account. They must use `/register <account name>` first.")
                .await?;
            return Ok(None);
        };

        let player_id = doc
            .get_i64("swrt_player_id")
            .map_err(|_| Error::from(std::io::Error::other("Invalid stored player_id in DB")))?;

        return Ok(Some(ResolvedPlayer {
            player_id,
            reply_handle: None,
        }));
    }

    // Alias lookup
    if let Some(&swrt_id) = ALIAS_LOOKUP_MAP.get(&player_name.to_lowercase()) {
        return Ok(Some(ResolvedPlayer {
            player_id: swrt_id,
            reply_handle: None,
        }));
    }

    // Lucksack search
    let players = search_players_lucksack(player_name).await.map_err(|e| {
        let msg = e.to_string();
        Error::from(std::io::Error::other(if is_maintenance_error(&msg) {
            LUCKSACK_MAINTENANCE_MSG.to_string()
        } else {
            format!("API error: {}", msg)
        }))
    })?;

    if players.is_empty() {
        ctx.say(format!("No players found for `{}`.", player_name))
            .await?;

        send_log(LoggerDocument::new(
            &ctx.author().name,
            "get_player_stats",
            &get_server_name(ctx).await?,
            false,
            chrono::Utc::now().timestamp(),
        ))
        .await?;

        return Ok(None);
    }

    if players.len() == 1 {
        return Ok(Some(ResolvedPlayer {
            player_id: players[0].player_id,
            reply_handle: None,
        }));
    }

    let selected = select_player_from_menu(ctx, &players).await?;
    Ok(selected.map(|(id, handle)| ResolvedPlayer {
        player_id: id,
        reply_handle: Some(handle),
    }))
}

async fn select_player_from_menu<'a>(
    ctx: &'a poise::ApplicationContext<'a, Data, Error>,
    players: &[LucksackSearchPlayer],
) -> Result<Option<(i64, poise::ReplyHandle<'a>)>, Error> {
    let options: Vec<CreateSelectMenuOption> = players
        .iter()
        .take(25)
        .map(|player| {
            let emoji = if player.country.to_uppercase() == "UNKNOWN" {
                serenity::ReactionType::Unicode("❌".to_string())
            } else {
                serenity::ReactionType::Unicode(country_code_to_flag_emoji(&player.country))
            };

            let score = player
                .current_score
                .map(|s| s.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let rank = player
                .current_rank
                .map(|r| format!("#{}", r))
                .unwrap_or_else(|| "N/A".to_string());

            let description = format!("Elo: {} | Rank: {}", score, rank);

            CreateSelectMenuOption::new(&player.username, player.player_id.to_string())
                .description(description)
                .emoji(emoji)
        })
        .collect();

    let select_menu =
        CreateSelectMenu::new("select_player", CreateSelectMenuKind::String { options });
    let action_row = CreateActionRow::SelectMenu(select_menu);

    let reply_handle = ctx
        .send(CreateReply {
            content: Some(
                "🧙 Several players match the given username, please select a player:".to_string(),
            ),
            components: Some(vec![action_row]),
            ..Default::default()
        })
        .await?;

    let user_id = ctx.author().id;

    let interaction = serenity::ComponentInteractionCollector::new(&ctx.serenity_context.shard)
        .filter(move |i| i.data.custom_id == "select_player" && i.user.id == user_id)
        .timeout(std::time::Duration::from_secs(30))
        .await;

    let Some(component_interaction) = interaction else {
        ctx.say("⏰ Time expired or no selection.").await?;

        send_log(LoggerDocument::new(
            &ctx.author().name,
            "get_player_stats",
            &get_server_name(ctx).await?,
            false,
            chrono::Utc::now().timestamp(),
        ))
        .await?;

        return Ok(None);
    };

    component_interaction
        .create_response(
            &ctx.serenity_context,
            serenity::CreateInteractionResponse::UpdateMessage(
                serenity::CreateInteractionResponseMessage::default(),
            ),
        )
        .await?;

    reply_handle
        .edit(
            poise::Context::Application(*ctx),
            CreateReply {
                content: Some("<a:loading:1358029412716515418> Retrieving data...".to_string()),
                components: Some(vec![]),
                embeds: vec![],
                ..Default::default()
            },
        )
        .await?;

    let selected_str = match &component_interaction.data.kind {
        serenity::ComponentInteractionDataKind::StringSelect { values } => {
            values.first().cloned().unwrap_or_default()
        }
        _ => String::new(),
    };

    let selected_id: i64 = selected_str.parse().map_err(|_| {
        Error::from(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid player ID format",
        ))
    })?;

    Ok(Some((selected_id, reply_handle)))
}

pub(crate) async fn show_player_stats<'a>(
    ctx: &'a poise::ApplicationContext<'a, Data, Error>,
    player_id: i64,
    existing_reply: Option<poise::ReplyHandle<'a>>,
) -> Result<(), Error> {
    // Fetch seasons (regular seasons and special leagues), most recent first.
    let seasons = match get_lucksack_seasons().await {
        Ok(s) => s,
        Err(e) => {
            let e_str = e.to_string();
            let msg = if is_maintenance_error(&e_str) {
                format!("❌ {}", LUCKSACK_MAINTENANCE_MSG)
            } else {
                format!("❌ Failed to fetch seasons: {}", e_str)
            };
            let reply = ctx.send(create_embed_error(&msg)).await?;
            schedule_message_deletion(reply, *ctx).await?;
            return Ok(());
        }
    };

    if seasons.is_empty() {
        let reply = ctx
            .send(create_embed_error("❌ No valid season found."))
            .await?;
        schedule_message_deletion(reply, *ctx).await?;
        return Ok(());
    }

    let mut season_index = 0usize;

    // --- Step 1: fetch summary + picks for the most recent season (SL or regular), show initial embed with loading gif ---
    let stats = fetch_season_stats(player_id, &seasons[season_index], None).await?;
    let mut summary = stats.summary;
    let mut top_monsters = stats.top_monsters;
    let mut rank_emojis = stats.rank_emojis;
    let mut total_matches = stats.total_matches;
    let mut last_replay_page = stats.last_replay_page;

    // Kept as a fallback so seasons without any match still display the player's identity.
    let reference_user_info = summary.user_info.clone();

    // The LD box (career Light/Dark 5★ picks) is cumulative across all regular seasons,
    // independent of the season currently selected in the dropdown.
    let mut regular_season_numbers: Vec<i32> =
        seasons.iter().filter_map(|s| s.season_number).collect();
    regular_season_numbers.sort_unstable();
    regular_season_numbers.dedup();

    let mut ld_box = Vec::new();
    for season_number in &regular_season_numbers {
        if let Ok(mut season_box) = get_lucksack_player_ld5_box(player_id, *season_number).await {
            ld_box.append(&mut season_box);
        }
    }
    let ld_monsters = format_lucksack_ld_monsters_emojis(&ld_box).await;

    let mut replay_page = 1i32;
    let loading_gif_image_ref = PLAYER_STATS_LOADING_REPLAY_GIF_URL;

    let initial_embed = create_lucksack_player_embed(
        &summary,
        &seasons[season_index].season_name,
        rank_emojis.clone(),
        top_monsters.clone(),
        ld_monsters.clone(),
    )
    .image(loading_gif_image_ref);

    let reply_handle = match existing_reply {
        Some(handle) => {
            handle
                .edit(
                    poise::Context::Application(*ctx),
                    CreateReply {
                        content: Some("".to_string()),
                        embeds: vec![initial_embed],
                        components: Some(vec![]),
                        ..Default::default()
                    },
                )
                .await?;
            handle
        }
        None => {
            ctx.send(CreateReply {
                embeds: vec![create_lucksack_player_embed(
                    &summary,
                    &seasons[season_index].season_name,
                    rank_emojis.clone(),
                    top_monsters.clone(),
                    ld_monsters.clone(),
                )
                .image(loading_gif_image_ref)],
                ..Default::default()
            })
            .await?
        }
    };

    // --- Step 2: fetch matches, generate replay image, update embed ---
    let matches = fetch_matches_for_page(player_id, &seasons[season_index], 0).await;

    let replay_image_path = if !matches.is_empty() {
        create_lucksack_replay_image(&matches).await.ok()
    } else {
        None
    };

    let replay_attachment_name = replay_image_path
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .map(|name| name.to_string());

    let final_embed = build_season_embed(SeasonEmbedArgs {
        summary: &summary,
        season_name: &seasons[season_index].season_name,
        rank_emojis: &rank_emojis,
        top_monsters: &top_monsters,
        ld_monsters: &ld_monsters,
        replay_attachment_name: replay_attachment_name.as_deref(),
        total_matches,
        replay_page,
        last_replay_page,
    });

    let mut final_message = EditMessage::new()
        .content("")
        .embeds(vec![final_embed])
        .components(build_components(
            &seasons,
            season_index,
            replay_page,
            last_replay_page,
            false,
        ))
        .attachments(EditAttachments::new());

    if let Some(ref path) = replay_image_path {
        if let Ok(attachment) = serenity::CreateAttachment::path(path).await {
            final_message = final_message.attachments(EditAttachments::new().add(attachment));
        }
    }

    let mut message = reply_handle.message().await?.into_owned();
    message
        .edit(&ctx.serenity_context.http, final_message)
        .await?;

    let message_id = reply_handle.message().await?.id;
    let channel_id = ctx.channel_id();
    let user_id = ctx.author().id;

    while let Some(interaction) =
        serenity::ComponentInteractionCollector::new(&ctx.serenity_context.shard)
            .channel_id(channel_id)
            .message_id(message_id)
            .filter(move |i| i.user.id == user_id)
            .timeout(std::time::Duration::from_secs(600))
            .await
    {
        let mut season_changed = false;

        match interaction.data.custom_id.as_str() {
            "player_stats_replays_previous_page" if replay_page > 1 => replay_page -= 1,
            "player_stats_replays_next_page" if replay_page < last_replay_page => replay_page += 1,
            SEASON_SELECT_CUSTOM_ID => {
                let selected_str = match &interaction.data.kind {
                    serenity::ComponentInteractionDataKind::StringSelect { values } => {
                        values.first().cloned().unwrap_or_default()
                    }
                    _ => String::new(),
                };

                let Ok(new_index) = selected_str.parse::<usize>() else {
                    continue;
                };

                if new_index >= seasons.len() {
                    continue;
                }

                season_index = new_index;
                replay_page = 1;
                season_changed = true;
            }
            _ => continue,
        }

        let loading_text = if season_changed {
            format!("Loading {}...", seasons[season_index].season_name)
        } else {
            format!("Loading page {}/{}...", replay_page, last_replay_page)
        };

        let loading_embed = create_lucksack_player_embed(
            &summary,
            &seasons[season_index].season_name,
            rank_emojis.clone(),
            top_monsters.clone(),
            ld_monsters.clone(),
        )
        .image(loading_gif_image_ref)
        .field("Recent Replays", loading_text, false);

        let loading_message = serenity::CreateInteractionResponseMessage::new()
            .add_embed(loading_embed)
            .components(build_components(
                &seasons,
                season_index,
                replay_page,
                last_replay_page,
                true,
            ));

        interaction
            .create_response(
                &ctx.serenity_context,
                serenity::CreateInteractionResponse::UpdateMessage(loading_message),
            )
            .await?;

        if season_changed {
            match fetch_season_stats(
                player_id,
                &seasons[season_index],
                Some(&reference_user_info),
            )
            .await
            {
                Ok(stats) => {
                    summary = stats.summary;
                    top_monsters = stats.top_monsters;
                    rank_emojis = stats.rank_emojis;
                    total_matches = stats.total_matches;
                    last_replay_page = stats.last_replay_page;
                }
                Err(e) => {
                    let error_embed = serenity::CreateEmbed::default()
                        .title("Error")
                        .description(format!("❌ {}", e))
                        .color(serenity::Colour::RED);
                    let response = EditInteractionResponse::new()
                        .embeds(vec![error_embed])
                        .components(vec![]);
                    let _ = interaction
                        .edit_response(&ctx.serenity_context.http, response)
                        .await;
                    continue;
                }
            }
        }

        let offset = ((replay_page - 1) as usize) * REPLAY_PAGE_SIZE;
        let matches = fetch_matches_for_page(player_id, &seasons[season_index], offset).await;

        let replay_image_path = if !matches.is_empty() {
            create_lucksack_replay_image(&matches).await.ok()
        } else {
            None
        };

        let replay_attachment_name = replay_image_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(|name| name.to_string());

        let updated_embed = build_season_embed(SeasonEmbedArgs {
            summary: &summary,
            season_name: &seasons[season_index].season_name,
            rank_emojis: &rank_emojis,
            top_monsters: &top_monsters,
            ld_monsters: &ld_monsters,
            replay_attachment_name: replay_attachment_name.as_deref(),
            total_matches,
            replay_page,
            last_replay_page,
        });

        let mut response = EditInteractionResponse::new()
            .embeds(vec![updated_embed])
            .components(build_components(
                &seasons,
                season_index,
                replay_page,
                last_replay_page,
                false,
            ))
            .attachments(EditAttachments::new());

        if let Some(path) = replay_image_path {
            if let Ok(attachment) = serenity::CreateAttachment::path(path).await {
                response = response.attachments(EditAttachments::new().add(attachment));
            }
        }

        interaction
            .edit_response(&ctx.serenity_context.http, response)
            .await?;
    }

    // Disabling the components after timeout is cosmetic; ignore permission errors
    // (e.g. "Missing access" in servers where the bot cannot edit interaction
    // responses via the REST API after the interaction token window closes).
    if let Ok(mut message) = reply_handle.message().await.map(|m| m.into_owned()) {
        let _ = message
            .edit(
                &ctx.serenity_context.http,
                EditMessage::new().components(build_components(
                    &seasons,
                    season_index,
                    replay_page,
                    last_replay_page,
                    true,
                )),
            )
            .await;
    }

    Ok(())
}

struct SeasonStats {
    summary: LucksackPlayerSummary,
    top_monsters: String,
    rank_emojis: String,
    total_matches: usize,
    last_replay_page: i32,
}

/// Fetches summary/picks for a season. If the request fails and a `fallback_user_info` is
/// provided, falls back to an all-zero summary so a season with no matches can still render.
async fn fetch_season_stats(
    player_id: i64,
    entry: &LucksackSeasonEntry,
    fallback_user_info: Option<&crate::commands::player_stats::utils::LucksackUserInfo>,
) -> Result<SeasonStats, Error> {
    let season = entry.query_season();
    let special_league = entry.is_special_league();

    let (summary_res, picks_res) = tokio::join!(
        get_lucksack_player_summary(player_id, season, special_league),
        get_lucksack_player_picks(player_id, season, special_league),
    );

    let summary = match summary_res {
        Ok(s) => s,
        Err(e) => {
            let Some(user_info) = fallback_user_info else {
                let msg = e.to_string();
                return Err(Error::from(std::io::Error::other(
                    if is_maintenance_error(&msg) {
                        LUCKSACK_MAINTENANCE_MSG.to_string()
                    } else {
                        format!("Error retrieving player summary: {}", msg)
                    },
                )));
            };
            empty_lucksack_summary(user_info.clone())
        }
    };

    let picks = picks_res.unwrap_or_default();
    let top_monsters = format_lucksack_top_monsters(&picks).await;
    let rank_emojis = get_rank_emojis_for_bracket(summary.summary.current_rank_bracket);
    let total_matches = summary.summary.total_matches.max(0) as usize;
    let last_replay_page = total_matches.div_ceil(REPLAY_PAGE_SIZE).max(1) as i32;

    Ok(SeasonStats {
        summary,
        top_monsters,
        rank_emojis,
        total_matches,
        last_replay_page,
    })
}

async fn fetch_matches_for_page(
    player_id: i64,
    entry: &LucksackSeasonEntry,
    offset: usize,
) -> Vec<crate::commands::player_stats::utils::LucksackMatch> {
    get_lucksack_player_matches(
        player_id,
        entry.query_season(),
        entry.is_special_league(),
        REPLAY_PAGE_SIZE,
        offset,
    )
    .await
    .unwrap_or_default()
}

struct SeasonEmbedArgs<'a> {
    summary: &'a LucksackPlayerSummary,
    season_name: &'a str,
    rank_emojis: &'a str,
    top_monsters: &'a str,
    ld_monsters: &'a str,
    replay_attachment_name: Option<&'a str>,
    total_matches: usize,
    replay_page: i32,
    last_replay_page: i32,
}

fn build_season_embed(args: SeasonEmbedArgs<'_>) -> serenity::CreateEmbed {
    let mut e = create_lucksack_player_embed(
        args.summary,
        args.season_name,
        args.rank_emojis.to_string(),
        args.top_monsters.to_string(),
        args.ld_monsters.to_string(),
    );

    if let Some(attachment_name) = args.replay_attachment_name {
        e = e.image(format!("attachment://{}", attachment_name));
    }

    let replays_text = if args.total_matches == 0 {
        "No matches recorded this season.".to_string()
    } else {
        format!("Page {}/{}", args.replay_page, args.last_replay_page)
    };

    e.field("Recent Replays", replays_text, false)
}

fn build_components(
    seasons: &[LucksackSeasonEntry],
    season_index: usize,
    replay_page: i32,
    last_replay_page: i32,
    disabled: bool,
) -> Vec<CreateActionRow> {
    let mut rows = vec![build_season_select_menu(seasons, season_index, disabled)];
    if last_replay_page > 1 {
        rows.push(create_replay_pagination_buttons(
            replay_page,
            last_replay_page,
            disabled,
        ));
    }
    rows
}

fn build_season_select_menu(
    seasons: &[LucksackSeasonEntry],
    selected_index: usize,
    disabled: bool,
) -> CreateActionRow {
    let options: Vec<CreateSelectMenuOption> = seasons
        .iter()
        .enumerate()
        .take(25)
        .map(|(idx, entry)| {
            let emoji = if entry.is_special_league() {
                serenity::ReactionType::Unicode("⚔️".to_string())
            } else {
                serenity::ReactionType::Unicode("🏆".to_string())
            };

            CreateSelectMenuOption::new(&entry.season_name, idx.to_string())
                .emoji(emoji)
                .default_selection(idx == selected_index)
        })
        .collect();

    let select_menu = CreateSelectMenu::new(
        SEASON_SELECT_CUSTOM_ID,
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Select a season")
    .disabled(disabled);

    CreateActionRow::SelectMenu(select_menu)
}

fn create_replay_pagination_buttons(
    page: i32,
    last_page: i32,
    disabled: bool,
) -> serenity::CreateActionRow {
    let previous_button = serenity::CreateButton::new("player_stats_replays_previous_page")
        .label("⬅️ Previous")
        .style(serenity::ButtonStyle::Primary)
        .disabled(disabled || page <= 1);

    let next_button = serenity::CreateButton::new("player_stats_replays_next_page")
        .label("➡️ Next")
        .style(serenity::ButtonStyle::Primary)
        .disabled(disabled || page >= last_page);

    serenity::CreateActionRow::Buttons(vec![previous_button, next_button])
}

fn country_code_to_flag_emoji(country_code: &str) -> String {
    country_code
        .to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| char::from_u32(0x1F1E6 + (c as u32 - 'A' as u32)).unwrap_or('∅'))
        .collect()
}
