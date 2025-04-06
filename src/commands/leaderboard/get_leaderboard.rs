use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serenity::{
    builder::{CreateEmbed, CreateEmbedFooter},
    Error,
};

use crate::commands::leaderboard::utils::get_leaderboard_data;
use crate::commands::leaderboard::utils::LeaderboardPlayer;
use crate::{Data, API_TOKEN};

#[poise::command(slash_command)]
pub async fn get_leaderboard(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    ctx.defer().await?;
    let user_id = ctx.author().id;

    let token = {
        let guard = API_TOKEN.lock().unwrap();
        guard.clone().ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing API token",
            ))
        })?
    };

    let mut page = 1;

    // Fonction pour générer l'embed
    let build_embed = |players: &[LeaderboardPlayer], page: i32| {
        let mut leaderboard_string = String::new();
        for (rank, player) in players.iter().enumerate() {
            leaderboard_string.push_str(&format!(
                "{}. :flag_{}: {} - `{}`\n",
                rank + 1 + ((page - 1) * 15) as usize,
                player.player_country.to_lowercase(),
                player.player_elo,
                player.name
            ));
        }

        CreateEmbed::default()
            .title("Leaderboard")
            .description(leaderboard_string)
            .footer(CreateEmbedFooter::new(
                "Use /send_suggestion to report issues.",
            ))
            .color(serenity::Colour::from_rgb(0, 255, 0))
    };

    let players = get_leaderboard_data(&token, &(page as i32))
        .await
        .map_err(|e| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("API error: {}", e),
            ))
        })?;

    // Envoie initial du message et sauvegarde du message ID
    let response = ctx
        .send(CreateReply {
            embeds: vec![build_embed(&players, page)],
            components: Some(create_pagination_buttons(page as i32)),
            ..Default::default()
        })
        .await?;

    let message_id = response.message().await?.id;
    let channel_id = ctx.channel_id();

    // Interaction loop
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

        let players = match get_leaderboard_data(&token, &(page as i32)).await {
            Ok(p) => p,
            Err(e) => {
                ctx.say(format!("Failed to load page {}: {}", page, e))
                    .await?;
                break;
            }
        };

        interaction
            .create_response(
                &ctx.serenity_context,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .add_embed(build_embed(&players, page))
                        .components(create_pagination_buttons(page as i32)),
                ),
            )
            .await?;
    }

    Ok(())
}

fn create_pagination_buttons(page: i32) -> Vec<serenity::CreateActionRow> {
    let previous_button = serenity::CreateButton::new("previous_page")
        .label("⬅️ Previous")
        .style(serenity::ButtonStyle::Primary)
        .disabled(page <= 1);

    let next_button = serenity::CreateButton::new("next_page")
        .label("➡️ Next")
        .style(serenity::ButtonStyle::Primary);

    let action_row = serenity::CreateActionRow::Buttons(vec![previous_button, next_button]);

    vec![action_row]
}
