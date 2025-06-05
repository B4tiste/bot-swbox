use crate::commands::rta_core::models::{Monster, MonsterEntry, MonstersFile, TierListData, MonsterDuoStat};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use reqwest::Client;
use mongodb::{bson::doc, Collection};
use crate::commands::mob_stats::utils::remap_monster_id;

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
        .filter_map(|u| u.get("unit_master_id")?.as_u64().map(|id| remap_monster_id(id as i32) as u32))
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
            unit_master_id: m.com2us_id
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

    // I want to return the data field of the json, but only the field listed above
    let data = json.get("data").ok_or("Missing data field")?;

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
    };

    Ok(tierlist_data)
}

/// Récupère les duos (highOneWithTwoList) pour un monstre donné
pub async fn get_monster_duos(
    token: &str,
    season: i64,
    monster_id: u32,
    level: i32,
) -> Result<Vec<MonsterDuoStat>> {
    let url = format!(
        "https://m.swranking.com/api/monster/highdata?pageNum=1&pageSize=20&monsterId={}&season={}&version=&level={}&factor=0.01&real=0",
        monster_id, season, level
    );
    let client = Client::new();
    let resp = client
        .get(&url)
        .header("Authentication", token)
        .header("Referer", "https://m.swranking.com/")
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await?
        .json::<Value>()
        .await?;
    let list = resp["data"]["highOneWithTwoList"]
        .as_array()
        .context("Missing highOneWithTwoList")?;
    let duos = serde_json::from_value(Value::Array(list.clone()))?;
    Ok(duos)
}

pub fn filter_monster(tierlist_data: &TierListData, monsters: &[Monster]) -> TierListData {
    let mut filtered_tierlist = tierlist_data.clone();
    filtered_tierlist.sss_monster.retain(|m| {
        monsters.iter().any(|monster| monster.unit_master_id == m.monster_id)
    });
    filtered_tierlist.ss_monster.retain(|m| {
        monsters.iter().any(|monster| monster.unit_master_id == m.monster_id)
    });
    filtered_tierlist.s_monster.retain(|m| {
        monsters.iter().any(|monster| monster.unit_master_id == m.monster_id)
    });
    filtered_tierlist.a_monster.retain(|m| {
        monsters.iter().any(|monster| monster.unit_master_id == m.monster_id)
    });
    filtered_tierlist.b_monster.retain(|m| {
        monsters.iter().any(|monster| monster.unit_master_id == m.monster_id)
    });
    filtered_tierlist.c_monster.retain(|m| {
        monsters.iter().any(|monster| monster.unit_master_id == m.monster_id)
    });

    filtered_tierlist
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