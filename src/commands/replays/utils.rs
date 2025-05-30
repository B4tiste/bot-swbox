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
        "pageSize":16,
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

pub fn create_replays_embed(monster_names: &Vec<String>, level: i32) -> CreateEmbed {
    let level_str = match level {
        0 => "C1-C3",
        1 => "G1-G2",
        3 => "G3",
        4 => "P1-P3",
        _ => "Unknown",
    };

    let description = if monster_names.len() == 1 {
        format!("Recent replays for **{}** - **Level**: {}", monster_names[0], level_str)
    } else {
        let monsters_list = monster_names
            .iter()
            .map(|name| format!("• {}", name))
            .collect::<Vec<_>>()
            .join("\n");

        format!("Recent replays for:\n{}\n\n**Level**: {}", monsters_list, level_str)
    };

    CreateEmbed::default()
        .title("🎬 Replays")
        .description(description)
        .color(serenity::Colour::from_rgb(0, 123, 255)) // Bleu
        .image("attachment://replay.png")
        .footer(CreateEmbedFooter::new(
            "Use the buttons below to view replays for different RTA ranks • Use /send_suggestion to report issues.",
        ))
}

pub fn create_loading_replays_embed(monster_names: &Vec<String>, level: i32) -> CreateEmbed {
    let level_str = match level {
        0 => "C1-C3",
        1 => "G1-G2",
        3 => "G3",
        4 => "P1-P3",
        _ => "Unknown",
    };

    let description = if monster_names.len() == 1 {
        format!("Loading replays for **{}** - **Level**: {}", monster_names[0], level_str)
    } else {
        let monsters_list = monster_names
            .iter()
            .map(|name| format!("• {}", name))
            .collect::<Vec<_>>()
            .join("\n");

        format!("Loading replays for:\n{}\n\n**Level**: {}", monsters_list, level_str)
    };

    CreateEmbed::default()
        .title("🎬 Replays")
        .description(description)
        .color(serenity::Colour::from_rgb(255, 165, 0)) // Orange pour le chargement
        .field(
            "Status",
            "<a:loading:1358029412716515418> Loading new replay data...",
            false,
        )
        .footer(CreateEmbedFooter::new(
            "Please wait while we fetch the replay data...",
        ))
}