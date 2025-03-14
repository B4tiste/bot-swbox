use crate::{CONQUEROR_EMOJI_ID, GUARDIAN_EMOJI_ID, PUNISHER_EMOJI_ID};
use serde::Deserialize;

#[derive(Deserialize)]
struct ApiResponse {
    data: RankData,
}

#[derive(Deserialize)]
pub struct RankData {
    c1: RankInfo,
    c2: RankInfo,
    c3: RankInfo,
    s1: RankInfo,
    s2: RankInfo,
    s3: RankInfo,
    g1: RankInfo,
    g2: RankInfo,
    g3: RankInfo,
}

#[derive(Deserialize)]
struct RankInfo {
    score: i32,
}

pub async fn get_rank_info() -> Result<Vec<(String, i32)>, String> {
    let url = "https://m.swranking.com/api/player/nowline";
    let response = match reqwest::get(url).await {
        Ok(response) => response,
        Err(_) => return Err("Error sending the request.".into()),
    };

    if response.status().is_success() {
        let api_response: ApiResponse = match response.json().await {
            Ok(api_response) => api_response,
            Err(_) => return Err("Error converting to JSON".into()),
        };

        let conqueror_emote_str = format!("<:conqueror:{}>", CONQUEROR_EMOJI_ID.lock().unwrap());
        let punisher_emote_str = format!("<:punisher:{}>", PUNISHER_EMOJI_ID.lock().unwrap());
        let guardian_emote_str = format!("<:guardian:{}>", GUARDIAN_EMOJI_ID.lock().unwrap());

        let scores = vec![
            (conqueror_emote_str.repeat(1), api_response.data.c1.score),
            (conqueror_emote_str.repeat(2), api_response.data.c2.score),
            (conqueror_emote_str.repeat(3), api_response.data.c3.score),
            (punisher_emote_str.repeat(1), api_response.data.s1.score),
            (punisher_emote_str.repeat(2), api_response.data.s2.score),
            (punisher_emote_str.repeat(3), api_response.data.s3.score),
            (guardian_emote_str.repeat(1), api_response.data.g1.score),
            (guardian_emote_str.repeat(2), api_response.data.g2.score),
            (guardian_emote_str.repeat(3), api_response.data.g3.score),
        ];

        Ok(scores)
    } else {
        Err("Failed to fetch data from API".into())
    }
}
