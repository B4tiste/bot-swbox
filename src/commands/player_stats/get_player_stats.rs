use poise::serenity_prelude as serenity;
use poise::serenity_prelude::CreateSelectMenuKind;
use poise::CreateReply;
use serenity::{
    builder::{CreateActionRow, CreateSelectMenu, CreateSelectMenuOption},
    Error,
};

use crate::commands::player_stats::utils::{
    create_player_embed, format_player_ld_monsters_emojis, format_player_monsters, get_user_detail,
    search_users,
};
use crate::commands::shared::logs::send_log;
use crate::{Data, API_TOKEN};

/// 📂 Displays the RTA stats of the given player. (LD & most used monsters)
///
/// Usage: `/get_player_stats`
#[poise::command(slash_command)]
pub async fn get_player_stats(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Player name"] player_name: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let token = {
        let guard = API_TOKEN.lock().unwrap();
        guard.clone().ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing API token",
            ))
        })?
    };

    let players = search_users(&token, &player_name).await.map_err(|e| {
        Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("API error: {}", e),
        ))
    })?;

    if players.is_empty() {
        ctx.say("No players found.").await?;
        send_log(
            &ctx,
            "Command: /get_player_stats".to_string(),
            false,
            format!("No players found for '{}'", player_name),
        )
        .await?;
        return Ok(());
    }

    if players.len() == 1 {
        let details = get_user_detail(&token, &players[0].swrt_player_id)
            .await
            .map_err(|e| {
                Error::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Error retrieving player details: {}", e),
                ))
            })?;

        let embed = create_player_embed(
            &details,
            vec!["<a:loading:1358029412716515418> Loading emojis...".to_string()],
            vec!["<a:loading:1358029412716515418> Loading top monsters...".to_string()],
        );
        let reply_handle = ctx
            .send(CreateReply {
                embeds: vec![embed],
                ..Default::default()
            })
            .await?;

        let ld_emojis = format_player_ld_monsters_emojis(&details).await;
        let top_monsters = format_player_monsters(&details).await;

        let updated_embed = create_player_embed(&details, ld_emojis, top_monsters);
        reply_handle
            .edit(
                poise::Context::Application(ctx),
                CreateReply {
                    embeds: vec![updated_embed],
                    ..Default::default()
                },
            )
            .await?;

        send_log(
            &ctx,
            "Command: /get_player_stats".to_string(),
            true,
            format!("Displayed stats for '{}'", players[0].name),
        )
        .await?;

        return Ok(());
    }

    let options: Vec<CreateSelectMenuOption> = players
        .iter()
        .take(25)
        .map(|player| {
            let emoji =
                serenity::ReactionType::Unicode(country_code_to_flag_emoji(&player.player_country));
            let description = format!("Elo: {}", player.player_score.unwrap_or(0));

            CreateSelectMenuOption::new(&player.name, player.swrt_player_id.to_string())
                .description(description)
                .emoji(emoji)
        })
        .collect();

    let select_menu =
        CreateSelectMenu::new("select_player", CreateSelectMenuKind::String { options });
    let action_row = CreateActionRow::SelectMenu(select_menu);

    let msg = ctx
        .send(CreateReply {
            content: Some(
                "🧙 Several players match the given username, please select a player :".to_string(),
            ),
            components: Some(vec![action_row]),
            ..Default::default()
        })
        .await?;

    let user_id = ctx.author().id;
    if let Some(component_interaction) =
        serenity::ComponentInteractionCollector::new(&ctx.serenity_context.shard)
            .filter(move |i| i.data.custom_id == "select_player" && i.user.id == user_id)
            .timeout(std::time::Duration::from_secs(30))
            .await
    {
        component_interaction
            .create_response(
                &ctx.serenity_context,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::default(),
                ),
            )
            .await?;

        // Step 1: update message to show loading
        msg.edit(
            poise::Context::Application(ctx),
            CreateReply {
                content: Some("<a:loading:1358029412716515418> Retrieving data...".to_string()),
                components: Some(vec![]),
                embeds: vec![],
                ..Default::default()
            },
        )
        .await?;

        let selected_id = if let serenity::ComponentInteractionDataKind::StringSelect { values } =
            &component_interaction.data.kind
        {
            values.get(0).cloned().unwrap_or_default()
        } else {
            String::new()
        };

        let selected_id: i64 = selected_id.parse().map_err(|_| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid player ID format",
            ))
        })?;

        let details = get_user_detail(&token, &selected_id).await.map_err(|e| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error retrieving player details: {}", e),
            ))
        })?;

        // Embed initial loading state
        let embed = create_player_embed(
            &details,
            vec!["<a:loading:1358029412716515418> Loading emojis...".to_string()],
            vec!["<a:loading:1358029412716515418> Loading top monsters...".to_string()],
        );
        msg.edit(
            poise::Context::Application(ctx),
            CreateReply {
                content: Some("".to_string()),
                embeds: vec![embed],
                ..Default::default()
            },
        )
        .await?;

        // Step 2: load emojis + monsters
        let ld_emojis = format_player_ld_monsters_emojis(&details).await;
        let top_monsters = format_player_monsters(&details).await;

        let updated_embed = create_player_embed(&details, ld_emojis, top_monsters);
        msg.edit(
            poise::Context::Application(ctx),
            CreateReply {
                content: Some("".to_string()),
                embeds: vec![updated_embed],
                ..Default::default()
            },
        )
        .await?;

        send_log(
            &ctx,
            "Command: /get_player_stats".to_string(),
            true,
            format!("Displayed stats after selection for '{}'", player_name),
        )
        .await?;
    } else {
        ctx.say("⏰ Time expired or no selection.").await?;
        send_log(
            &ctx,
            "Command: /get_player_stats".to_string(),
            false,
            format!("Timeout or no selection for '{}'", player_name),
        )
        .await?;
    }

    Ok(())
}

fn country_code_to_flag_emoji(country_code: &str) -> String {
    country_code
        .to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| {
            char::from_u32(0x1F1E6 + (c as u32 - 'A' as u32)).unwrap_or('∅')
        })
        .collect()
}