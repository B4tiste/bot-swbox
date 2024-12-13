use poise::serenity_prelude::{ChannelId, CreateEmbed, CreateMessage, Error};
use std::fmt::Debug;

use crate::{Data, LOG_CHANNEL_ID};

pub async fn send_log<T: Debug, G: Debug>(
    ctx: &poise::ApplicationContext<'_, Data, Error>,
    user_input: T,
    response_state: bool,
    response_output: G,
) -> Result<(), Error> {
    let user_input_str = format!("{:?}", user_input);
    let response_output_str = format!("{:?}", response_output);

    let log_embed = CreateEmbed::default()
        .title("Log d'interaction")
        .field("Utilisateur", &ctx.author().name, true)
        .field("Commande", &ctx.command().name, true)
        .field("Input utilisateur", user_input_str, false)
        .field("Réponse réussie", format!("{}", response_state), true)
        .field("Résultat de sortie", response_output_str, false)
        .color(if response_state { 0x00ff00 } else { 0xff0000 }) // Vert pour succès, rouge pour échec
        .timestamp(chrono::Utc::now());

        let channel_id = ChannelId::from(*LOG_CHANNEL_ID.lock().unwrap());
        let message = CreateMessage::default()
            .content("")
            .embed(log_embed);
        channel_id.send_message(&ctx.serenity_context().http, message).await?;
    Ok(())
}
