use poise::{
    serenity_prelude::{self as serenity, Error},
    CreateReply,
};
use crate::commands::shared::logs::send_log;

/// 📂 Affiche les commandes disponibles
///
/// Displays the available commands
///
/// Usage: `/help`
#[poise::command(slash_command)]
pub async fn help(ctx: poise::ApplicationContext<'_, (), Error>) -> Result<(), Error> {
    let thumbnail = "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true";

    let mut embed = serenity::CreateEmbed::default()
        .title("Commandes")
        .color(serenity::Colour::from_rgb(0, 255, 255))
        .thumbnail(thumbnail);

    for command in &ctx.framework().options().commands {
        let description = command
            .description
            .clone()
            .unwrap_or("No description available".to_string());
        embed = embed.field(command.name.clone(), description, true);
    }

    embed = embed.field("Créé par", "<@!191619427584835585> & <@!366631137562329091>", true);

    embed = embed.field(
        "Code source & Road Map du projet",
        "[bot-swbox](https://github.com/B4tiste/bot-swbox)",
        true,
    );

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
                format!("Embed envoyé"),
            )
            .await?;
        }
        Err(err) => {
            send_log(
                &ctx,
                "Command: /help".to_string(),
                false,
                format!("Erreur lors de l'envoi : {:?}", err),
            )
            .await?;
        }
    }

    Ok(())
}
