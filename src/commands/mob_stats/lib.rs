use anyhow::{Result, Context};
use log::info;

use super::get_mob_stats_utils::{MonsterGeneralInfoData, SlugData};

pub async fn get_monster_slug(mob_name: String) -> Result<SlugData, anyhow::Error> {
    let slug_url = format!("https://api.swarena.gg/monster/search/{}", mob_name);
    let response = reqwest::get(slug_url).await.context("Failed to send request")?;

    if response.status().is_success() {
        let api_response: serde_json::Value = response.json().await.context("Failed to parse JSON")?;


        if !api_response["data"].as_array().unwrap().is_empty() {
            let first_element = &api_response["data"][0];
            let slug_data = SlugData{
                name: first_element["name"].as_str().unwrap().to_string(),
                slug: first_element["slug"].as_str().unwrap().to_string(),
            };
            return Ok(slug_data);
        }
    }

    Err(anyhow::anyhow!("Monster not found"))
}

pub async fn get_monster_id(mob_formatted: String) -> Result<MonsterGeneralInfoData, anyhow::Error> {
    let monster_id_url = format!("https://api.swarena.gg/monster/{}/details", mob_formatted);
    let response = reqwest::get(monster_id_url).await.context("Failed to send request")?;

    if response.status().is_success() {
        let api_response: serde_json::Value = response.json().await.context("Failed to parse JSON")?;

        if api_response["data"]["id"].is_i64() {
            let mob_id = api_response["data"]["id"].as_i64().unwrap() as i32;
            let mob_image_filename = api_response["data"]["image_filename"].as_str().unwrap().to_string();
            
            let mob_general_info = MonsterGeneralInfoData{
                id: mob_id,
                image_filename: mob_image_filename,
            };
            return Ok(mob_general_info);
        }
    }

    Err(anyhow::anyhow!("Monster not found"))
}

pub async fn get_latest_season() -> Result<i64, anyhow::Error> {
    let season_url = "https://api.swarena.gg/general/seasons";
    let response = reqwest::get(season_url).await.context("Failed to send request")?;

    if response.status().is_success() {
        let season_data: serde_json::Value = response.json().await.context("Failed to parse JSON")?;

        if season_data["data"].is_array() {
            let season_name = season_data["data"][season_data["data"].as_array().unwrap().len() - 1]["season"].as_i64().unwrap();

            info!("Latest season: {}", season_name);

            return Ok(season_name);
        }
    }

    Err(anyhow::anyhow!("Failed to get latest season"))
}

