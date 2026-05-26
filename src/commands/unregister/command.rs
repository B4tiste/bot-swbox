
use poise::serenity_prelude::Error;
use poise::CreateReply;

use crate::commands::register::utils::delete_user_link;
use crate::commands::shared::logs::{get_server_name, send_log};
use crate::commands::shared::models::LoggerDocument;
use crate::Data;

/// 📂 Unlink your in-game account from your Discord profile
///
/// Usage: /unregister
#[poise::command(slash_command)]
pub async fn unregister(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    ctx.defer().await?;

    let result: Result<(), Error> = async {
        // Message placeholder (qu’on va éditer)
        let msg = ctx
            .send(CreateReply {
                content: Some("<a:loading:1358029412716515418> Unregistering...".to_string()),
                ..Default::default()
            })
            .await?;

        let discord_user_id = ctx.author().id.get();

        let deleted = delete_user_link(discord_user_id).await.map_err(|e| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("DB error: {e}"),
            ))
        })?;

        let content = if deleted == 0 {
            "❌ No linked account found for your Discord profile.".to_string()
        } else {
            "✅ Your linked account has been removed. You can link a new one using `/register <account name>`."
                .to_string()
        };

        msg.edit(
            poise::Context::Application(ctx),
            CreateReply {
                content: Some(content),
                components: Some(vec![]),
                embeds: vec![],
                ..Default::default()
            },
        )
        .await?;

        Ok(())
    }
    .await;

    send_log(LoggerDocument::new(
        &ctx.author().name,
        &"unregister".to_string(),
        &get_server_name(&ctx).await?,
        result.is_ok(),
        chrono::Utc::now().timestamp(),
    ))
    .await?;

    result
}
