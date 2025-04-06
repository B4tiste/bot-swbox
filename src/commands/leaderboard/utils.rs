use anyhow::{anyhow, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LeaderboardPlayer {
    #[serde(rename = "playerName")]
    pub name: String,
    // #[serde(rename = "swrtPlayerId")]
    // pub swrt_player_id: i64,
    #[serde(rename = "playerScore")]
    pub player_elo: i64,
    #[serde(rename = "playerCountry")]
    pub player_country: String,
}

pub async fn get_leaderboard_data(token: &str, page: &i32) -> Result<Vec<LeaderboardPlayer>> {
    let url = "https://m.swranking.com/api/player/list";
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "pageNum": page,
        "pageSize": 15,
        "playerName": "",
        "online": false,
        "level": null,
        "playerMonsters": []
    });

    let res = client
        .post(url)
        .json(&body)
        .header("Authentication", token)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let status = res.status();
    let resp_json: serde_json::Value = res.json().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "Error status {}: {:?}",
            status,
            resp_json["enMessage"]
        ));
    }

    let players = resp_json["data"]["list"]
        .as_array()
        .ok_or_else(|| anyhow!("Failed to parse player list"))?
        .iter()
        .map(|player| {
            serde_json::from_value::<LeaderboardPlayer>(player.clone())
                .map_err(|e| anyhow!("Failed to parse player: {}", e))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(players)
}
