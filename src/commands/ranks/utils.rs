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

        let scores = vec![
            ("C1".to_string(), api_response.data.c1.score),
            ("C2".to_string(), api_response.data.c2.score),
            ("C3".to_string(), api_response.data.c3.score),
            ("P1".to_string(), api_response.data.s1.score),
            ("P2".to_string(), api_response.data.s2.score),
            ("P3".to_string(), api_response.data.s3.score),
            ("G1".to_string(), api_response.data.g1.score),
            ("G2".to_string(), api_response.data.g2.score),
            ("G3".to_string(), api_response.data.g3.score),
        ];

        Ok(scores)
    } else {
        Err("Failed to fetch data from API".into())
    }
}
