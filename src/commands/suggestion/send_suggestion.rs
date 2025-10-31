use poise::{
    serenity_prelude::{self as serenity, Error},
    CreateReply, Modal,
};

use crate::commands::shared::{
    logs::{get_server_name, send_log},
    models::LoggerDocument,
};
use crate::commands::suggestion::modal::SuggestionModal;
use crate::{
    commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion},
    Data,
};

/// 📂 Allows users to send a feature suggestion or report a BUG
///
/// Usage: `/send_suggestion`
#[poise::command(slash_command)]
pub async fn send_suggestion(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    let modal_data: SuggestionModal = match SuggestionModal::execute(ctx).await {
        Ok(Some(data)) => data,
        Ok(None) => return Ok(()),
        Err(_) => {
            let error_message = "Error executing the modal.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"send_suggestion".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let suggestion_channel_id = serenity::ChannelId::new(1311965614658687018);
    let user_name = &ctx.author().name;

    let mut embed = serenity::CreateEmbed::default()
        .title("New Suggestion")
        .description(modal_data.name.clone())
        .color(serenity::Colour::from_rgb(70, 200, 120))
        .field("User", user_name, false)
        .field("Suggestion", modal_data.description.clone(), false);

    if let Some(image) = modal_data.image.clone() {
        embed = embed.image(image);
    }

    let builder = serenity::CreateMessage::new().embed(embed.clone());
    let suggestion_result = suggestion_channel_id
        .send_message(&ctx.serenity_context().http, builder)
        .await;

    match suggestion_result {
        Ok(_) => {
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"send_suggestion".to_string(),
                &get_server_name(&ctx).await?,
                true,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
        }
        Err(_) => {
            let error_message = "Error sending the suggestion.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"send_suggestion".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    }

    let reply_embed = serenity::CreateEmbed::default()
        .title("Suggestion Sent")
        .description("Your suggestion has been sent successfully. Thank you!")
        .color(serenity::Colour::from_rgb(70, 200, 120));

    let reply = CreateReply {
        embeds: vec![reply_embed.clone()],
        ..Default::default()
    };

    let reply_handle = ctx.send(reply).await?;
    schedule_message_deletion(reply_handle, ctx).await?;

    send_log(LoggerDocument::new(
        &ctx.author().name,
        &"send_suggestion".to_string(),
        &get_server_name(&ctx).await?,
        true,
        chrono::Utc::now().timestamp(),
    ))
    .await?;

    Ok(())
}
