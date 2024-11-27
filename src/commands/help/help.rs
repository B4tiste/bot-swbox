use poise::{serenity_prelude::{self as serenity}, CreateReply};

use crate::commands::help::lib::{Context, Error};

/// 📂 Affiche les commandes disponibles
///
/// Displays the available commands
///
/// Usage: `/help`
#[poise::command(slash_command, prefix_command)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let thumbnail = "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true";

    let mut embed = serenity::CreateEmbed::default()
        .title("Commandes")
        .color(serenity::Colour::from_rgb(0, 255, 255))
        .thumbnail(thumbnail);

    // Add a field for each command
    for command in &ctx.framework().options().commands {
        let description = command.description.clone().unwrap_or("No description available".to_string());
        embed = embed.field(command.name.clone(), description, true);
    }

    // Add a field that show the @ of the creators
    embed = embed.field("Créé par", "<@!191619427584835585> & <@!366631137562329091>", true);

    // Add a field for the github link
    embed = embed.field("Github (Open Source)", "[bot-swbox](https://github.com/B4tiste/bot-swbox)", true);

    let reply = CreateReply {
        embeds: vec![embed],
        ..Default::default()
    };

    // Attempt to send the reply, but handle errors gracefully
    if ctx.send(reply).await.is_err() {
        eprintln!("Failed to send help message");
    }

    Ok(()) // Return success
}
