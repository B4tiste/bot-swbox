use crate::commands::shared::models::MonsterRtaInfoData;

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