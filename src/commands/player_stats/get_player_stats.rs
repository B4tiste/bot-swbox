use poise::serenity_prelude as serenity;
use poise::serenity_prelude::CreateSelectMenuKind;
use poise::CreateReply;
use serenity::{
    builder::{CreateActionRow, CreateSelectMenu, CreateSelectMenuOption},
    Error,
};

use crate::commands::register::utils::get_user_link;
use crate::commands::shared::logs::send_log;
use crate::commands::shared::player_alias::ALIAS_LOOKUP_MAP;
use crate::commands::{
    mob_stats::utils::get_swrt_settings,
    player_stats::utils::{
        create_player_embed, create_replay_image, format_opponent_monsters_worst5,
        format_player_ld_monsters_emojis, format_player_monsters, get_person_one_monster_list,
        get_rank_emojis_for_score, get_recent_replays, get_user_detail,
        parse_discord_mention_to_id, search_users, Player,
    },
    shared::{
        embed_error_handling::{create_embed_error, schedule_message_deletion},
        logs::get_server_name,
        models::LoggerDocument,
    },
};
use crate::{Data, API_TOKEN};

/// Résultat de résolution : id + (optionnel) handle du message à éditer (menu)
struct ResolvedPlayer<'a> {
    swrt_player_id: i64,
    reply_handle: Option<poise::ReplyHandle<'a>>,
}

/// 📂 Displays the RTA stats of the given player. (LD & most used monsters)
///
/// Usage: /get_player_stats
#[poise::command(slash_command)]
pub async fn get_player_stats(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Player name"] player_name: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let token = get_token()?;

    // 1) Resolve swrt_player_id (alias / search / select)
    let resolved = match resolve_player_id(&ctx, &token, &player_name).await? {
        Some(r) => r,
        None => {
            // resolve_player_id already replied + logged failure if needed
            return Ok(());
        }
    };

    // 2) Show stats (single pipeline)
    // Important : on passe le reply_handle, pour réutiliser le message du menu
    let result = show_player_stats(
        &ctx,
        &token,
        &resolved.swrt_player_id,
        resolved.reply_handle,
    )
    .await;

    // 3) Log
    send_log(LoggerDocument::new(
        &ctx.author().name,
        &"get_player_stats".to_string(),
        &get_server_name(&ctx).await?,
        result.is_ok(),
        chrono::Utc::now().timestamp(),
    ))
    .await?;

    result
}

/// Centralized token retrieval
pub(crate) fn get_token() -> Result<String, Error> {
    let guard = API_TOKEN.lock().unwrap();
    guard.clone().ok_or_else(|| {
        Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Missing API Token, please contact **b4tiste** on Discord : <https://discord.gg/AfANrTVaDJ>.",
        ))
    })
}

async fn resolve_player_id<'a>(
    ctx: &'a poise::ApplicationContext<'a, Data, Error>,
    token: &str,
    player_name: &str,
) -> Result<Option<ResolvedPlayer<'a>>, Error> {
    // Mention Discord
    if let Some(discord_id) = parse_discord_mention_to_id(player_name) {
        let doc_opt = get_user_link(discord_id).await.map_err(|e| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("DB error: {e}"),
            ))
        })?;

        let Some(doc) = doc_opt else {
            ctx.say("❌ This Discord user has no linked account. They must use `/register <account name>` first.")
                .await?;
            return Ok(None);
        };

        let swrt_player_id = doc.get_i64("swrt_player_id").map_err(|_| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Invalid stored swrt_player_id in DB",
            ))
        })?;

        return Ok(Some(ResolvedPlayer {
            swrt_player_id,
            reply_handle: None,
        }));
    }

    // Alias direct
    if let Some(swrt_id) = ALIAS_LOOKUP_MAP.get(&player_name.to_lowercase()) {
        return Ok(Some(ResolvedPlayer {
            swrt_player_id: *swrt_id,
            reply_handle: None,
        }));
    }

    // Search
    let players = search_users(token, player_name).await.map_err(|e| {
        Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("API error: {}", e),
        ))
    })?;

    if players.is_empty() {
        ctx.say(format!("No players found for `{}`.", player_name))
            .await?;

        // log failure here because we return early without calling show_player_stats
        send_log(LoggerDocument::new(
            &ctx.author().name,
            &"get_player_stats".to_string(),
            &get_server_name(ctx).await?,
            false,
            chrono::Utc::now().timestamp(),
        ))
        .await?;

        return Ok(None);
    }

    if players.len() == 1 {
        return Ok(Some(ResolvedPlayer {
            swrt_player_id: players[0].swrt_player_id,
            reply_handle: None,
        }));
    }

    // Multiple => select menu
    let selected = select_player_from_menu(ctx, &players).await?;
    Ok(selected.map(|(id, handle)| ResolvedPlayer {
        swrt_player_id: id,
        reply_handle: Some(handle),
    }))
}

/// Build select menu, wait for interaction, return selected swrt_player_id + le handle du message
async fn select_player_from_menu<'a>(
    ctx: &'a poise::ApplicationContext<'a, Data, Error>,
    players: &[Player],
) -> Result<Option<(i64, poise::ReplyHandle<'a>)>, Error> {
    let options: Vec<CreateSelectMenuOption> = players
        .iter()
        .take(25)
        .map(|player| {
            let emoji = if player.player_country.to_uppercase() == "UNKNOWN" {
                serenity::ReactionType::Unicode("❌".to_string())
            } else {
                serenity::ReactionType::Unicode(country_code_to_flag_emoji(&player.player_country))
            };

            let description = format!(
                "Elo : {} - Server : {}",
                player.player_score.unwrap_or(0),
                server_code_to_tag(player.player_server)
            );

            CreateSelectMenuOption::new(&player.name, player.swrt_player_id.to_string())
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
                "🧙 Several players match the given username, please select a player :".to_string(),
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
            &"get_player_stats".to_string(),
            &get_server_name(ctx).await?,
            false,
            chrono::Utc::now().timestamp(),
        ))
        .await?;

        return Ok(None);
    };

    // Ack interaction (update message)
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
            values.get(0).cloned().unwrap_or_default()
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
    token: &str,
    swrt_id: &i64,
    existing_reply: Option<poise::ReplyHandle<'a>>,
) -> Result<(), Error> {
    // Fetch details
    let details = get_user_detail(token, swrt_id).await.map_err(|e| {
        Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error retrieving player details: {}", e),
        ))
    })?;

    // Rank emojis
    let rank_emojis = get_rank_emojis_for_score(details.player_level.unwrap_or(0))
        .await
        .unwrap_or_else(|_| "❓".to_string());

    // 1) Embed loading
    let loading_embed = create_player_embed(
        &details,
        vec!["<a:loading:1358029412716515418> Loading emojis...".to_string()],
        vec!["<a:loading:1358029412716515418> Loading top monsters...".to_string()],
        vec!["<a:loading:1358029412716515418> Loading opponent monsters...".to_string()],
        rank_emojis.clone(),
        0,
    );

    let reply_handle = match existing_reply {
        Some(handle) => {
            handle
                .edit(
                    poise::Context::Application(*ctx),
                    CreateReply {
                        content: None,
                        embeds: vec![loading_embed],
                        components: Some(vec![]),
                        attachments: vec![],
                        ..Default::default()
                    },
                )
                .await?;
            handle
        }
        None => {
            ctx.send(CreateReply {
                embeds: vec![loading_embed],
                ..Default::default()
            })
            .await?
        }
    };

    // 2) Load extras + replay image
    let ld_emojis = format_player_ld_monsters_emojis(&details).await;
    let top_monsters = format_player_monsters(&details).await;

    let season = match get_swrt_settings(&token).await {
        Ok(data) => data,
        Err(e) => {
            let reply = ctx.send(create_embed_error(&e)).await?;
            schedule_message_deletion(reply, *ctx).await?;
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"get_mob_stats".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            return Ok(());
        }
    };

    // Opponent monsters
    let person_list = get_person_one_monster_list(token, details.swrt_player_id, season)
        .await
        .map_err(|e| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error retrieving opponent monsters: {}", e),
            ))
        })?;

    let worst_opponents = format_opponent_monsters_worst5(&details, &person_list).await;

    let recent_replays = get_recent_replays(token, &details.swrt_player_id)
        .await
        .map_err(|e| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error retrieving recent replays: {}", e),
            ))
        })?;

    let replay_image_path = create_replay_image(recent_replays, 3, 2)
        .await
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let attachment = serenity::CreateAttachment::path(replay_image_path).await?;

    // 3) Edit with final embed
    let updated_embed = create_player_embed(
        &details,
        ld_emojis,
        top_monsters,
        worst_opponents,
        rank_emojis,
        1,
    );

    reply_handle
        .edit(
            poise::Context::Application(*ctx),
            CreateReply {
                content: Some("".to_string()),
                embeds: vec![updated_embed],
                attachments: vec![attachment],
                components: Some(vec![]),
                ..Default::default()
            },
        )
        .await?;

    Ok(())
}

fn country_code_to_flag_emoji(country_code: &str) -> String {
    country_code
        .to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| char::from_u32(0x1F1E6 + (c as u32 - 'A' as u32)).unwrap_or('∅'))
        .collect()
}

fn server_code_to_tag(code: i32) -> &'static str {
    match code {
        1 => "Korea",
        2 => "Japan",
        3 => "China",
        4 => "Global",
        5 => "Asia",
        6 => "Europe",
        _ => "??",
    }
}
