use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serenity::{
    builder::{CreateEmbed, CreateEmbedFooter},
    Error,
};

use crate::commands::leaderboard::utils::{get_leaderboard_data, LeaderboardPlayer};
use crate::{Data, API_TOKEN};

#[poise::command(slash_command)]
pub async fn get_leaderboard(
    ctx: poise::ApplicationContext<'_, Data, Error>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let user_id = ctx.author().id;
    let mut page = 1;

    // Récupération du token
    let token = {
        let guard = API_TOKEN.lock().unwrap();
        guard.clone().ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing API token",
            ))
        })?
    };

    // Récupération des joueurs de la première page
    let players = get_leaderboard_data(&token, &page).await.map_err(|e| {
        Error::from(std::io::Error::new(std::io::ErrorKind::Other, format!("API error: {}", e)))
    })?;

    // Envoi du message initial + récupération du message ID
    let response = ctx
        .send(CreateReply {
            embeds: vec![build_leaderboard_embed(&players, page)],
            components: Some(create_pagination_buttons(page)),
            ..Default::default()
        })
        .await?;

    let message_id = response.message().await?.id;
    let channel_id = ctx.channel_id();

    // Boucle de gestion d'interaction (pagination)
    while let Some(interaction) =
        serenity::ComponentInteractionCollector::new(&ctx.serenity_context.shard)
            .channel_id(channel_id)
            .message_id(message_id)
            .filter(move |i| i.user.id == user_id)
            .timeout(std::time::Duration::from_secs(60))
            .await
    {
        match interaction.data.custom_id.as_str() {
            "previous_page" if page > 1 => page -= 1,
            "next_page" => page += 1,
            _ => continue,
        }

        let players = match get_leaderboard_data(&token, &page).await {
            Ok(p) => p,
            Err(e) => {
                ctx.say(format!("Failed to load page {}: {}", page, e)).await?;
                break;
            }
        };

        interaction
            .create_response(
                &ctx.serenity_context,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .add_embed(build_leaderboard_embed(&players, page))
                        .components(create_pagination_buttons(page)),
                ),
            )
            .await?;
    }

    Ok(())
}

/// Construit l'embed du leaderboard pour une page donnée
fn build_leaderboard_embed(players: &[LeaderboardPlayer], page: i32) -> serenity::CreateEmbed {
    let mut description = String::new();
    for (rank, player) in players.iter().enumerate() {
        let position = rank + 1 + ((page - 1) * 15) as usize;
        description.push_str(&format!(
            "{}. :flag_{}: {} - `{}`\n",
            position,
            player.player_country.to_lowercase(),
            player.player_elo,
            player.name
        ));
    }

    CreateEmbed::default()
        .title("Leaderboard")
        .description(description)
        .field(
            "💡 Tip",
            "Use `/get_player <player_name>` to get player details.",
            false,
        )
        .footer(CreateEmbedFooter::new("Use /send_suggestion to report issues."))
        .color(serenity::Colour::from_rgb(0, 255, 0))
}

/// Crée les boutons de pagination
fn create_pagination_buttons(page: i32) -> Vec<serenity::CreateActionRow> {
    let previous_button = serenity::CreateButton::new("previous_page")
        .label("⬅️ Previous")
        .style(serenity::ButtonStyle::Primary)
        .disabled(page <= 1);

    let next_button = serenity::CreateButton::new("next_page")
        .label("➡️ Next")
        .style(serenity::ButtonStyle::Primary);

    vec![serenity::CreateActionRow::Buttons(vec![
        previous_button,
        next_button,
    ])]
}
