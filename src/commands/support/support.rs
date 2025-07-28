use crate::Data;
use poise::{
    serenity_prelude::{self as serenity, Error},
    CreateReply,
};
use serenity::builder::CreateEmbedFooter;

/// 📂 Link to the Ko-Fi to support the project.
///
/// Usage: `/support`
#[poise::command(slash_command)]
pub async fn support(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    // Create the embed that provides support information
    let embed = serenity::CreateEmbed::default()
        .title("Support the Project")
        .color(serenity::Colour::from_rgb(0, 255, 255))
        .description("If you like this bot and want to support the project, consider donating on [Ko-Fi](https://ko-fi.com/swbox).")
        .field(
            "Commission : Custom Player Alias (3€)",
            "You can add a custom player alias for yourself or someone else. This alias can be used to search for that player using the command, and will also be displayed on the player profile.",
            false,
        )
        .footer(CreateEmbedFooter::new("Thank you for your support!"));

    // Send the embed as a reply to the command
    let reply = CreateReply {
        embeds: vec![embed.clone()],
        ..Default::default()
    };

    let _ = ctx.send(reply).await;

    Ok(())
}