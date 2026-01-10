use anyhow::{anyhow, Result};
use poise::serenity_prelude as serenity;
use reqwest::Client;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};

use crate::commands::player_stats::utils::Replay;
use crate::commands::replays::models::Root;

pub async fn get_replays_data(ids: &Vec<i32>, level: i32) -> Result<Vec<Replay>> {
    let url = "https://m.swranking.com/api/player/replayallist";
    let client = Client::new();

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
        return Err(anyhow!("Error status {}: {:?}", status, json.data.list));
    }

    // If playerOne does have the monster in the ids, swap playerOne with playerTwo
    let mut replays = json.data.list;
    for replay in &mut replays {
        if replay
            .player_one
            .monster_info_list
            .iter()
            .any(|m| ids.contains(&(m.monster_id as i32)))
        {
            // Player one has the monster, no need to swap
            continue;
        } else if replay
            .player_two
            .monster_info_list
            .iter()
            .any(|m| ids.contains(&(m.monster_id as i32)))
        {
            // Player two has the monster, swap players
            std::mem::swap(&mut replay.player_one, &mut replay.player_two);
            // if replay.status is 1, set it to 2 else if replay.status is 2, set it to 1
            if replay.status == 1 {
                replay.status = 2;
            } else if replay.status == 2 {
                replay.status = 1;
            }
        }
    }
    // Return the list of replays
    Ok(replays)
}

pub fn create_replays_embed(
    monster_names: &Vec<String>,
    level: i32,
    player_names: &Vec<String>,
) -> CreateEmbed {
    let level_str = match level {
        1 => "G1-G2",
        3 => "G3",
        4 => "P1-P3",
        _ => "Unknown",
    };

    let description = if monster_names.len() == 1 {
        format!(
            "Recent replays for **{}** - **Level**: {}",
            monster_names[0], level_str
        )
    } else {
        let monsters_list = monster_names
            .iter()
            .map(|name| format!("• **{}**", name))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "Recent replays for:\n{}\n\n**Level**: {}",
            monsters_list, level_str
        )
    };

    // Construire la chaîne des joueurs avec format en liste :
    /*
    - `PLAYER1`
    - `PLAYER2`
    - `PLAYER3`
    */
    let players_field = if player_names.is_empty() {
        "None".to_string()
    } else {
        player_names
            .iter()
            .map(|name| format!("• `{}`", name))
            .collect::<Vec<_>>()
            .join("\n")
    };

    CreateEmbed::default()
        .title("🎬 Replays")
        .description(description)
        .color(serenity::Colour::from_rgb(0, 123, 255)) // Bleu
        .image("attachment://replay.png")
        .field("Players", players_field, false)  // ← insertion du champ
        .field(
            "ℹ️ Tip",
            "Use the buttons below to view stats for different RTA ranks (P1-P3, G1-G2, G3).",
            false,
        )
        .footer(CreateEmbedFooter::new(
            "Data is gathered from m.swranking.com",
        ))
}

pub fn create_loading_replays_embed(monster_names: &Vec<String>, level: i32) -> CreateEmbed {
    let level_str = match level {
        1 => "G1-G2",
        3 => "G3",
        4 => "P1-P3",
        _ => "Unknown",
    };

    let description = if monster_names.len() == 1 {
        format!(
            "Loading replays for **{}** - **Level**: {}",
            monster_names[0], level_str
        )
    } else {
        let monsters_list = monster_names
            .iter()
            .map(|name| format!("• **{}**", name))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "Loading replays for:\n{}\n\n**Level**: {}",
            monsters_list, level_str
        )
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

pub fn create_replay_level_buttons(
    guardian_id: u64,
    punisher_id: u64,
    selected_level: i32,
    disabled: bool,
) -> serenity::CreateActionRow {
    let style_for = |level| {
        if level == selected_level {
            serenity::ButtonStyle::Primary
        } else {
            serenity::ButtonStyle::Secondary
        }
    };

    serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new("level_p1p3")
            .label("P1-P3")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: punisher_id.into(),
                name: Some("punisher".to_string()),
            })
            .style(style_for(4)),
        serenity::CreateButton::new("level_g1g2")
            .label("G1-G2")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: guardian_id.into(),
                name: Some("guardian".to_string()),
            })
            .style(style_for(1)),
        serenity::CreateButton::new("level_g3")
            .label("G3")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: guardian_id.into(),
                name: Some("guardian".to_string()),
            })
            .style(style_for(3)),
    ])
}
