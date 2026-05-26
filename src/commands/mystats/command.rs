use poise::serenity_prelude::Error;

use crate::commands::player_stats::command::show_player_stats;
use crate::commands::register::utils::get_user_link;
use crate::commands::shared::logs::{get_server_name, send_log};
use crate::commands::shared::models::LoggerDocument;
use crate::Data;

/// 📂 Displays your linked account stats (register first)
#[poise::command(slash_command)]
pub async fn mystats(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    ctx.defer().await?;

    let result: Result<(), Error> = async {
        let discord_user_id = ctx.author().id.get();

        let doc_opt = get_user_link(discord_user_id)
            .await
            .map_err(|e| Error::from(std::io::Error::other(format!("DB error: {e}"))))?;

        let Some(doc) = doc_opt else {
            ctx.say("❌ No linked account yet. Use `/register <account name>` first.")
                .await?;
            return Ok(());
        };

        let player_id = doc
            .get_i64("swrt_player_id")
            .map_err(|_| Error::from(std::io::Error::other("Invalid stored player_id in DB")))?;

        show_player_stats(&ctx, player_id, None).await
    }
    .await;

    send_log(LoggerDocument::new(
        &ctx.author().name,
        "mystats",
        &get_server_name(&ctx).await?,
        result.is_ok(),
        chrono::Utc::now().timestamp(),
    ))
    .await?;

    result
}
