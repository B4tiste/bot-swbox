use poise::serenity_prelude as serenity;
use poise::serenity_prelude::CreateSelectMenuKind;
use poise::CreateReply;
use serenity::{
    builder::{CreateActionRow, CreateSelectMenu, CreateSelectMenuOption},
    Error,
};

use crate::commands::player_stats::get_player_stats::get_token;
use crate::commands::player_stats::utils::search_users;
use crate::commands::register::utils::upsert_user_link;
use crate::Data;

#[derive(Debug, Clone)]
struct RegisterCandidate {
    swrt_player_id: i64,
    name: String,
    server: i32,
    country: String,
    score: i32,
}

/// 📂 Link an in-game account to your Discord profile
///
/// Usage: /register [account_name]
#[poise::command(slash_command)]
pub async fn register(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Account name"] account_name: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    // 1) On envoie un message "placeholder" qu'on éditera ensuite
    let msg_handle = ctx
        .send(CreateReply {
            content: Some(format!(
                "<a:loading:1358029412716515418> Searching accounts for `{}`...",
                account_name
            )),
            ..Default::default()
        })
        .await?;

    let token = get_token()?;

    let players = search_users(&token, &account_name).await.map_err(|e| {
        Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("API error: {e}"),
        ))
    })?;

    if players.is_empty() {
        msg_handle
            .edit(
                poise::Context::Application(ctx),
                CreateReply {
                    content: Some(format!("No players found for `{}`.", account_name)),
                    components: Some(vec![]),
                    embeds: vec![],
                    ..Default::default()
                },
            )
            .await?;
        return Ok(());
    }

    let candidates: Vec<RegisterCandidate> = players
        .into_iter()
        .map(|p| RegisterCandidate {
            swrt_player_id: p.swrt_player_id,
            name: p.name,
            server: p.player_server,
            country: p.player_country,
            score: p.player_score.unwrap_or(0),
        })
        .collect();

    // 2) Si plusieurs, on édite le message initial pour afficher le select menu
    let picked = if candidates.len() == 1 {
        candidates[0].swrt_player_id
    } else {
        match select_player_from_menu_editing(&ctx, &msg_handle, &candidates).await? {
            Some(id) => id,
            None => {
                // timeout => on édite le message initial
                msg_handle
                    .edit(
                        poise::Context::Application(ctx),
                        CreateReply {
                            content: Some("⏰ Time expired or no selection.".to_string()),
                            components: Some(vec![]),
                            embeds: vec![],
                            ..Default::default()
                        },
                    )
                    .await?;
                return Ok(());
            }
        }
    };

    // Find selected candidate to save metadata (name/server/country)
    let selected = candidates
        .iter()
        .find(|c| c.swrt_player_id == picked)
        .cloned()
        .unwrap_or(RegisterCandidate {
            swrt_player_id: picked,
            name: account_name.clone(),
            server: 0,
            country: "UNKNOWN".to_string(),
            score: 0,
        });

    // 3) Edit "Saving link..."
    msg_handle
        .edit(
            poise::Context::Application(ctx),
            CreateReply {
                content: Some("<a:loading:1358029412716515418> Saving link...".to_string()),
                components: Some(vec![]),
                embeds: vec![],
                ..Default::default()
            },
        )
        .await?;

    let discord_user_id = ctx.author().id.get();
    let now_ts = chrono::Utc::now().timestamp();

    upsert_user_link(
        discord_user_id,
        selected.swrt_player_id,
        &selected.name,
        selected.server,
        &selected.country,
        now_ts,
    )
    .await
    .map_err(|e| {
        Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("DB error: {e}"),
        ))
    })?;

    // 4) Edit final success message (pas de nouveau message envoyé)
    msg_handle
        .edit(
            poise::Context::Application(ctx),
            CreateReply {
                content: Some(format!(
                    "✅ Registered **{}** to your Discord account. Now {} can use the command `/my_stats` to see their stats! And other can use `/get_player_stats @{}` to access their stats.",
                    selected.name,
                    ctx.author().name,
                    ctx.author().name,
                )),
                components: Some(vec![]),
                embeds: vec![],
                ..Default::default()
            },
        )
        .await?;

    Ok(())
}

/// Identique à ton select menu, mais au lieu de ctx.send(...), on édite `msg_handle`
async fn select_player_from_menu_editing(
    ctx: &poise::ApplicationContext<'_, Data, Error>,
    msg_handle: &poise::ReplyHandle<'_>,
    players: &[RegisterCandidate],
) -> Result<Option<i64>, Error> {
    let options: Vec<CreateSelectMenuOption> = players
        .iter()
        .take(25)
        .map(|p| {
            let emoji = if p.country.to_uppercase() == "UNKNOWN" {
                serenity::ReactionType::Unicode("❌".to_string())
            } else {
                serenity::ReactionType::Unicode(country_code_to_flag_emoji(&p.country))
            };

            let description = format!(
                "Elo : {} - Server : {}",
                p.score,
                server_code_to_tag(p.server)
            );

            CreateSelectMenuOption::new(&p.name, p.swrt_player_id.to_string())
                .description(description)
                .emoji(emoji)
        })
        .collect();

    let select_menu = CreateSelectMenu::new(
        "register_select_player",
        CreateSelectMenuKind::String { options },
    );
    let action_row = CreateActionRow::SelectMenu(select_menu);

    // 👉 ici on EDIT au lieu d'envoyer un nouveau message
    msg_handle
        .edit(
            poise::Context::Application(*ctx),
            CreateReply {
                content: Some("🧙 Select the account to link to your Discord:".to_string()),
                components: Some(vec![action_row]),
                embeds: vec![],
                ..Default::default()
            },
        )
        .await?;

    let user_id = ctx.author().id;

    let interaction = serenity::ComponentInteractionCollector::new(&ctx.serenity_context.shard)
        .filter(move |i| i.data.custom_id == "register_select_player" && i.user.id == user_id)
        .timeout(std::time::Duration::from_secs(30))
        .await;

    let Some(component_interaction) = interaction else {
        return Ok(None);
    };

    // Ack interaction
    component_interaction
        .create_response(
            &ctx.serenity_context,
            serenity::CreateInteractionResponse::UpdateMessage(
                serenity::CreateInteractionResponseMessage::default(),
            ),
        )
        .await?;

    // Remove components quickly after selection (UX)
    msg_handle
        .edit(
            poise::Context::Application(*ctx),
            CreateReply {
                content: Some(
                    "<a:loading:1358029412716515418> Processing selection...".to_string(),
                ),
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

    Ok(Some(selected_id))
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
