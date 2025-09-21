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
use serenity::builder::CreateEmbedFooter;
use std::vec;

/// 📂 Displays the current scores for ranks (C1 -> G3)
///
/// Usage: `/get_ranks`
#[poise::command(slash_command)]
pub async fn get_ranks(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    // Defer the response to avoid the 3 seconds timeout
    ctx.defer().await?;

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

    // cutoff image url : https://sw-tt.com/test.png
    let cutoff_image_url = "https://sw-tt.com/test.png";

    // Télécharger l'image
    let response = reqwest::get(cutoff_image_url)
        .await
        .map_err(|_| serenity::Error::Other("Failed to download cutoff image"))?;
    let image_bytes = response
        .bytes()
        .await
        .map_err(|_| serenity::Error::Other("Failed to read cutoff image bytes"))?;

    // Creating the embed using the description
    let embed = serenity::CreateEmbed::default()
        .title("Current rank thresholds")
        .color(serenity::Colour::from_rgb(0, 0, 255))
        .thumbnail(thumbnail)
        .description(description)
        .field(
            "Cutoffs prediction :",
            format!("From [SW-TT](https://sw-tt.com) (I'm not responsible for any inaccuracy from the predictions)"),
            false,
        )
        .image("attachment://cutoffs.png")
        .footer(CreateEmbedFooter::new(
            "Join our community on discord.gg/AfANrTVaDJ to share feedback, get support, and connect with others!",
        ));

    let attachements = serenity::CreateAttachment::bytes(image_bytes.to_vec(), "cutoffs.png");

    let reply = CreateReply {
        embeds: vec![embed],
        attachments: vec![attachements],
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
