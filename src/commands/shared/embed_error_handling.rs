use poise::{
    serenity_prelude::{CreateEmbed, CreateEmbedFooter, Error},
    CreateReply, ReplyHandle,
};
use tokio::time::{sleep, Duration};

use crate::Data;

pub fn create_embed_error(error_message: &str) -> CreateReply {
    let embed: CreateEmbed = CreateEmbed::default()
        .title("Error")
        .description(error_message)
        .color(0xff0000)
        .footer(CreateEmbedFooter::new(
            "Join our community on discord.gg/AfANrTVaDJ to share feedback, get support, and connect with others!",
        ))
        .thumbnail("https://raw.githubusercontent.com/B4tiste/landing-page-bot/refs/heads/main/src/assets/images/old_bot_logo.gif");
    CreateReply {
        embeds: vec![embed],
        ..Default::default()
    }
}

pub async fn schedule_message_deletion(
    sent_message: ReplyHandle<'_>,
    ctx: poise::ApplicationContext<'_, Data, Error>,
) -> Result<(), Error> {
    sleep(Duration::from_secs(60)).await;
    if let Ok(sent_msg) = sent_message.message().await {
        sent_msg.delete(&ctx.serenity_context().http).await?;
    }
    Ok(())
}
