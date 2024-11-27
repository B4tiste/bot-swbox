use anyhow::Result;

use super::models::{MonsterGeneralInfoData, MonsterRtaInfoData, SlugData};

pub async fn get_monster_slug(mob_name: String) -> Result<SlugData, String> {
    let slug_url = format!("https://api.swarena.gg/monster/search/{}", mob_name);
    let response = reqwest::get(slug_url).await.map_err(|_| "Failed to send request".to_string())?;

    if response.status().is_success() {
        let api_response: serde_json::Value = response.json().await.map_err(|_| "Failed to parse JSON".to_string())?;

        if let Some(array) = api_response["data"].as_array() {
            // Étape 1: Chercher un élément correspondant exactement à `mob_name`
            if let Some(exact_match) = array.iter().find(|&element| {
                element["name"].as_str().unwrap_or_default().eq_ignore_ascii_case(&mob_name)
            }) {
                return Ok(SlugData {
                    name: exact_match["name"].as_str().unwrap_or_default().to_string(),
                    slug: exact_match["slug"].as_str().unwrap_or_default().to_string(),
                });
            }

            // Étape 2: Chercher un élément contenant "2A" ou "2a" dans `name`
            if let Some(matching_element) = array.iter().find(|&element| {
                element["name"].as_str().unwrap_or_default().to_lowercase().contains("2a")
            }) {
                return Ok(SlugData {
                    name: matching_element["name"].as_str().unwrap_or_default().to_string(),
                    slug: matching_element["slug"].as_str().unwrap_or_default().to_string(),
                });
            }

            // Étape 3: Prendre le premier élément
            if let Some(first_element) = array.get(0) {
                return Ok(SlugData {
                    name: first_element["name"].as_str().unwrap_or_default().to_string(),
                    slug: first_element["slug"].as_str().unwrap_or_default().to_string(),
                });
            }
        }
    }
    Err("Monster not found".to_string())
}


pub async fn get_monster_general_info(mob_formatted: String) -> Result<MonsterGeneralInfoData, String> {
    let monster_id_url = format!("https://api.swarena.gg/monster/{}/details", mob_formatted);
    let response = reqwest::get(monster_id_url).await.map_err(|_| "Failed to send request".to_string())?;

    if response.status().is_success() {
        let api_response: serde_json::Value = response.json().await.map_err(|_| "Failed to parse JSON".to_string())?;
        if let Some(id) = api_response["data"]["id"].as_i64() {
            return Ok(MonsterGeneralInfoData {
                id: id as i32,
                image_filename: api_response["data"]["image_filename"].as_str().unwrap_or_default().to_string(),
            });
        }
    }

    Err("Monster not found".to_string())
}

pub async fn get_monster_rta_info(mob_id: String, season: i64, is_g3: bool) -> Result<MonsterRtaInfoData, String> {
    let monster_rta_info_url_g3 = format!("https://api.swarena.gg/monster/{}/summary?season={}&isG3={}", mob_id, season, is_g3);
    let response = reqwest::get(monster_rta_info_url_g3).await.map_err(|_| "Failed to send request".to_string())?;

    if response.status().is_success() {
        let api_response: serde_json::Value = response.json().await.map_err(|_| "Failed to parse JSON".to_string())?;
        if !api_response["data"].is_null() {
            return Ok(MonsterRtaInfoData {
                played: api_response["data"]["played"].as_i64().unwrap_or(0) as i32,
                winner: api_response["data"]["winner"].as_i64().unwrap_or(0) as i32,
                banned: api_response["data"]["banned"].as_i64().unwrap_or(0) as i32,
                leader: api_response["data"]["leader"].as_i64().unwrap_or(0) as i32,
                play_rate: api_response["data"]["play_rate"].as_f64().unwrap_or(0.0) as f32,
                win_rate: api_response["data"]["win_rate"].as_f64().unwrap_or(0.0) as f32,
                ban_rate: api_response["data"]["ban_rate"].as_f64().unwrap_or(0.0) as f32,
                lead_rate: api_response["data"]["lead_rate"].as_f64().unwrap_or(0.0) as f32,
            });
        }
    }

    Err("Monster not found".to_string())
}

async fn get_latest_season() -> Result<i64, String> {
    let season_url = "https://api.swarena.gg/general/seasons";
    let response = reqwest::get(season_url).await.map_err(|_| "Failed to send request".to_string())?;

    if response.status().is_success() {
        let season_data: serde_json::Value = response.json().await.map_err(|_| "Failed to parse JSON".to_string())?;
        if let Some(season) = season_data["data"].as_array().and_then(|arr| arr.last()).and_then(|s| s["season"].as_i64()) {
            return Ok(season);
        }
    }

    Err("Failed to get latest season".to_string())
}

async fn verify_season(season: i64) -> Result<i64, String> {
    let season_url = "https://api.swarena.gg/general/seasons";
    let response = reqwest::get(season_url).await.map_err(|_| "Failed to send request".to_string())?;

    if response.status().is_success() {
        let season_data: serde_json::Value = response.json().await.map_err(|_| "Failed to parse JSON".to_string())?;
        if let Some(_) = season_data["data"].as_array().and_then(|arr| arr.iter().find(|s| s["season"].as_i64() == Some(season))) {
            return Ok(season);
        } else {
            return Err("Pas de données trouvées pour cette saison".to_string());
        }
    }
    Err("Nous n'avons pas pu verifier si cette saison existait.".to_string())
}
pub async fn get_season(season: Option<String>) -> Result<i64, String> {
    if let Some(season) = season {
        if let Ok(season) = season.parse::<i64>() {
            match verify_season(season).await {
                Ok(valid_season) => return Ok(valid_season),
                Err(e) => return Err(e),
            }
        }
        else {
            return Err("La saison doit être un nombre".to_string());
        }
    }
    get_latest_season().await
}