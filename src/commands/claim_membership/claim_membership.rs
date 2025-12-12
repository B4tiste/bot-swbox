use poise::{serenity_prelude::Error, Modal};
use crate::commands::claim_membership::modal::ClaimMembershipModal;

use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::{get_server_name, send_log};
use crate::commands::shared::models::LoggerDocument;
use crate::{Data, API_TOKEN};

/// 📂 Claims membership using the email used to buy the membership on Ko-Fi
///
/// Usage: `/claim_membership`
#[poise::command(slash_command)]
pub async fn claim_membership(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    let modal_data: ClaimMembershipModal = match ClaimMembershipModal::execute(ctx).await {
        Ok(Some(data)) => data,
        Ok(None) => return Ok(()),
        Err(err) => {
            let error_message = format!("Error executing the modal: {:?}", err);
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            // send_log(&ctx, "No data received".to_string(), false, error_message).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let email = modal_data.email.as_str();

    // send the email back to the user
    ctx.say(format!(
        "You have entered the email: **{}**",
        email
    ))
    .await?;

    Ok(())
}