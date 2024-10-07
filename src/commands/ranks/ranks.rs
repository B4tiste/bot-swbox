use std::vec;

use poise::{serenity_prelude as serenity, CreateReply};
use crate::commands::ranks::utils::{info_rank_sw, Context, Error};

/// Displays the current scores for ranks.
///
/// Usage: `/ranks`
#[poise::command(slash_command, prefix_command)]
pub async fn ranks(ctx: Context<'_>) -> Result<(), Error> {
    let scores = info_rank_sw().await?;

    let url_reddit_ranks = "https://www.reddit.com/r/summonerswar/wiki/gettingstarted/wings";

    // Create the embed
    let mut embed = serenity::CreateEmbed::default()
        .title("Current Scores for Ranks")
        .color(serenity::Colour::from_rgb(0, 150, 255));

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
