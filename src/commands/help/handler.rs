use crate::Data;
use poise::{serenity_prelude::Error, CreateReply};

use super::service::log_command;
use super::builder::build_help_embed;

pub(super) async fn help_handler(
    ctx: poise::ApplicationContext<'_, Data, Error>
) -> Result<(), Error> {

    let embed = build_help_embed(&ctx).await;

    let reply = CreateReply {
        embeds: vec![embed.clone()],
        ..Default::default()
    };

    let result = ctx.send(reply).await;

    log_command("help", &ctx, result.is_ok()).await?;

    Ok(())
}
