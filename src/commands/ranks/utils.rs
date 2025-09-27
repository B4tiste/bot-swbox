use crate::{CONQUEROR_EMOJI_ID, GUARDIAN_EMOJI_ID, PUNISHER_EMOJI_ID};
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

// ---------- Prediction thresholds (HTML scraping) ----------

/// Fetch predicted thresholds from https://swrta.top/predict
/// Returns the same (emote_string, score) vector order: C1,C2,C3,P1,P2,P3,G1,G2,G3
pub async fn get_prediction_info() -> Result<Vec<(String, i32)>, String> {
    let url = "https://swrta.top/predict";
    let resp = reqwest::get(url).await.map_err(|_| "Error sending the request.".to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Non-success status: {}", resp.status()));
    }
    let html = resp.text().await.map_err(|_| "Error reading response body".to_string())?;

    // Parse the HTML and extract pairs like ("C1", 1300), ...
    // We use the 'scraper' crate's CSS selectors.
    let document = scraper::Html::parse_document(&html);
    let box_sel = scraper::Selector::parse(".predict_box .point_box").unwrap();
    let rank_sel = scraper::Selector::parse(".rank_icon").unwrap();
    let val_sel  = scraper::Selector::parse(".col-8").unwrap();

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
            .replace(',', "") // just in case formatting changes
            .parse()
            .map_err(|_| format!("Invalid number for {rank}: {val_text}"))?;

        found.insert(rank, score);
    }

    // Build in fixed order, converting to your emote strings
    let conqueror_emote_str = format!("<:conqueror:{}>", CONQUEROR_EMOJI_ID.lock().unwrap());
    let punisher_emote_str  = format!("<:punisher:{}>",  PUNISHER_EMOJI_ID.lock().unwrap());
    let guardian_emote_str  = format!("<:guardian:{}>",  GUARDIAN_EMOJI_ID.lock().unwrap());

    fn emotes_for(rank: &str, c: &str, p: &str, g: &str) -> String {
        // rank like "C1", "P3", "G2"
        let (letter, num) = rank.split_at(1);
        let repeats = num.parse::<usize>().unwrap_or(1).max(1).min(3);
        match letter {
            "C" => c.repeat(repeats),
            "P" => p.repeat(repeats),
            "G" => g.repeat(repeats),
            _   => c.repeat(1),
        }
    }

    let order = ["C1","C2","C3","P1","P2","P3","G1","G2","G3"];
    let mut out: Vec<(String,i32)> = Vec::with_capacity(9);
    for key in order {
        let score = *found.get(key).ok_or_else(|| format!("Missing prediction for {key}"))?;
        let emote = emotes_for(key, &conqueror_emote_str, &punisher_emote_str, &guardian_emote_str);
        out.push((emote, score));
    }

    Ok(out)
}
