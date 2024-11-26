use serde::Deserialize;
pub type Context<'a> = poise::Context<'a, (), Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Deserialize)]
struct ApiResponse {
    data: Data,
}

#[derive(Deserialize)]
pub struct Data {
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

pub async fn info_rank_sw() -> Result<Vec<(String, i32)>, Error> {
    let url = "https://m.swranking.com/api/player/nowline";
    let response = reqwest::get(url).await?;

    if response.status().is_success() {
        let api_response: ApiResponse = response.json().await?;

        // All ranks emotes
        let conqueror_emote_str = "<:conqueror:1310904791114842134>";
        let punisher_emote_str = "<:punisher:1310904805576937472>";
        let guardian_emote_str = "<:guardian:1310904819200032801>";

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
