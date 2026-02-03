use poise::serenity_prelude::Error;

use crate::commands::player_stats::get_player_stats::{get_token, show_player_stats};
use crate::commands::register::utils::get_user_link;
use crate::Data;

/// 📂 Displays your linked account stats (register first)
#[poise::command(slash_command)]
pub async fn mystats(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    ctx.defer().await?;

    let discord_user_id = ctx.author().id.get();

    let doc_opt = get_user_link(discord_user_id).await.map_err(|e| {
        Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("DB error: {e}"),
        ))
    })?;

    let Some(doc) = doc_opt else {
        ctx.say("❌ No linked account yet. Use `/register <account name>` first.")
            .await?;
        return Ok(());
    };

    let swrt_player_id = doc.get_i64("swrt_player_id").map_err(|_| {
        Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Invalid stored swrt_player_id in DB",
        ))
    })?;

    let token = get_token()?;
    show_player_stats(&ctx, &token, &swrt_player_id, None).await
}
