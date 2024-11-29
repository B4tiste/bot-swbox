use poise::{serenity_prelude::{self as serenity, Error}, Modal, CreateReply};

use crate::commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion};
use crate::commands::suggestion::modal::SuggestionModal;

/// 📂 Permet d'envoyer une suggestion de fonctionnalité ou de déclarer un BUG
///
/// Allow users to send a suggestion or report a bug.
///
/// Usage: `/send_suggestion`
#[poise::command(slash_command)]
pub async fn send_suggestion(ctx: poise::ApplicationContext<'_, (), Error>) -> Result<(), Error> {
    let modal_data: SuggestionModal = match SuggestionModal::execute(ctx).await {
        Ok(Some(data)) => data,
        Ok(None) => return Ok(()),
        Err(_) => {
            let error_message = "Erreur lors de l'exécution du modal.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let suggestion_channel_id = serenity::ChannelId::new(1311965614658687018);
    let user_name = &ctx.author().name;

    let mut embed = serenity::CreateEmbed::default()
        .title("Nouvelle suggestion")
        .color(serenity::Colour::from_rgb(70, 200, 120))
        .field("Utilisateur", user_name, false)
        .field("Suggestion", modal_data.description, false);

    if let Some(image) = modal_data.image {
        embed = embed.image(image);
    }

    // Send the suggestion to the suggestion channel
    let builder = serenity::CreateMessage::new().embed(embed);

    if let Err(_) = suggestion_channel_id.send_message(&ctx.serenity_context().http, builder).await {
        let error_message = format!("Erreur lors de l'envoi de la suggestion.");
        let reply = ctx.send(create_embed_error(&error_message)).await?;
        schedule_message_deletion(reply, ctx).await?;
        return Ok(());
    }

    // Reply to the user
    let reply_embed = serenity::CreateEmbed::default()
        .title("Suggestion envoyée")
        .description("Votre suggestion a bien été envoyée. Merci !")
        .color(serenity::Colour::from_rgb(70, 200, 120));

    let reply = CreateReply{
        embeds: vec![reply_embed],
        ..Default::default()
    };

    ctx.send(reply).await?;

    Ok(())
}