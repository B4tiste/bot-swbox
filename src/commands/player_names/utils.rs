use futures::future;
use poise::serenity_prelude::Error;
use poise::Modal;

use mongodb::{bson::doc, Client};
use crate::MONGO_URI;

use crate::{
    commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion},
    Data,
};

use super::models::PlayerSearchInput;

pub async fn handle_modal<M, F>(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    transform: F,
) -> Result<Option<PlayerSearchInput>, Error>
where
    M: Modal + Send,
    F: Fn(M) -> PlayerSearchInput + Send,
{
    let result = M::execute(ctx).await?;
    Ok(result.map(transform))
}

async fn get_player_id_by_name(name: String) -> Result<String, String> {
    let url = format!("https://api.swarena.gg/player/search/{}", name);
    let response = reqwest::get(url)
        .await
        .map_err(|_| "Failed to send request".to_string())?;

    if response.status().is_success() {
        let api_response: serde_json::Value = response
            .json()
            .await
            .map_err(|_| "Failed to parse JSON".to_string())?;
        if !api_response["data"].is_null() && !api_response["data"].as_array().unwrap().is_empty() {
            return Ok(api_response["data"][0]["id"].as_i64().unwrap().to_string());
        }
    }
    Err(format!("Player **{}** not found.", name))
}

pub async fn resolve_player_id(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    modal_result: Result<Option<PlayerSearchInput>, Error>,
) -> Result<Option<String>, Error> {
    match modal_result {
        Ok(Some(modal_data)) => {
            if let Some(id) = modal_data.id.clone() {
                if let Ok(_) = id.parse::<i64>() {
                    return Ok(Some(id));
                } else {
                    let error_message = format!("The ID **{}** is not a valid integer.", id);
                    let reply = ctx.send(create_embed_error(&error_message)).await?;
                    schedule_message_deletion(reply, ctx).await?;
                    return Ok(None);
                }
            } else if let Some(name) = modal_data.name {
                // ✅ BONUS: si l’utilisateur a mis un nombre dans "Name", on le considère comme un ID
                if name.parse::<i64>().is_ok() {
                    return Ok(Some(name));
                }

                // Sinon, on tente la recherche SWArena par nom comme avant
                match get_player_id_by_name(name).await {
                    Ok(id) => return Ok(Some(id)),
                    Err(err) => {
                        let error_message = format!("Error: {}", err);
                        let reply = ctx.send(create_embed_error(&error_message)).await?;
                        schedule_message_deletion(reply, ctx).await?;
                        return Ok(None);
                    }
                }
            }
        }

        Ok(None) => return Ok(None),

        Err(e) => return Err(e),
    }

    Ok(None)
}

async fn get_player_seasons_played(player_id: String) -> Result<Vec<i64>, String> {
    let url = format!("https://api.swarena.gg/player/{}/seasons", player_id);
    let response = reqwest::get(url)
        .await
        .map_err(|_| "Failed to send request".to_string())?;

    if response.status().is_success() {
        let api_response: serde_json::Value = response
            .json()
            .await
            .map_err(|_| "Failed to parse JSON".to_string())?;
        if !api_response["data"].is_null() && api_response["data"].is_array() {
            let seasons_played: Vec<i64> = api_response["data"]
                .as_array()
                .unwrap()
                .iter()
                .map(|season| season.as_i64().unwrap())
                .collect();
            return Ok(seasons_played);
        }
    }
    Err("No seasons played.".to_string())
}

async fn get_player_name(player_id: String, seasons_played: String) -> Result<String, String> {
    let url = format!(
        "https://api.swarena.gg/player/{}/summary?season={}",
        player_id, seasons_played
    );
    let response = reqwest::get(url)
        .await
        .map_err(|_| "Failed to send request".to_string())?;

    if response.status().is_success() {
        let api_response: serde_json::Value = response
            .json()
            .await
            .map_err(|_| "Failed to parse JSON".to_string())?;
        if !api_response["data"].is_null() {
            let wizard_name = api_response["data"]["wizard_name"]
                .as_str()
                .ok_or("Failed to get wizard_name".to_string())?;
            return Ok(wizard_name.to_string());
        }
    }
    Err("Failed to retrieve player name.".to_string())
}

pub async fn get_player_all_names(player_id: String) -> Result<Vec<String>, String> {
    let seasons_played = get_player_seasons_played(player_id.clone()).await?;

    let player_names_futures = seasons_played.into_iter().map(|season| {
        let player_id = player_id.clone();
        tokio::spawn(async move { get_player_name(player_id, season.to_string()).await })
    });

    let results = future::join_all(player_names_futures).await;

    let mut player_names = Vec::new();
    for result in results {
        if let Ok(Ok(name)) = result {
            if !player_names.contains(&name) {
                player_names.push(name);
            }
        }
    }

    Ok(player_names)
}

pub async fn get_swrt_id_from_db_by_player_id(player_id: i64) -> Result<i64, String> {
    let mongo_uri = {
        let guard = MONGO_URI.lock().map_err(|_| "Failed to lock MONGO_URI".to_string())?;
        guard.clone()
    };

    let client = Client::with_uri_str(&mongo_uri)
        .await
        .map_err(|e| format!("Mongo connection error: {e}"))?;

    let coll = client
        .database("bot-swbox-db")
        .collection::<mongodb::bson::Document>("players");

    let filter = doc! { "playerId": player_id };
    let doc = coll
        .find_one(filter)
        .await
        .map_err(|e| format!("Mongo query error: {e}"))?
        .ok_or_else(|| "Player not found in DB".to_string())?;

    let swrt_player_id = doc
        .get_i64("swrtPlayerId")
        .or_else(|_| doc.get_i32("swrtPlayerId").map(|v| v as i64))
        .map_err(|_| "Missing swrtPlayerId in DB document".to_string())?;

    Ok(swrt_player_id)
}

pub async fn get_current_detail_from_swrt(swrt_player_id: i64) -> Result<(String, Option<String>), String> {
    // Retourne (playerName, headImg)
    // On peut réutiliser la fonction existante get_user_detail(...) mais pour éviter
    // une dépendance circulaire, on fait un appel léger ici.
    let token = {
        use crate::API_TOKEN;
        let guard = API_TOKEN.lock().map_err(|_| "Failed to lock API_TOKEN".to_string())?;
        guard.clone().ok_or_else(|| "Missing API token".to_string())?
    };

    let url = format!("https://m.swranking.com/api/player/detail?swrtPlayerId={}", swrt_player_id);
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("Authentication", token)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("SWRanking request error: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("SWRanking status {}", res.status()));
    }

    let v: serde_json::Value = res.json().await.map_err(|_| "Failed to parse SWRanking JSON".to_string())?;

    let player = v["data"]["player"].clone();
    if player.is_null() {
        return Err("Missing 'data.player' in SWRanking response".to_string());
    }

    let name = player["playerName"]
        .as_str()
        .ok_or("Missing playerName")?
        .to_string();

    let head_img = player["headImg"].as_str().map(|s| s.to_string());
    Ok((name, head_img))
}