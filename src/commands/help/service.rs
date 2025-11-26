use poise::serenity_prelude::Error;
use crate::commands::shared::{logs::{get_server_name, send_log}, models::LoggerDocument};

pub async fn log_command(
    name: &str,
    ctx: &poise::ApplicationContext<'_, crate::Data, Error>,
    success: bool,
) -> Result<(), Error> {
    send_log(
        LoggerDocument::new(
            &ctx.author().name,
            &name.to_string(),
            &get_server_name(ctx).await?,
            success,
            chrono::Utc::now().timestamp(),
        )
    ).await?;

    Ok(())
}
