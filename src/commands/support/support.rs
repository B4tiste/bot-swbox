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
    let embed = serenity::CreateEmbed::default()
        .title("Support SWbox")
        .color(serenity::Colour::from_rgb(0, 255, 255))
        .description(
            "SWbox is a free, community-driven Discord bot.\n\
            Due to the removal of the free hosting tier, the project now has a fixed monthly cost.\n\
            If you enjoy using SWbox and want to help keep it online, consider supporting the project on **[Ko-Fi](https://ko-fi.com/swbox)** 💙",
        )
        .field(
            "🥉 Conqueror — 1€ / month",
            "One coffee a month to keep SWbox online.\n",
            false,
        )
        .field(
            "🥈 Punisher — 3€ / month",
            "Extra support to help SWbox grow.",
            false,
        )
        .field(
            "🥇 Guardian — 5€ / month",
            "Strong support to secure SWbox’s future.",
            false,
        )
        .field(
            "🎨 Custom Commission — Custom Player Alias (3€)",
            "Add a custom player alias for yourself or someone else.\n\
            The alias can be used in commands and will be displayed on the player profile.",
            false,
        )
        .field(
            "💬 Community Discord",
            "Join our community on [Discord](https://discord.gg/AfANrTVaDJ) to share feedback, get support, and connect with others!",
            false,
        )
        .thumbnail("https://bot-swbox.netlify.app/assets/images/old_bot_logo.gif")
        .footer(CreateEmbedFooter::new("Thank you for supporting SWbox ❤️"));

    let reply = CreateReply {
        embeds: vec![embed],
        ..Default::default()
    };

    let _ = ctx.send(reply).await;
    Ok(())
}
