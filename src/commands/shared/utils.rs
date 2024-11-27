use crate::commands::shared::models::{MonsterGeneralInfoData, SlugData};

pub async fn get_monster_slug(mob_name: String) -> Result<SlugData, String> {
    let slug_url = format!("https://api.swarena.gg/monster/search/{}", mob_name);
    let response = reqwest::get(slug_url).await.map_err(|_| "Failed to send request".to_string())?;

    if response.status().is_success() {
        let api_response: serde_json::Value = response.json().await.map_err(|_| "Failed to parse JSON".to_string())?;

        if let Some(array) = api_response["data"].as_array() {
            // Chercher un élément contenant "2A" ou "2a" uniquement dans le champ `name`
            if let Some(matching_element) = array.iter().find(|&element| {
                element["name"].as_str().unwrap_or_default().to_lowercase().contains("2a")
            }) {
                return Ok(SlugData {
                    name: matching_element["name"].as_str().unwrap_or_default().to_string(),
                    slug: matching_element["slug"].as_str().unwrap_or_default().to_string(),
                });
            }

            // Sinon, prendre le premier élément
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