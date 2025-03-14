use crate::{commands::shared::logs::send_log, Data};
use poise::{
    serenity_prelude::{self as serenity, Error},
    CreateReply,
};

/// 📂 Displays the available commands
///
/// Usage: `/help`
#[poise::command(slash_command)]
pub async fn help(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    let thumbnail = "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true";

    let mut embed = serenity::CreateEmbed::default()
        .title("Commands")
        .color(serenity::Colour::from_rgb(0, 255, 255))
        .thumbnail(thumbnail);

    for command in &ctx.framework().options().commands {
        let description = command
            .description
            .clone()
            .unwrap_or("No description available".to_string());
        embed = embed.field(command.name.clone(), description, true);
    }

    embed = embed.field(
        "Created by",
        "<@!191619427584835585> & <@!366631137562329091>",
        true,
    );

    embed = embed.field(
        "Source code & Project Road Map",
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
                format!("Embed sent"),
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
