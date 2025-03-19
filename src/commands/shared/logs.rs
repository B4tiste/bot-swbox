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

    // Récupérer le nom du serveur
    let guild_name = if let Some(guild_id) = ctx.guild_id() {
        if let Some(guild) = guild_id.to_guild_cached(&ctx.serenity_context().cache) {
            guild.name.clone()
        } else {
            "Unknown Guild".to_string()
        }
    } else {
        "DM (Direct Message)".to_string()
    };

    let log_embed = CreateEmbed::default()
        .title("Interaction Log")
        .field("User", &ctx.author().name, true)
        .field("Command", &ctx.command().name, true)
        .field("Server", guild_name, true) // Ajouter le nom du serveur
        .field("User Input", user_input_str, false)
        .field("Response Successful", format!("{}", response_state), true)
        .field("Output Result", response_output_str, false)
        .color(if response_state { 0x00ff00 } else { 0xff0000 }) // Green for success, red for failure
        .timestamp(chrono::Utc::now());

    let channel_id = ChannelId::from(*LOG_CHANNEL_ID.lock().unwrap());
    let message = CreateMessage::default().content("").embed(log_embed);
    channel_id
        .send_message(&ctx.serenity_context().http, message)
        .await?;
    Ok(())
}
