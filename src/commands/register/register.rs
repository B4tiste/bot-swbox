use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::MONGO_URI;
use mongodb::{bson::doc, Client, Collection};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, USER_AGENT};

use crate::Data;
use poise::serenity_prelude::Error;

#[derive(Debug, poise::ChoiceParameter)]
pub enum Server {
    Europe,
    Global,
    Korea,
    Japan,
    China,
    Asia,
}


/// 📂 Register your Hive ID and server for the coupon tracker.
///
/// Usage: `/register <hive_id> <server>`
#[poise::command(slash_command)]
pub async fn register(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    hive_id: String,
    #[description = "Select the server of the SW account"] server: Server,
) -> Result<(), Error> {
    ctx.defer().await?;

    let server_string = match server {
        Server::Europe => "europe",
        Server::Global => "global",
        Server::Korea => "korea",
        Server::Japan => "japan",
        Server::China => "china",
        Server::Asia => "asia",
    };

    let coupon = "BLANK"; // Remplace par ton code coupon réel

    let client = reqwest::Client::new();
    let check_user_url = "https://event.withhive.com/ci/smon/evt_coupon/checkUser";

    let params = [
        ("country", "EN"),
        ("lang", "en"),
        ("server", server_string),
        ("hiveid", &hive_id.to_string()),
        ("coupon", coupon),
    ];

    // Création des headers
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/javascript, */*; q=0.01"),
    );
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-www-form-urlencoded; charset=UTF-8"),
    );
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36"));
    headers.insert(
        "Accept-Language",
        HeaderValue::from_static("fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7"),
    );
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://event.withhive.com"),
    );
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://event.withhive.com/ci/smon/evt_coupon"),
    );
    headers.insert(
        "X-Requested-With",
        HeaderValue::from_static("XMLHttpRequest"),
    );
    headers.insert(
        "sec-ch-ua",
        HeaderValue::from_static(
            "\"Not)A;Brand\";v=\"8\", \"Chromium\";v=\"138\", \"Google Chrome\";v=\"138\"",
        ),
    );
    headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
    headers.insert(
        "sec-ch-ua-platform",
        HeaderValue::from_static("\"Windows\""),
    );
    headers.insert("Cookie", HeaderValue::from_static("gdpr_section=true; _ga=GA1.1.1229236292.1730271769; _ga_FWV2C4HMXW=GS1.1.1730271768.1.1.1730271786.0.0.0; language=en"));

    let response = client
        .post(check_user_url)
        .headers(headers)
        .form(&params)
        .send()
        .await
        .map_err(|e| Error::Other(Box::leak(e.to_string().into_boxed_str())))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| Error::Other(Box::leak(e.to_string().into_boxed_str())))?;

    if json.get("retCode").and_then(|v| v.as_i64()) == Some(100) {
        // Ajout de l'utilisateur dans la base de données
        let mongo_uri = {
            let uri_guard = MONGO_URI.lock().unwrap();
            uri_guard.clone()
        };

        let collection = match get_mongo_collection(&mongo_uri).await {
            Ok(collection) => collection,
            Err(e) => {
                let error_message = format!("Failed to get MongoDB collection: {}", e);
                ctx.send(create_embed_error(&error_message)).await.ok();
                return Err(Error::Other(Box::leak(e.to_string().into_boxed_str())));
            }
        };

        // Get the discord usr id from the context
        let user_id = ctx.author().id.to_string();

        let user_doc = doc! {
            "user_id": user_id,
            "hive_id": hive_id.clone(),
            "server": server_string,
        };

        if let Ok(Some(_)) = collection.find_one(user_doc.clone()).await {
            let error_message = format!(
                "You are already registered with Hive ID: **{}** on server: **{}**.",
                hive_id, server_string
            );
            ctx.send(create_embed_error(&error_message)).await.ok();
            return Err(Error::Other(Box::leak(error_message.into_boxed_str())));
        }

        if let Err(e) = collection.insert_one(user_doc).await {
            let error_message = format!("Failed to insert user into MongoDB: {}", e);
            ctx.send(create_embed_error(&error_message)).await.ok();
            return Err(Error::Other(Box::leak(e.to_string().into_boxed_str())));
        }

        let success_message = format!(
            "You have successfully registered with Hive ID: **{}** on server: **{}**.",
            hive_id, server_string
        );
        ctx.say(success_message).await.ok();
    } else {
        let error_message = "Failed to register. Please check your Hive ID and server selection.";
        let reply = ctx.send(create_embed_error(&error_message)).await?;
        schedule_message_deletion(reply, ctx).await?;
        return Err(Error::Other(Box::leak(
            error_message.to_string().into_boxed_str(),
        )));
    }

    Ok(())
}

async fn get_mongo_collection(
    mongo_uri: &str,
) -> Result<Collection<mongodb::bson::Document>, mongodb::error::Error> {
    let client = Client::with_uri_str(mongo_uri).await?;

    let db = client.database("bot-swbox-db");
    Ok(db.collection("registered_users"))
}
