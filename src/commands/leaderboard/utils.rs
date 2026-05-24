use anyhow::{anyhow, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LeaderboardResponse {
    pub count: i64,
    pub data: Vec<LeaderboardPlayer>,
}

#[derive(Debug, Deserialize)]
pub struct LeaderboardPlayer {
    pub player_id: i64,
    pub username: String,
    pub country: String,
    pub current_score: i64,
    pub rank: i64,
}

pub async fn get_leaderboard_data(
    season: i32,
    page: i32,
    page_size: i32,
) -> Result<LeaderboardResponse> {
    let safe_page = page.max(1);
    let safe_size = page_size.max(1);
    let offset = (safe_page - 1) * safe_size;

    let url = format!(
        "https://api.lucksack.gg/players/leaderboard?season={}&limit={}&offset={}",
        season, safe_size, offset
    );

    let client = reqwest::Client::new();

    let res = client
        .get(&url)
        .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .header("sec-fetch-site", "none")
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(anyhow!("Error status {}", res.status()));
    }

    res.json::<LeaderboardResponse>()
        .await
        .map_err(|e| anyhow!("Failed to parse leaderboard JSON: {}", e))
}
