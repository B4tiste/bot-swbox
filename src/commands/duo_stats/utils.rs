use crate::commands::shared::models::{DuoStatsInfosData, MonsterGeneralInfoData, SlugData};


pub async fn get_monsters_duo_stats(mob_a_info: MonsterGeneralInfoData, mob_b_slug: SlugData, mob_b_info: MonsterGeneralInfoData, season: i64) -> Result<DuoStatsInfosData, String> {
    let monster_duo_stats_url = format!("https://api.swarena.gg/monster/{}/pairs?season={}&isG3=false&searchPairName={}&orderBy=win_against_rate&orderDirection=DESC&minPlayedAgainst=0&minPlayedTogether=0&limit=5&offset=0", mob_a_info.id, season, mob_b_slug.slug.to_lowercase());
    let response = reqwest::get(monster_duo_stats_url).await.map_err(|_| "Failed to send request".to_string())?;

    if response.status().is_success() {
        let api_response: serde_json::Value = response.json().await.map_err(|_| "Failed to parse JSON".to_string())?;

        // Vérifie que les données sont présentes
        if !api_response["data"].is_null() {
            // Trouver l'entrée avec le bon b_monster_id
            for i in 0..api_response["data"].as_array().unwrap().len() {
                let data = &api_response["data"][i];
                if data["b_monster_id"].as_i64().unwrap() == mob_b_info.id as i64 {
                    return Ok(DuoStatsInfosData {
                        b_monster_image_filename: data["b_monster_image_filename"].as_str().ok_or("Missing b_monster_image_filename")?.to_string(),
                        win_against_rate: data["win_against_rate"].to_string(),
                        win_together_rate: data["win_together_rate"].to_string(),
                    });
                }
            }
        }
    }

    Err("Data not found".to_string())
}