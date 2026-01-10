// commands/how_to_build/utils.rs
use std::{collections::HashMap, path::Path};

use poise::serenity_prelude as serenity;
use reqwest::Client;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};

use serde_json;
use std::io;

use crate::commands::how_to_build::models::{
    LucksackBuildResponse, LucksackSeason, MonsterElementList,
};
use crate::{CONQUEROR_EMOJI_ID, GUARDIAN_EMOJI_ID, PUNISHER_EMOJI_ID};

pub async fn get_latest_lucksack_season() -> Result<i32, String> {
    let url = "https://api.lucksack.gg/seasons";

    let client = Client::new();
    let res = client
        .get(url)
        .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .header("sec-fetch-site", "none")
        .send()
        .await
        .map_err(|_| "Failed to send request".to_string())?;

    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }

    let seasons = res
        .json::<Vec<LucksackSeason>>()
        .await
        .map_err(|_| "Failed to parse seasons JSON".to_string())?;

    seasons
        .into_iter()
        .filter_map(|s| s.season_number)
        .max()
        .ok_or_else(|| "No valid season_number found".to_string())
}

// ---------------------------
// Load image mapping (comme avant)
// ---------------------------
pub fn load_monster_images<P: AsRef<Path>>(path: P) -> Result<HashMap<String, String>, io::Error> {
    let content = std::fs::read_to_string(path)?;
    let monster_list: MonsterElementList = serde_json::from_str(&content)?;

    let map = monster_list
        .monsters
        .into_iter()
        .map(|m| (m.name.to_lowercase(), m.image_filename))
        .collect();

    Ok(map)
}

// ---------------------------
// Lucksack fetch
// ---------------------------
pub async fn fetch_lucksack_build(
    monster_id: i32,
    season: i32,
    rank: i32,
) -> Result<LucksackBuildResponse, String> {
    let url = format!(
        "https://api.lucksack.gg/monsters/{}/stats?season={}&rank={}",
        monster_id, season, rank
    );

    let client = Client::new();
    let res = client
        .get(&url)
        .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .header("sec-fetch-site", "none")
        .send()
        .await
        .map_err(|_| "Failed to send request".to_string())?;

    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }

    res.json::<LucksackBuildResponse>()
        .await
        .map_err(|_| "Failed to parse JSON".to_string())
}

// ---------------------------
// Mapping constants
// ---------------------------
fn rune_set_name(id: i32) -> &'static str {
    match id {
        1 => "Energy",
        2 => "Fatal",
        3 => "Blade",
        4 => "Rage",
        5 => "Swift",
        6 => "Focus",
        7 => "Guard",
        8 => "Endure",
        9 => "Violent",
        10 => "Will",
        11 => "Nemesis",
        12 => "Shield",
        13 => "Revenge",
        14 => "Despair",
        15 => "Vampire",
        16 => "Destroy",
        17 => "Fight",
        18 => "Determination",
        19 => "Enhance",
        20 => "Accuracy",
        21 => "Tolerance",
        22 => "Intangible",
        23 => "Seal",
        _ => "Unknown",
    }
}

fn stat_short(id: i32) -> &'static str {
    match id {
        1 => "HP",
        2 => "HP%",
        3 => "ATK",
        4 => "ATK%",
        5 => "DEF",
        6 => "DEF%",
        8 => "SPD",
        9 => "CR",
        10 => "CD",
        11 => "RES",
        12 => "ACC",
        _ => "?",
    }
}

fn rank_label(rank: i32) -> &'static str {
    match rank {
        0 => "G3",
        1 => "G1-G2",
        2 => "P1-P3",
        3 => "C1-C3",
        _ => "Unknown",
    }
}

// ✅ 1 chiffre après la virgule
fn fmt_pct(x: f32) -> String {
    format!("{:.1}%", x * 100.0)
}

// ---------------------------
// Formatters
// ---------------------------
fn format_top_rune_sets(build: &LucksackBuildResponse, top_n: usize) -> String {
    if build.rune_sets.is_empty() {
        return "No rune set data.".to_string();
    }

    let mut sets = build.rune_sets.clone();
    sets.sort_by(|a, b| {
        b.pickrate
            .partial_cmp(&a.pickrate)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    sets.into_iter()
        .take(top_n)
        .enumerate()
        .map(|(i, s)| {
            let primary = rune_set_name(s.primary_set);
            let secondary = s.secondary_set.map(rune_set_name);
            let tertiary = s.tertiary_set.map(rune_set_name);

            let name = match (secondary, tertiary) {
                (Some(b), Some(c)) => format!("{} + {} + {}", primary, b, c),
                (Some(b), None) => format!("{} + {}", primary, b),
                (None, Some(c)) => format!("{} + {}", primary, c),
                (None, None) => primary.to_string(),
            };

            format!(
                "{}. **{}** : {} / {}",
                i + 1,
                name,
                fmt_pct(s.winrate),
                fmt_pct(s.pickrate),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ✅ malus si WR < 50% (score utilisé pour trier)
fn adjusted_slot_score(pickrate: f32, winrate: f32) -> f32 {
    let base = pickrate * winrate;

    let factor = if winrate < 0.5 {
        (winrate / 0.5).clamp(0.0, 1.0)
    } else {
        1.0
    };

    base * factor
}

fn format_top_slots(build: &LucksackBuildResponse, top_n: usize) -> String {
    if build.slot_stats.is_empty() {
        return "No slot stats data.".to_string();
    }

    let mut slots = build.slot_stats.clone();
    slots.sort_by(|a, b| {
        let sa = adjusted_slot_score(a.pickrate, a.winrate);
        let sb = adjusted_slot_score(b.pickrate, b.winrate);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    slots
        .into_iter()
        .take(top_n)
        .enumerate()
        .map(|(i, s)| {
            format!(
                "{}. **{} / {} / {}** : {} / {}",
                i + 1,
                stat_short(s.slot_two),
                stat_short(s.slot_four),
                stat_short(s.slot_six),
                fmt_pct(s.winrate),
                fmt_pct(s.pickrate),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------
// Embed builder
// ---------------------------
pub fn build_how_to_build_embed(
    monster_name: &str,
    season: i32,
    rank: i32,
    build: &LucksackBuildResponse,
    image_url: Option<String>,
) -> CreateEmbed {
    let top_sets = format_top_rune_sets(build, 3);
    let top_slots = format_top_slots(build, 3);

    let mut embed = serenity::CreateEmbed::default()
        .title(format!(
            "How to build - {} - Season {}",
            monster_name.split(" - ").next().unwrap_or(monster_name),
            season
        ))
        .description(format!("**Rank**: {}", rank_label(rank)))
        .color(serenity::Colour::from_rgb(120, 153, 255))
        .field("Top Rune Sets [WinRate - PickRate]", top_sets, false)
        .field("Top Slot 2/4/6 [WinRate - PickRate]", top_slots, false)
        .footer(CreateEmbedFooter::new("Data is gathered from lucksack.gg"));

    if let Some(url) = image_url {
        embed = embed.thumbnail(url);
    }

    embed
}

// ---------------------------
// Buttons (rank lucksack)
// ---------------------------
pub fn create_lucksack_rank_buttons(
    selected_rank: i32,
    disabled: bool,
) -> serenity::CreateActionRow {
    let conqueror_id: u64 = CONQUEROR_EMOJI_ID.lock().unwrap().parse().unwrap();
    let guardian_id: u64 = GUARDIAN_EMOJI_ID.lock().unwrap().parse().unwrap();
    let punisher_id: u64 = PUNISHER_EMOJI_ID.lock().unwrap().parse().unwrap();

    let style_for = |rank| {
        if rank == selected_rank {
            serenity::ButtonStyle::Primary
        } else {
            serenity::ButtonStyle::Secondary
        }
    };

    serenity::CreateActionRow::Buttons(vec![
        // C1-C3 (rank 3)
        serenity::CreateButton::new("rank_c1c3")
            .label("C1-C3")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: conqueror_id.into(),
                name: Some("conqueror".to_string()),
            })
            .style(style_for(3)),
        // P1-P3 (rank 2)
        serenity::CreateButton::new("rank_p1p3")
            .label("P1-P3")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: punisher_id.into(),
                name: Some("punisher".to_string()),
            })
            .style(style_for(2)),
        // G1-G2 (rank 1)
        serenity::CreateButton::new("rank_g1g2")
            .label("G1-G2")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: guardian_id.into(),
                name: Some("guardian".to_string()),
            })
            .style(style_for(1)),
        // G3 (rank 0)
        serenity::CreateButton::new("rank_g3")
            .label("G3")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: guardian_id.into(),
                name: Some("guardian".to_string()),
            })
            .style(style_for(0)),
    ])
}
