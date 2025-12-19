use anyhow::{Context, Result};
use mongodb::bson::{doc, Document};
use mongodb::Collection;
use reqwest::Client;
use std::collections::HashMap;

use poise::serenity_prelude as serenity;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};

use crate::commands::best_pve_teams::models::{ApiResponse, DungeonTeamData, MonstersFile};

pub async fn get_dungeon_stats(dungeon_id: u32) -> Result<Vec<DungeonTeamData>> {
    let client = Client::new();

    let resp = client
        .get("https://swcalc.cz/api/dungeon-teams")
        .query(&[
            ("dungeon_id", dungeon_id.to_string()),
            ("sort_by", "rank_score".to_string()),
        ])
        .header("Accept", "application/json")
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .context("Failed download dungeon data")?;

    // println!("Response : {:?}", resp);

    let api = resp
        .json::<ApiResponse>()
        .await
        .context("Failed to parse JSON")?;

    Ok(api.data)
}

// Crée un embed pour afficher les meilleures équipes PvE
pub async fn create_pve_teams_embed(
    dungeon_name: &str,
    dungeon_slug: &str,
    teams: &[DungeonTeamData],
    collection: &Collection<Document>,
    monster_name_map: &HashMap<String, String>,
) -> CreateEmbed {
    let thumbnail = "https://raw.githubusercontent.com/B4tiste/landing-page-bot/refs/heads/main/src/assets/images/old_bot_logo.gif";

    let mut embed = CreateEmbed::default()
        .title(format!("🏆 Best PvE Teams - {}", dungeon_name))
        .color(serenity::Colour::from_rgb(0, 255, 127))
        .thumbnail(thumbnail)
        .footer(CreateEmbedFooter::new("Data is gathered from swcalc.cz"));

    for (i, team) in teams.iter().enumerate() {
        let mut monsters_line = String::new();
        let mut missing_names: Vec<String> = Vec::new();

        for img_id in &team.members {
            if let Some(emoji) = get_emoji_from_img_id(collection, img_id.clone()).await {
                monsters_line.push_str(&emoji);
                monsters_line.push(' ');
            } else {
                // emoji manquant -> + NAME
                let name = monster_name_map
                    .get(img_id)
                    .cloned()
                    .unwrap_or_else(|| img_id.clone()); // fallback si pas trouvé dans le json
                missing_names.push(name);
            }
        }

        if monsters_line.is_empty() {
            monsters_line = "*No emojis found*".to_string();
        } else {
            monsters_line = monsters_line.trim_end().to_string();
        }

        // Ajoute les "+ NAME" à la suite
        if !missing_names.is_empty() {
            for name in missing_names {
                monsters_line.push_str(&format!(" + {}", name));
            }
        }

        let avg_str = format_duration(team.average_time_ms);

        let value = format!(
            "**Monsters :** {}\n\
             Success rate and average time : **{:.2}** %, **{}**\n\
             Score : {:.2}\n\
             [Runes/Artifacts setup and run time distribution](https://swcalc.cz/team-detail?team={})",
            monsters_line,
            team.success_rate,
            avg_str,
            team.rank,
            team.id,
        );

        embed = embed.field(format!("Team {}", i + 1), value, false);
    }

    embed = embed.field(
        "Other teams",
        format!(
            "[Click here to check other teams for **{}**](https://swcalc.cz/dungeons/{})",
            dungeon_name, dungeon_slug
        ),
        true,
    );

    embed = embed.field(
        "Note",
        "The success rate and average time may vary depending on runes and artifacts quality.",
        true,
    );

    embed
}


// Convertit ms -> mm:ss.mmm (ex: 01:12.345)
fn format_duration(ms: u32) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let millis = ms % 1000;
    format!("{:02}:{:02}.{:03}", minutes, seconds, millis)
}

pub async fn get_emoji_from_img_id(
    collection: &Collection<mongodb::bson::Document>,
    image_id: String,
) -> Option<String> {
    let emoji_doc = collection
        .find_one(doc! { "name": image_id })
        .await
        .ok()??;

    let emoji_id = emoji_doc.get_str("id").ok()?;
    let emoji_name = emoji_doc.get_str("name").ok()?;

    Some(format!("<:{}:{}>", emoji_name, emoji_id))
}

/// Construit une map: "0003_0_1" => "Forest Keeper - Fire"
pub fn build_monster_name_map() -> HashMap<String, String> {
    // adapte le chemin à ton projet (ex: assets/monsters_elements.json)
    let raw = include_str!("../../../monsters_elements.json");

    let parsed: MonstersFile =
        serde_json::from_str(raw).expect("Failed to parse monsters_elements.json");

    let mut map = HashMap::with_capacity(parsed.monsters.len());

    for m in parsed.monsters {
        if let Some(core) = core_id_from_image_filename(&m.image_filename) {
            // si doublon, on garde le premier (ou remplace, comme tu veux)
            map.entry(core).or_insert(m.name);
        }
    }

    map
}

/// "unit_icon_0003_0_1.png" -> Some("0003_0_1")
fn core_id_from_image_filename(filename: &str) -> Option<String> {
    // Tolérant (png/jpg/…)
    let base = filename.strip_prefix("unit_icon_")?;
    let core = base.split('.').next()?; // "0003_0_1"
    Some(core.to_string())
}