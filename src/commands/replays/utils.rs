use poise::serenity_prelude as serenity;
use reqwest::Client;
use anyhow::{anyhow, Context, Result};
use serenity::builder::{CreateEmbed, CreateEmbedFooter};

use crate::commands::replays::models::Root;
use crate::commands::player_stats::utils::Replay;

pub async fn get_replays_data(ids: &Vec<i32>, level: i32) -> Result<Vec<Replay>> {
    let url = "https://m.swranking.com/api/player/replayallist";
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "pageNum":1,
        "pageSize":10,
        "level": level,
        "monsterIds": ids,
    });

    let res = client
        .post(url)
        .json(&body)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let status = res.status();
    let json: Root = res.json().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "Error status {}: {:?}",
            status,
            json.data.list
        ));
    }

    Ok(json.data.list)
}

pub fn create_replays_embed(monsters_ids: Vec<i32>) -> CreateEmbed {

    /*
    title : Replays
    description : Recent replays for : - m1 \n - m2 \n - m3 \n - m4 \n - m5
    */
    CreateEmbed::default()
    .title("Replays")
    .image("attachment://replay.png")
    .footer(CreateEmbedFooter::new(
        "Please use /send_suggestion to report any issue.",
    ))
}