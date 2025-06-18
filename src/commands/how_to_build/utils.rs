use std::{collections::HashMap, path::Path};

use crate::commands::how_to_build::models::MonsterStats;
use poise::serenity_prelude as serenity;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};

use serde_json;
use std::io;

use crate::commands::how_to_build::models::MonsterElementList;

pub fn load_monster_stats<P: AsRef<Path>>(
    path: P,
) -> Result<HashMap<String, MonsterStats>, MonsterStatsLoadError> {
    let content = std::fs::read_to_string(path)?;
    let data: HashMap<String, MonsterStats> = serde_json::from_str(&content)?;
    Ok(data)
}

/// Une erreur concrète et `Send` pour la lecture JSON
#[allow(dead_code)]
#[derive(Debug)]
pub enum MonsterStatsLoadError {
    Io(io::Error),
    Json(serde_json::Error),
}

impl From<io::Error> for MonsterStatsLoadError {
    fn from(err: io::Error) -> Self {
        MonsterStatsLoadError::Io(err)
    }
}

impl From<serde_json::Error> for MonsterStatsLoadError {
    fn from(err: serde_json::Error) -> Self {
        MonsterStatsLoadError::Json(err)
    }
}

/// Formate un embed à partir des statistiques d’un monstre
pub fn format_monster_stats(
    monster_name: &str,
    stats: &MonsterStats,
    image_url: Option<String>,
) -> CreateEmbed {
    let stat_lines = format_stats_display(stats);

    let sets_text = vec![&stats.set1, &stats.set2, &stats.set3]
        .iter()
        .filter_map(|opt| opt.as_ref())
        .map(|s| {
            // Tente d’extraire le pourcentage entre parenthèses
            if let Some((main, percent)) = s.rsplit_once('(') {
                let percent = percent.trim_end_matches(')');
                format!("• {} — {}", percent.trim(), main.trim())
            } else {
                format!("• {}", s.trim())
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut embed = CreateEmbed::default()
        .title(format!(
            "Average stats for {} in G+",
            monster_name.split(" - ").next().unwrap_or(monster_name)
        ))
        .color(serenity::Colour::from_rgb(120, 153, 255))
        .footer(CreateEmbedFooter::new(
            "Please use /send_suggestion to report any issue.",
        ))
        .field(
            "Stats",
            format!("```autohotkey\n{}\n```", stat_lines.join("\n")),
            false,
        );

    if !sets_text.is_empty() {
        embed = embed.field("Sets", sets_text, false);
    }

    if let Some(url) = image_url {
        embed = embed.thumbnail(&url);
    }

    if let Some(arti_1) = &stats.arti_1 {
        if !arti_1.is_empty() {
            let arti_1_text = arti_1
                .iter()
                .map(|s| format!("• {}", s))
                .collect::<Vec<_>>()
                .join("\n");
            embed = embed.field(
                "Left Artifact (From most to least used)",
                arti_1_text,
                false,
            );
        }
    }

    if let Some(arti_2) = &stats.arti_2 {
        if !arti_2.is_empty() {
            let arti_2_text = arti_2
                .iter()
                .map(|s| format!("• {}", s))
                .collect::<Vec<_>>()
                .join("\n");
            embed = embed.field(
                "Right Artifact (From most to least used)",
                arti_2_text,
                false,
            );
        }
    }

    embed
}

fn format_stats_display(stats: &MonsterStats) -> Vec<String> {
    let mut lines = vec![];

    let data = vec![
        ("hp", "HP", &stats.hp),
        ("atk", "ATK", &stats.atk),
        ("def", "DEF", &stats.def),
        ("spd", "SPD", &stats.speed),
        ("crit_rate", "CRate", &stats.crit_rate),
        ("crit_damage", "CDmg", &stats.crit_damage),
        ("resistance", "RES", &stats.resistance),
        ("accuracy", "ACC", &stats.accuracy),
    ];

    // 1. Parser tous les champs, en séparant base/bonus si besoin
    let mut parsed_stats = Vec::new();
    let mut left_width = 0;

    for (_key, label, value) in &data {
        let (base, bonus) = if value.contains('+') {
            let parts: Vec<&str> = value.split('+').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                (parts[0].to_string(), Some(parts[1].to_string()))
            } else {
                (value.to_string(), None)
            }
        } else {
            (value.to_string(), None)
        };

        left_width = left_width.max(base.len());
        parsed_stats.push((label.to_string(), base, bonus));
    }

    // 2. Formatter les lignes avec alignement
    for (label, base, bonus_opt) in parsed_stats {
        match bonus_opt {
            Some(bonus) => {
                lines.push(format!(
                    "{:<8}: {:>width$} + {}",
                    label,
                    base,
                    bonus,
                    width = left_width
                ));
            }
            None => {
                lines.push(format!(
                    "{:<8}: {:>width$}",
                    label,
                    base,
                    width = left_width
                ));
            }
        }
    }

    lines
}

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
