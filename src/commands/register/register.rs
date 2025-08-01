use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::register::utils::{apply_missing_coupons_to_user};
use crate::MONGO_URI;
use mongodb::{bson::doc, Client, Collection};
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

/// Registers to the bot and applies all available coupons to the user.
///
/// Usage: `/register <hive_id> <server>`
#[poise::command(slash_command)]
pub async fn register(
    ctx: poise::ApplicationContext<'_, crate::Data, Error>,
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

    // Check Hive ID
    let coupon = "BLANK";
    let client = reqwest::Client::new();
    let check_user_url = "https://event.withhive.com/ci/smon/evt_coupon/checkUser";
    let params = [
        ("country", "EN"),
        ("lang", "en"),
        ("server", server_string),
        ("hiveid", &hive_id.to_string()),
        ("coupon", coupon),
    ];

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Accept",
        "application/json, text/javascript, */*; q=0.01"
            .parse()
            .unwrap(),
    );
    headers.insert(
        "Content-Type",
        "application/x-www-form-urlencoded; charset=UTF-8"
            .parse()
            .unwrap(),
    );
    headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36".parse().unwrap());
    headers.insert(
        "Accept-Language",
        "fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7".parse().unwrap(),
    );
    headers.insert("Origin", "https://event.withhive.com".parse().unwrap());
    headers.insert(
        "Referer",
        "https://event.withhive.com/ci/smon/evt_coupon"
            .parse()
            .unwrap(),
    );
    headers.insert("X-Requested-With", "XMLHttpRequest".parse().unwrap());
    headers.insert(
        "sec-ch-ua",
        r#""Not)A;Brand";v="8", "Chromium";v="138", "Google Chrome";v="138""#
            .parse()
            .unwrap(),
    );
    headers.insert("sec-ch-ua-mobile", "?0".parse().unwrap());
    headers.insert("sec-ch-ua-platform", r#""Windows""#.parse().unwrap());
    headers.insert("Cookie", "gdpr_section=true; _ga=GA1.1.1229236292.1730271769; _ga_FWV2C4HMXW=GS1.1.1730271768.1.1.1730271786.0.0.0; language=en".parse().unwrap());

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
        let mongo_uri = {
            let uri_guard = MONGO_URI.lock().unwrap();
            uri_guard.clone()
        };
        let collection = get_mongo_collection(&mongo_uri).await.map_err(|e| {
            let error_message = format!("Failed to get MongoDB collection: {}", e);
            Error::Other(Box::leak(error_message.into_boxed_str()))
        })?;

        let user_id = ctx.author().id.to_string();

        let user_doc = doc! {
            "user_id": user_id.clone(),
            "hive_id": hive_id.clone(),
            "server": server_string,
            "applied_coupons": [],
        };

        if let Ok(Some(_)) = collection
            .find_one(doc! { "hive_id": &hive_id })
            .await
        {
            let error_message = "User already registered with this Hive ID and server.";
            ctx.send(create_embed_error(&error_message)).await.ok();
            return Err(Error::Other(Box::leak(
                error_message.to_string().into_boxed_str(),
            )));
        }

        if let Err(e) = collection.insert_one(user_doc).await {
            let error_message = format!("Failed to insert user into MongoDB: {}", e);
            ctx.send(create_embed_error(&error_message)).await.ok();
            return Err(Error::Other(Box::leak(e.to_string().into_boxed_str())));
        }

        // Immediately apply all coupons to this new user (non-blocking)
        let mongo_uri = mongo_uri.clone();
        tokio::spawn(async move {
            if let Err(e) = apply_missing_coupons_to_user(&mongo_uri, &user_id).await {
                eprintln!("Coupon sync for new user failed: {e:?}");
            }
        });

        ctx.say("You have successfully registered. All available coupons are being applied!")
            .await
            .ok();
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
