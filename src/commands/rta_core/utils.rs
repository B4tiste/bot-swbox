use crate::commands::mob_stats::utils::remap_monster_id;
use crate::commands::rta_core::models::{
    LucksackPatch, LucksackTrioRecord, LucksackTrioResponse, LucksackWithTrioResponse, Monster,
    MonsterEntry, MonstersFile, TierListData, TrioStat,
};
use crate::commands::shared::clients::http_client;
use anyhow::{Context, Result};
use chrono::NaiveDate;
use mongodb::{bson::doc, Collection};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;

/// Nombre maximal de pages (100 trios chacune) récupérées par appel Lucksack.
const MAX_TRIO_PAGES: i32 = 15;

/// Lit le JSON dynamique (upload), extrait les unit_master_id,
/// puis charge monsters.json et renvoie les Monster correspondants.
pub fn get_monsters_from_json_bytes(
    upload_bytes: &[u8],
    monsters_json_path: &str,
) -> Result<Vec<Monster>> {
    // 1) Parser le JSON uploadé
    let dynamic: Value =
        serde_json::from_slice(upload_bytes).context("Failed to parse uploaded JSON")?;

    // 2) Extraire la liste des unit_master_id
    let unit_list = dynamic
        .get("unit_list")
        .and_then(|v| v.as_array())
        .context("Champ unit_list introuvable ou pas un tableau")?;
    let wanted_ids: HashSet<u32> = unit_list
        .iter()
        .filter_map(|u| {
            u.get("unit_master_id")?
                .as_u64()
                .map(|id| remap_monster_id(id as i32) as u32)
        })
        .collect();

    // 3) Lire et parser monsters.json
    let monsters_data =
        fs::read_to_string(monsters_json_path).context("Impossible de lire monsters.json")?;
    let all: MonstersFile =
        serde_json::from_str(&monsters_data).context("Impossible de parser monsters.json")?;

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
                "Fire" | "Water" | "Wind" => m.natural_stars >= 3,
                "Light" | "Dark" => m.natural_stars >= 3,
                _ => false,
            }
        })
        .map(|m| Monster {
            unit_master_id: m.com2us_id,
        })
        .collect();

    Ok(result)
}

pub async fn get_tierlist_data(api_level: i32, token: &str) -> Result<TierListData, String> {
    let url = format!(
        "https://m.swranking.com/api/monsterBase/getMonsterLevel?level={}",
        api_level
    );

    let client = Client::new();
    let response = client
        .get(url)
        .header("Authentication", token)
        .header("Referer", "https://m.swranking.com/")
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .map_err(|_| "Failed download TL".to_string())?;

    let json = response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| "Failed to parse JSON".to_string())?;

    let data = json.get("data").ok_or("Missing data field")?;

    let date_str = data
        .get("createDate")
        .and_then(|v| v.as_str())
        .ok_or("Missing createDate field")?;

    // Parse and format the date
    let formatted_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map(|date| date.format("%d-%m-%Y").to_string())
        .unwrap_or_else(|_| date_str.to_string()); // Fallback to original if parsing fails

    let tierlist_data = TierListData {
        level: data.get("level").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
        sss_monster: serde_json::from_value(data.get("sssMonster").cloned().unwrap_or_default())
            .unwrap_or_default(),
        ss_monster: serde_json::from_value(data.get("ssMonster").cloned().unwrap_or_default())
            .unwrap_or_default(),
        s_monster: serde_json::from_value(data.get("smonster").cloned().unwrap_or_default())
            .unwrap_or_default(),
        a_monster: serde_json::from_value(data.get("amonster").cloned().unwrap_or_default())
            .unwrap_or_default(),
        b_monster: serde_json::from_value(data.get("bmonster").cloned().unwrap_or_default())
            .unwrap_or_default(),
        c_monster: serde_json::from_value(data.get("cmonster").cloned().unwrap_or_default())
            .unwrap_or_default(),
        date: Some(formatted_date),
    };

    Ok(tierlist_data)
}

/// Récupère le dernier patch d'une saison Lucksack (celui avec le patch_order maximal).
pub async fn get_latest_patch(season: i32) -> Result<i32, String> {
    let url = format!("https://api.lucksack.gg/seasons/{}/patches", season);

    let res = http_client()
        .get(&url)
        .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .header("sec-fetch-site", "none")
        .send()
        .await
        .map_err(|_| "Failed to send request".to_string())?;

    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }

    let patches = res
        .json::<Vec<LucksackPatch>>()
        .await
        .map_err(|_| "Failed to parse patches JSON".to_string())?;

    patches
        .into_iter()
        .max_by_key(|p| p.patch_order)
        .map(|p| p.patch_id)
        .ok_or_else(|| "No patch found".to_string())
}

/// Récupère les trios globaux d'un rank pour une saison/patch, filtrés localement
/// par min_games (statistics/trio ne supporte pas min_appearances).
pub async fn fetch_global_trios(
    season: i32,
    patch: i32,
    rank: i32,
    min_games: u32,
) -> Result<Vec<TrioStat>, String> {
    let mut result: Vec<TrioStat> = Vec::new();

    for page in 0..MAX_TRIO_PAGES {
        let offset = page * 100;
        let url = format!(
            "https://api.lucksack.gg/statistics/trio?season={}&rank={}&patch={}&limit=100&offset={}&order_by=Pick&order_direction=Desc",
            season, rank, patch, offset
        );

        let res = http_client()
            .get(&url)
            .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")
            .header("sec-fetch-site", "none")
            .send()
            .await
            .map_err(|_| "Failed to send request".to_string())?;

        if !res.status().is_success() {
            return Err(format!("HTTP {}", res.status()));
        }

        let body = res
            .json::<LucksackTrioResponse>()
            .await
            .map_err(|_| "Failed to parse trio JSON".to_string())?;

        let page_len = body.records.len();

        for record in body.records {
            let LucksackTrioRecord {
                monster_id,
                played_count,
                win_rate,
            } = record;

            if monster_id.len() != 3 {
                continue;
            }

            if played_count < min_games {
                continue;
            }

            result.push(TrioStat {
                ids: [monster_id[0], monster_id[1], monster_id[2]],
                count: played_count,
                win_rate,
            });
        }

        if page_len < 100 {
            break;
        }
    }

    Ok(result)
}

/// Récupère les trios contenant un monstre donné. min_games est appliqué côté serveur
/// via min_appearances.
pub async fn fetch_monster_trios(
    season: i32,
    patch: i32,
    rank: i32,
    monster_id: u32,
    min_games: u32,
) -> Result<Vec<TrioStat>, String> {
    let mut result: Vec<TrioStat> = Vec::new();

    for page in 0..MAX_TRIO_PAGES {
        let offset = page * 100;
        let url = format!(
            "https://api.lucksack.gg/monsters/{}/with-trio?season={}&rank={}&patch={}&limit=100&offset={}&order_by=appearances&order_direction=desc&min_appearances={}",
            monster_id, season, rank, patch, offset, min_games
        );

        let res = http_client()
            .get(&url)
            .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")
            .header("sec-fetch-site", "none")
            .send()
            .await
            .map_err(|_| "Failed to send request".to_string())?;

        if !res.status().is_success() {
            return Err(format!("HTTP {}", res.status()));
        }

        let body = res
            .json::<LucksackWithTrioResponse>()
            .await
            .map_err(|_| "Failed to parse with-trio JSON".to_string())?;

        let page_len = body.records.len();

        for record in body.records {
            result.push(TrioStat {
                ids: [
                    record.units1.monster_id,
                    record.units2.monster_id,
                    record.units3.monster_id,
                ],
                count: record.appearances,
                win_rate: record.winrate,
            });
        }

        if page_len < 100 {
            break;
        }
    }

    Ok(result)
}

pub async fn get_emoji_from_id(
    collection: &Collection<mongodb::bson::Document>,
    monster_id: u32,
) -> Option<String> {
    // println!("Searching for emoji with monster_id: {}", monster_id);

    let emoji_doc = collection
        .find_one(doc! { "com2us_id": monster_id })
        .await
        .ok()??;

    // println!("Found emoji document: {:?}", emoji_doc);

    let emoji_id = emoji_doc.get_str("id").ok()?;
    let emoji_name = emoji_doc.get_str("name").ok()?;

    // println!("Extracted emoji_id: {}, emoji_name: {}", emoji_id, emoji_name);

    Some(format!("<:{}:{}>", emoji_name, emoji_id))
}
