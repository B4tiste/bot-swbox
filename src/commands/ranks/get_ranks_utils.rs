use std::vec;

use poise::{serenity_prelude::{self as serenity}, CreateReply};
use crate::commands::ranks::lib::{info_rank_sw, Context, Error};

/// Displays the current scores for ranks.
///
/// Usage: `/ranks`
#[poise::command(slash_command, prefix_command)]
pub async fn get_ranks(ctx: Context<'_>) -> Result<(), Error> {
    // Retrieve the scores
    let scores = info_rank_sw().await?;

    // Create the embed
    let thumbnail = "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true";
    let mut embed = serenity::CreateEmbed::default().title("ELOs actuels").color(serenity::Colour::from_rgb(0, 0, 255)).thumbnail(thumbnail);

    // Add fields to the embed
    for (rank, score) in &scores {
        embed = embed.field(rank, score.to_string(), true);
    }

    let reply = CreateReply{
        embeds: vec![embed],
        ..Default::default()
    };

    // Send the embed
    ctx.send(reply).await?;

    Ok(())
}
