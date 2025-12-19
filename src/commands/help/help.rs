use crate::{Data, commands::shared::{logs::{get_server_name, send_log}, models::LoggerDocument}};
use poise::{
    serenity_prelude::{self as serenity, Error},
    CreateReply,
};
use serenity::builder::CreateEmbedFooter;

/// 📂 Displays the available commands and prints the list of servers to the console.
///
/// Usage: `/help`
#[poise::command(slash_command)]
pub async fn help(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    // Create the embed that lists the commands
    let mut embed = serenity::CreateEmbed::default()
        .title("Commands")
        .color(serenity::Colour::from_rgb(0, 255, 255))
        .description("Created by **b4tiste** & **shaakz**");
    // .thumbnail(thumbnail)

    // Add each command's name and description as an embed field
    for command in &ctx.framework().options().commands {
        let description = command
            .description
            .clone()
            .unwrap_or_else(|| "No description available".to_string());
        embed = embed.field(command.name.clone(), description, true);
    }

    // Additional fields for credits and source code links
    embed = embed.field(
        "Source code & Project Road Map",
        "[bot-swbox](https://github.com/B4tiste/bot-swbox)",
        false,
    );
    embed = embed.field(
        "My other project",
        "[BP Archive](https://bp-archive.netlify.app/)",
        false,
    );
    embed = embed.field(
        "Discord server",
        "https://discord.gg/AfANrTVaDJ",
        false,
    );
    embed = embed.field(
        "Tutorial video for setup and use",
        "https://www.youtube.com/watch?v=U6CxFH6WFKU",
        false,
    );
    embed = embed.field(
        "Data Sources",
        "[swranking](https://m.swranking.com/) & [swarena](https://swarena.gg/)",
        false,
    );

    embed = embed.footer(CreateEmbedFooter::new(
        "Join our community on discord.gg/AfANrTVaDJ to share feedback, get support, and connect with others!",
    ));

    // Send the embed reply
    let reply = CreateReply {
        embeds: vec![embed.clone()],
        ..Default::default()
    };

    let send_result = ctx.send(reply).await;

    match send_result {
        Ok(_) => {
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"help".to_string(),
                &get_server_name(&ctx).await?,
                true,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
        }
        Err(_err) => {
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"help".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
        }
    }

    Ok(())
}
