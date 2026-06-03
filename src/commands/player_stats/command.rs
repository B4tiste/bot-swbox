use poise::serenity_prelude as serenity;
use poise::serenity_prelude::CreateSelectMenuKind;
use poise::CreateReply;
use serenity::{
    builder::{CreateActionRow, CreateSelectMenu, CreateSelectMenuOption},
    builder::{EditAttachments, EditInteractionResponse},
    Error,
};

use crate::commands::register::utils::get_user_link;
use crate::commands::shared::logs::send_log;
use crate::commands::shared::player_alias::ALIAS_LOOKUP_MAP;
use crate::commands::{
    player_stats::utils::{
        create_lucksack_player_embed, create_lucksack_replay_image,
        format_lucksack_ld_monsters_emojis, format_lucksack_top_monsters,
        get_lucksack_player_ld5_box, get_lucksack_player_matches, get_lucksack_player_picks,
        get_lucksack_player_summary, get_lucksack_season_numbers, get_rank_emojis_for_bracket,
        parse_discord_mention_to_id, search_players_lucksack, LucksackSearchPlayer,
    },
    shared::{
        embed_error_handling::{create_embed_error, schedule_message_deletion},
        logs::get_server_name,
        models::LoggerDocument,
    },
};
use crate::Data;

const REPLAY_PAGE_SIZE: usize = 6;
const PLAYER_STATS_LOADING_REPLAY_GIF_PATH: &str =
    "assets/loading/player_stats_loading_1350x800.gif";
const PLAYER_STATS_LOADING_REPLAY_GIF_FALLBACK_URL: &str = "https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExczN3N3YxcjAzc3g5bWpqY2VleXA2MHN0bm9rcDVvaG00MGZrbHoweSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/2WjpfxAI5MvC9Nl8U7/giphy.gif";

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
    let players = search_players_lucksack(player_name)
        .await
        .map_err(|e| Error::from(std::io::Error::other(format!("API error: {}", e))))?;

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
    // Fetch season numbers from lucksack
    let seasons = match get_lucksack_season_numbers().await {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("❌ Failed to fetch seasons: {}", e);
            let reply = ctx.send(create_embed_error(&msg)).await?;
            schedule_message_deletion(reply, *ctx).await?;
            return Ok(());
        }
    };

    let Some(&season) = seasons.first() else {
        let reply = ctx
            .send(create_embed_error("❌ No valid season number found."))
            .await?;
        schedule_message_deletion(reply, *ctx).await?;
        return Ok(());
    };

    // --- Step 1: fetch summary + picks, show initial embed with loading gif ---
    let (summary_res, picks_res) = tokio::join!(
        get_lucksack_player_summary(player_id, season),
        get_lucksack_player_picks(player_id, season),
    );

    let summary = summary_res.map_err(|e| {
        Error::from(std::io::Error::other(format!(
            "Error retrieving player summary: {}",
            e
        )))
    })?;

    let picks = picks_res.unwrap_or_default();

    let mut ld_box = Vec::new();
    for season_number in &seasons {
        if let Ok(mut season_box) = get_lucksack_player_ld5_box(player_id, *season_number).await {
            ld_box.append(&mut season_box);
        }
    }

    let top_monsters = format_lucksack_top_monsters(&picks).await;
    let ld_monsters = format_lucksack_ld_monsters_emojis(&ld_box).await;
    let rank_emojis = get_rank_emojis_for_bracket(summary.summary.current_rank_bracket);
    let total_matches = summary.summary.total_matches.max(0) as usize;
    let last_replay_page = total_matches.div_ceil(REPLAY_PAGE_SIZE).max(1) as i32;
    let mut replay_page = 1i32;

    let loading_gif_attachment = load_player_stats_loading_gif_attachment().await;
    let loading_gif_image_ref = if loading_gif_attachment.is_some() {
        "attachment://loading.gif"
    } else {
        PLAYER_STATS_LOADING_REPLAY_GIF_FALLBACK_URL
    };

    let initial_embed = create_lucksack_player_embed(
        &summary,
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
                        attachments: loading_gif_attachment.into_iter().collect(),
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
                    rank_emojis.clone(),
                    top_monsters.clone(),
                    ld_monsters.clone(),
                )
                .image(loading_gif_image_ref)],
                attachments: load_player_stats_loading_gif_attachment()
                    .await
                    .into_iter()
                    .collect(),
                ..Default::default()
            })
            .await?
        }
    };

    // --- Step 2: fetch matches, generate replay image, update embed ---
    let matches = get_lucksack_player_matches(player_id, season, REPLAY_PAGE_SIZE, 0)
        .await
        .unwrap_or_default();

    let replay_image_path = if !matches.is_empty() {
        // println!("Generating replay image for {} matches...", matches.len());
        // let start = std::time::Instant::now();
        let result = create_lucksack_replay_image(&matches).await.ok();
        // let duration = start.elapsed();
        // println!("Replay image generation took: {:?}", duration);
        result
    } else {
        None
    };

    let final_embed = {
        let mut e = create_lucksack_player_embed(
            &summary,
            rank_emojis.clone(),
            top_monsters.clone(),
            ld_monsters.clone(),
        );
        if replay_image_path.is_some() {
            e = e.image("attachment://replay.png");
        }
        e = e.field(
            "Recent Replays",
            format!("Page {}/{}", replay_page, last_replay_page),
            false,
        );
        e
    };

    let mut final_reply = CreateReply {
        content: Some("".to_string()),
        embeds: vec![final_embed],
        components: Some(if last_replay_page > 1 {
            vec![create_replay_pagination_buttons(
                replay_page,
                last_replay_page,
                false,
            )]
        } else {
            vec![]
        }),
        ..Default::default()
    };
    if let Some(ref path) = replay_image_path {
        if let Ok(attachment) = serenity::CreateAttachment::path(path).await {
            final_reply.attachments.push(attachment);
        }
    }
    reply_handle
        .edit(poise::Context::Application(*ctx), final_reply)
        .await?;

    if last_replay_page <= 1 {
        return Ok(());
    }

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
        match interaction.data.custom_id.as_str() {
            "player_stats_replays_previous_page" if replay_page > 1 => replay_page -= 1,
            "player_stats_replays_next_page" if replay_page < last_replay_page => replay_page += 1,
            _ => continue,
        }

        let mut loading_embed = create_lucksack_player_embed(
            &summary,
            rank_emojis.clone(),
            top_monsters.clone(),
            ld_monsters.clone(),
        )
        .image(loading_gif_image_ref);
        loading_embed = loading_embed.field(
            "Recent Replays",
            format!("Loading page {}/{}...", replay_page, last_replay_page),
            false,
        );

        let mut loading_message = serenity::CreateInteractionResponseMessage::new()
            .add_embed(loading_embed)
            .components(vec![create_replay_pagination_buttons(
                replay_page,
                last_replay_page,
                true,
            )]);

        if let Some(attachment) = load_player_stats_loading_gif_attachment().await {
            loading_message = loading_message.add_file(attachment);
        }

        interaction
            .create_response(
                &ctx.serenity_context,
                serenity::CreateInteractionResponse::UpdateMessage(loading_message),
            )
            .await?;

        let offset = ((replay_page - 1) as usize) * REPLAY_PAGE_SIZE;
        let matches = get_lucksack_player_matches(player_id, season, REPLAY_PAGE_SIZE, offset)
            .await
            .unwrap_or_default();

        let replay_image_path = if !matches.is_empty() {
            create_lucksack_replay_image(&matches).await.ok()
        } else {
            None
        };

        let updated_embed = {
            let mut e = create_lucksack_player_embed(
                &summary,
                rank_emojis.clone(),
                top_monsters.clone(),
                ld_monsters.clone(),
            );
            if replay_image_path.is_some() {
                e = e.image("attachment://replay.png");
            }
            e.field(
                "Recent Replays",
                format!("Page {}/{}", replay_page, last_replay_page),
                false,
            )
        };

        let mut response = EditInteractionResponse::new()
            .embeds(vec![updated_embed])
            .components(vec![create_replay_pagination_buttons(
                replay_page,
                last_replay_page,
                false,
            )])
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

    reply_handle
        .edit(
            poise::Context::Application(*ctx),
            CreateReply {
                components: Some(vec![create_replay_pagination_buttons(
                    replay_page,
                    last_replay_page,
                    true,
                )]),
                ..Default::default()
            },
        )
        .await?;

    Ok(())
}

async fn load_player_stats_loading_gif_attachment() -> Option<serenity::CreateAttachment> {
    serenity::CreateAttachment::path(PLAYER_STATS_LOADING_REPLAY_GIF_PATH)
        .await
        .ok()
        .map(|mut attachment| {
            attachment.filename = "loading.gif".to_string();
            attachment
        })
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
