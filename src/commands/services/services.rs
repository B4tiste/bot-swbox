use crate::{
    commands::shared::{
        logs::{get_server_name, send_log},
        models::LoggerDocument,
    },
    Data,
};
use poise::{
    serenity_prelude::{self as serenity, Error},
    CreateReply,
};
use serenity::builder::CreateEmbedFooter;

/// 📂 Displays information from the MyShop services.
///
/// Usage : `/services`
#[poise::command(slash_command)]
pub async fn services(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    // Build the embed from your JSON data
    let embed = serenity::CreateEmbed::default()
        .title("MyShop Services")
        .url("https://discord.gg/myshop")
        .description(
            "Welcome to MyShop, your all-in-one marketplace for premium and secure services.\n\n\
             Discord Link : https://discord.gg/myshop",
        )
        // optional: pick a color you like
        .color(serenity::Colour::from_rgb(66, 30, 103))
        // author block
        .author(
            serenity::CreateEmbedAuthor::new("Join the SWbox Discord server by clicking here !")
                .url("https://discord.gg/AfANrTVaDJ")
                .icon_url("https://bot-swbox.netlify.app/assets/images/old_bot_logo.gif"),
        )
        // thumbnail
        .thumbnail("https://raw.githubusercontent.com/B4tiste/bot-swbox/refs/heads/B4tiste/myshop/Images/myshop_thumbnail.webp")
        // fields
        .field(
            "🎮 In-Game Services",
            "- Boosting\n- Coaching\n- FRR\n- Island decoration\n- And more !",
            false,
        )
        .field(
            "🛒 Marketplace – EU & Global Accounts",
            "- All types of accounts\n- Wide price range (budget → premium)\n- Starter accounts available",
            false,
        )
        .field(
            "🛍️ Cheap Packs",
            "- Much cheaper than competitors\n- Up to 33% discount ! Best Value on the Market !",
            false,
        )
        .field(
            "🔒 Middleman Service",
            "- Safe & Secure trading procedure\n- Trutsed process for buyers dans sellers",
            false,
        )
        .field(
            "💼 Brokering Service",
            "- We help you sell your account\n- Price advice and buyer search",
            false,
        )
        .field(
            "Need more information or support ?",
            "Feel free to contact :\n- `matmapoire` - MyShop owner\n- `b4tiste` - SWbox developper",
            true,
        )
        .field(
            "Join Us ! ",
            "https://discord.gg/myshop",
            false,
        )
        // image
        .image("https://raw.githubusercontent.com/B4tiste/bot-swbox/refs/heads/B4tiste/myshop/Images/myshop.png")
        // footer
        .footer(CreateEmbedFooter::new(
            "Join MyShop today and take your Summoners War experience to the next level!",
        ));

    // Send reply
    let reply = CreateReply {
        embeds: vec![embed],
        ..Default::default()
    };

    let send_result = ctx.send(reply).await;

    // Logging
    match send_result {
        Ok(_) => {
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"services".to_string(),
                &get_server_name(&ctx).await?,
                true,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
        }
        Err(_err) => {
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"services".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
        }
    }

    Ok(())
}
