use crate::{GUARDIAN_EMOJI_ID, PUNISHER_EMOJI_ID};
use serde::Deserialize;

/// Global toggle for prediction gathering (set to false to disable)
pub const ENABLE_PREDICTION: bool = false;

// ---------- Live thresholds (JSON API) ----------

#[derive(Deserialize)]
struct ApiResponse {
    data: RankData,
}

#[derive(Deserialize)]
pub struct RankData {
    // On ne garde que P2, P3, G1, G2, G3
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

/// Live thresholds from https://m.swranking.com/api/player/nowline
/// Returns in fixed order: P2,P3,G1,G2,G3
pub async fn get_rank_info() -> Result<Vec<(String, i32)>, String> {
    let url = "https://m.swranking.com/api/player/nowline";
    let response = match reqwest::get(url).await {
        Ok(response) => response,
        Err(_) => return Err("Error sending the request.".into()),
    };

    if !response.status().is_success() {
        return Err("Failed to fetch data from API".into());
    }

    let api_response: ApiResponse = match response.json().await {
        Ok(api_response) => api_response,
        Err(_) => return Err("Error converting to JSON".into()),
    };

    let punisher_emote_str = format!("<:punisher:{}>", PUNISHER_EMOJI_ID.lock().unwrap());
    let guardian_emote_str = format!("<:guardian:{}>", GUARDIAN_EMOJI_ID.lock().unwrap());

    let scores = vec![
        (punisher_emote_str.repeat(2), api_response.data.s2.score), // P2
        (punisher_emote_str.repeat(3), api_response.data.s3.score), // P3
        (guardian_emote_str.repeat(1), api_response.data.g1.score), // G1
        (guardian_emote_str.repeat(2), api_response.data.g2.score), // G2
        (guardian_emote_str.repeat(3), api_response.data.g3.score), // G3
    ];

    Ok(scores)
}

// ---------- Prediction thresholds (HTML scraping) ----------

/// Fetch predicted thresholds from https://swrta.top/predict
/// Returns in fixed order: P2,P3,G1,G2,G3
pub async fn get_prediction_info() -> Result<Vec<(String, i32)>, String> {
    let url = "https://swrta.top/predict";
    let resp = reqwest::get(url)
        .await
        .map_err(|_| "Error sending the request.".to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Non-success status: {}", resp.status()));
    }

    let html = resp
        .text()
        .await
        .map_err(|_| "Error reading response body".to_string())?;

    // Parse the HTML and extract pairs like ("P2", 1300), ...
    let document = scraper::Html::parse_document(&html);
    let box_sel = scraper::Selector::parse(".predict_box .point_box").unwrap();
    let rank_sel = scraper::Selector::parse(".rank_icon").unwrap();
    let val_sel = scraper::Selector::parse(".col-8").unwrap();

    use std::collections::HashMap;
    let mut found: HashMap<String, i32> = HashMap::new();

    for el in document.select(&box_sel) {
        let rank = el
            .select(&rank_sel)
            .next()
            .map(|n| n.text().collect::<String>().trim().to_string())
            .ok_or_else(|| "Missing .rank_icon".to_string())?;

        let val_text = el
            .select(&val_sel)
            .next()
            .map(|n| n.text().collect::<String>().trim().to_string())
            .ok_or_else(|| "Missing .col-8 (value)".to_string())?;

        let score: i32 = val_text
            .replace(',', "")
            .parse()
            .map_err(|_| format!("Invalid number for {rank}: {val_text}"))?;

        found.insert(rank, score);
    }

    let punisher_emote_str = format!("<:punisher:{}>", PUNISHER_EMOJI_ID.lock().unwrap());
    let guardian_emote_str = format!("<:guardian:{}>", GUARDIAN_EMOJI_ID.lock().unwrap());

    fn emotes_for(rank: &str, p: &str, g: &str) -> String {
        // rank like "P2", "G3"
        let (letter, num) = rank.split_at(1);
        let repeats = num.parse::<usize>().unwrap_or(1).max(1).min(3);
        match letter {
            "P" => p.repeat(repeats),
            "G" => g.repeat(repeats),
            _ => p.repeat(1),
        }
    }

    let order = ["P2", "P3", "G1", "G2", "G3"];
    let mut out: Vec<(String, i32)> = Vec::with_capacity(order.len());

    for key in order {
        let score = *found
            .get(key)
            .ok_or_else(|| format!("Missing prediction for {key}"))?;
        let emote = emotes_for(key, &punisher_emote_str, &guardian_emote_str);
        out.push((emote, score));
    }

    Ok(out)
}
