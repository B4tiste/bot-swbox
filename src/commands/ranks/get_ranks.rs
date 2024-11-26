use std::vec;

use poise::{serenity_prelude::{self as serenity}, CreateReply};
use crate::commands::{embed_error_handling::{create_embed_error, schedule_message_deletion}, ranks::lib::{info_rank_sw, Context, Error}};

/// Displays the current scores for ranks.
///
/// Usage: `/ranks`
#[poise::command(slash_command, prefix_command)]
pub async fn get_ranks(ctx: Context<'_>) -> Result<(), Error> {
    // Retrieve the scores
    let scores = match info_rank_sw().await {
        Ok(scores) => scores,
        Err(_) => {
            let error_message = "Impossible de récupérer les informations des ELOs.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let thumbnail = "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true";
    let mut embed = serenity::CreateEmbed::default().title("ELOs actuels").color(serenity::Colour::from_rgb(0, 0, 255)).thumbnail(thumbnail);

    for (rank, score) in &scores {
        embed = embed.field(rank, score.to_string(), true);
    }

    let reply = CreateReply{
        embeds: vec![embed],
        ..Default::default()
    };

    ctx.send(reply).await?;

    Ok(())
}
