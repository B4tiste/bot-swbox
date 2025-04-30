use crate::commands::rta_core::models::{Monster, MonsterEntry, MonstersFile};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use anyhow::{Context, Result};

/// Lit le JSON dynamique (upload), extrait les unit_master_id,
/// puis charge monsters.json et renvoie les Monster correspondants.
pub fn get_monsters_from_json_bytes(
    upload_bytes: &[u8],
    monsters_json_path: &str,
) -> Result<Vec<Monster>> {
    // 1) Parser le JSON uploadé
    let dynamic: Value = serde_json::from_slice(upload_bytes)
        .context("Failed to parse uploaded JSON")?;

    // 2) Extraire la liste des unit_master_id
    let unit_list = dynamic
        .get("unit_list")
        .and_then(|v| v.as_array())
        .context("Champ unit_list introuvable ou pas un tableau")?;
    let wanted_ids: HashSet<u32> = unit_list
        .iter()
        .filter_map(|u| u.get("unit_master_id")?.as_u64().map(|id| id as u32))
        .collect();

    // 3) Lire et parser monsters.json
    let monsters_data = fs::read_to_string(monsters_json_path)
        .context("Impossible de lire monsters.json")?;
    let all: MonstersFile = serde_json::from_str(&monsters_data)
        .context("Impossible de parser monsters.json")?;

    // 4) Filtrer selon unit_list **et** vos critères d’éveil / étoiles
    let result = all
    .monsters
    .into_iter()
    .filter(|m: &MonsterEntry| {
        // doit appartenir à unit_list
        if !wanted_ids.contains(&m.com2us_id) {
            return false;
        }
        // awaken_level ≥ 1
        if m.awaken_level < 1 {
            return false;
        }
        // règle par élément
        match m.element.as_str() {
            "Fire" | "Water" | "Wind" => m.natural_stars == 5,
            "Light" | "Dark"          => m.natural_stars >= 4,
            _                         => false,
        }
    })
    .map(|m| Monster {
        unit_master_id: m.com2us_id,
        image_filename: m.image_filename,
        element: m.element,
        awaken_level: m.awaken_level,
        natural_stars: m.natural_stars,
        name: m.name,
    })
    .collect();

    Ok(result)
}