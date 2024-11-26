use anyhow::{Result, Context};
use log::info;

use super::get_mob_stats_utils::{SlugApiResponse, SlugData};

pub async fn get_monster_slug(mob_name: String) -> Result<SlugData, anyhow::Error> {
    let slug_url = format!("https://api.swarena.gg/monster/search/{}", mob_name);
    let response = reqwest::get(slug_url).await.context("Failed to send request")?;

    if response.status().is_success() {
        let api_response: SlugApiResponse = response.json().await.context("Failed to parse JSON")?;

        if api_response.data.len() > 0 {
            let slug_data = SlugData{
                name: api_response.data[0].name.clone(),
                slug: api_response.data[0].slug.clone()
            };

            info!("Monster formatted: {}", slug_data.name);

            return Ok(slug_data);
        }
    }

    Err(anyhow::anyhow!("Monster not found"))
}