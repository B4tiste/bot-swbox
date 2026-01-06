use mongodb::Client;
use poise::serenity_prelude::Error;

use crate::{commands::shared::models::LoggerDocument, Data, MONGO_URI};

pub async fn send_log(log: LoggerDocument) -> Result<(), Error> {
    let mongo_uri = {
        let uri_guard = MONGO_URI.lock().unwrap();
        uri_guard.clone()
    };
    let client = Client::with_uri_str(mongo_uri)
        .await
        .map_err(|e| Error::Other(Box::leak(Box::new(e.to_string()))))?;
    let database = client.database("bot-swbox-db");
    let collection = database.collection::<LoggerDocument>("logs");

    collection
        .insert_one(log)
        .await
        .map_err(|e| Error::Other(Box::leak(Box::new(e.to_string()))))?;
    Ok(())
}

pub async fn get_server_name(
    ctx: &poise::ApplicationContext<'_, Data, Error>,
) -> Result<String, Error> {
    let guild_name = if let Some(server_id) = ctx.guild_id() {
        if let Some(guild) = server_id.to_guild_cached(&ctx.serenity_context().cache) {
            guild.name.clone()
        } else {
            "Unknown Guild".to_string()
        }
    } else {
        "DM".to_string()
    };

    Ok(guild_name)
}
