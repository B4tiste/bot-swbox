use crate::{commands::shared::logs::send_log, Data};
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
    let thumbnail = "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true";

    // Print the list of guilds (servers) the bot is in to the console
    // Access the cache from the serenity client
    let cache = &ctx.serenity_context().cache;
    let guild_ids = cache.guilds(); // récupère tous les IDs de serveurs
    println!(
        "Bot is in the following servers ({} total):",
        guild_ids.len()
    );

    // Iterate over all guild IDs from the cache
    for guild_id in guild_ids.iter() {
        // Retrieve the full guild from the cache using the guild ID.
        if let Some(guild) = cache.guild(guild_id) {
            println!("{} (ID: {})", guild.name, guild_id);
        }
    }

    // Create the embed that lists the commands
    let mut embed = serenity::CreateEmbed::default()
        .title("Commands")
        .color(serenity::Colour::from_rgb(0, 255, 255))
        .thumbnail(thumbnail)
        .description("Created by **b4tiste** & **shaakz**");

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

    embed = embed.footer(CreateEmbedFooter::new(
        "Please use /send_suggestion to report any issue.",
    ));

    // Send the embed reply
    let reply = CreateReply {
        embeds: vec![embed.clone()],
        ..Default::default()
    };

    let send_result = ctx.send(reply).await;

    match send_result {
        Ok(_) => {
            send_log(
                &ctx,
                "Command: /help".to_string(),
                true,
                "Embed sent".to_string(),
            )
            .await?;
        }
        Err(err) => {
            send_log(
                &ctx,
                "Command: /help".to_string(),
                false,
                format!("Error sending embed: {:?}", err),
            )
            .await?;
        }
    }

    Ok(())
}
