use crate::commands::ranks::utils::get_rank_info;
use crate::commands::shared::logs::send_log;
use crate::{
    commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion},
    Data,
};
use poise::{
    serenity_prelude::{self as serenity, Error},
    CreateReply,
};
use std::vec;

/// 📂 Displays the current scores for ranks (C1 -> G3)
///
/// Usage: `/ranks`
#[poise::command(slash_command)]
pub async fn get_ranks(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    let scores = match get_rank_info().await {
        Ok(scores) => scores,
        Err(_) => {
            let error_message = "Unable to retrieve ELO information.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(
                &ctx,
                "Command: /ranks".to_string(),
                false,
                error_message.to_string(),
            )
            .await?;
            return Ok(());
        }
    };

    let thumbnail = "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true";

    // Constructing the description with group headers
    let groups = ["Conqueror", "Punisher", "Guardian"];
    let mut description = String::new();

    for (i, group) in groups.iter().enumerate() {
        // Adding the group header in bold
        description.push_str(&format!("{} :\n", group));
        // For each group, add three lines "Rank : Score"
        for j in 0..3 {
            let index = i * 3 + j;
            let (rank, score) = &scores[index];
            description.push_str(&format!("{} : **{}**\n", rank, score));
        }
        description.push('\n');
    }

    // Creating the embed using the description
    let embed = serenity::CreateEmbed::default()
        .title("Current ELOs")
        .color(serenity::Colour::from_rgb(0, 0, 255))
        .thumbnail(thumbnail)
        .description(description);

    let reply = CreateReply {
        embeds: vec![embed],
        ..Default::default()
    };

    ctx.send(reply).await?;

    send_log(
        &ctx,
        "Command: /ranks".to_string(),
        true,
        format!("Embed sent"),
    )
    .await?;

    Ok(())
}
