use poise::serenity_prelude::Error;
use poise::Modal;
use futures::future;

use crate::{commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
}, Data};

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
    Err(format!("Joueur **{}** introuvable.", name))
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
                    let error_message = format!("L'ID **{}** n'est pas un entier valide.", id);
                    let reply = ctx.send(create_embed_error(&error_message)).await?;
                    schedule_message_deletion(reply, ctx).await?;
                    return Ok(None);
                }
            } else if let Some(name) = modal_data.name {
                match get_player_id_by_name(name).await {
                    Ok(id) => return Ok(Some(id)),
                    Err(err) => {
                        let error_message = format!("Erreur : {}", err);
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
    Err("Aucune saison jouée.".to_string())
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
        tokio::spawn(async move {
            get_player_name(player_id, season.to_string()).await
        })
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
