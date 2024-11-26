use poise::serenity_prelude::{CreateEmbed, CreateEmbedFooter};
use tokio::time::{sleep, Duration};
use poise::ReplyHandle;

use poise::CreateReply;
use crate::commands::ranks::lib::{Context, Error};


pub fn create_embed_error(error_message: &str) -> CreateReply {
    let embed: CreateEmbed = CreateEmbed::default()
        .title("Erreur")
        .description(error_message)
        .color(0xff0000)
        .footer(CreateEmbedFooter::new("Ce message sera supprimé dans 60 secondes."))
        .thumbnail("https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true");
    CreateReply {
        embeds: vec![embed],
        ..Default::default()
    }
}

pub async fn schedule_message_deletion(
    sent_message: ReplyHandle<'_>,
    ctx: Context<'_>,
) -> Result<(), Error> {
    sleep(Duration::from_secs(60)).await;
    if let Ok(sent_msg) = sent_message.message().await {
        sent_msg.delete(&ctx.serenity_context().http).await?;
    }
    Ok(())
}