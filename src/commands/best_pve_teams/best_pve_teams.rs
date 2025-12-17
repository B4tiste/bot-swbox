use crate::commands::player_stats::utils::get_mob_emoji_collection;
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::{get_server_name, send_log};
use crate::{Data, API_TOKEN, GUARDIAN_EMOJI_ID, PUNISHER_EMOJI_ID};
use serenity::{
    builder::EditInteractionResponse, CreateInteractionResponse, CreateInteractionResponseMessage,
    Error,
};

/// 📂 Displays the current best PvE teams to use
///
/// Usage: `/best_pve_teams`
#[poise::command(slash_command)]
pub async fn best_pve_teams(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {

    // Évite le timeout de 3 s
    ctx.defer().await?;

    let user_id = ctx.author().id;

    // 😃 Récupération de la collection d'emojis
    let collection = match get_mob_emoji_collection().await {
        Ok(c) => c,
        Err(_) => {
            let err_msg =
                "Impossible de récupérer les emojis des monstres (DB error). Réessaie plus tard.";
            let reply = ctx.send(create_embed_error(err_msg)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"get_meta".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            return Ok(());
        }
    };

    // TODO

    // 📝 Logging
    send_log(LoggerDocument::new(
        &ctx.author().name,
        &"get_meta".to_string(),
        &get_server_name(&ctx).await?,
        true,
        chrono::Utc::now().timestamp(),
    ))
    .await?;

    Ok(())
}