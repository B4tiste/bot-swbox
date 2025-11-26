use poise::{serenity_prelude as serenity, serenity_prelude::Error};

pub(super) async fn build_help_embed(
    ctx: &poise::ApplicationContext<'_, crate::Data, Error>
) -> serenity::CreateEmbed {
    let thumbnail = "https://raw.githubusercontent.com/B4tiste/landing-page-bot/refs/heads/main/src/assets/images/old_bot_logo.gif";

    let mut embed = serenity::CreateEmbed::default()
        .title("Commands")
        .description("Created by **b4tiste** & **shaakz**")
        .color(serenity::Colour::from_rgb(0, 255, 255))
        .thumbnail(thumbnail);

    for command in &ctx.framework().options().commands {
        let desc = command
            .description
            .clone()
            .unwrap_or_else(|| "No description available".to_string());

        embed = embed.field(command.name.clone(), desc, true);
    }

    embed = embed.field("Source code", "[bot-swbox](https://github.com/B4tiste/bot-swbox)", false);
    embed = embed.field("My other project", "[BP Archive](https://bp-archive.netlify.app/)", false);
    embed = embed.field("Discord server", "https://discord.gg/AfANrTVaDJ", false);

    embed = embed.footer(serenity::CreateEmbedFooter::new(
        "Join our community on discord.gg/AfANrTVaDJ"
    ));

    embed
}
