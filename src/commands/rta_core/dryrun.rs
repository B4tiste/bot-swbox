//! Harnais console pour essayer /get_rta_core en local, sans Discord ni Mongo.
//!
//! Lancement :
//!   RTA_CORE_TEST_JSON=/chemin/box.json cargo test rta_core_dryrun -- --nocapture
//!
//! Variables optionnelles :
//!   RTA_CORE_TEST_RANK    = P1 | P2-P3 | G1-G3 | G3   (défaut : G1-G3)
//!   RTA_CORE_TEST_MONSTER = nom exact d'un monstre     (active l'endpoint with-trio)
//!   RTA_CORE_TEST_MIN     = seuil minimum de games     (défaut : 100)
//!
//! Sans RTA_CORE_TEST_JSON, le test est ignoré (aucun appel réseau).
//! Réutilise le vrai code (parsing box, fetch Lucksack, filtre box, tri) ; les emojis
//! Discord sont remplacés par les noms de monstres pour l'affichage console.

use crate::commands::how_to_build::utils::get_latest_lucksack_season;
use crate::commands::rta_core::cache::get_trios_cached;
use crate::commands::rta_core::models::{Companion, TrioStat};
use crate::commands::rta_core::utils::{
    derive_companions, get_latest_patch, get_monsters_from_json_bytes,
};
use crate::MONSTER_MAP;
use std::collections::{HashMap, HashSet};

fn rank_value(label: &str) -> i32 {
    match label {
        "P1" => 11,
        "P2-P3" | "P2P3" => 103,
        "G3" => 16,
        _ => 102, // G1-G3 par défaut
    }
}

fn print_section(title: &str, trios: &[TrioStat], id_to_name: &HashMap<u32, String>) {
    println!("\n== {title} ==");
    if trios.is_empty() {
        println!("  (aucun trio)");
        return;
    }
    let name = |id: u32| {
        id_to_name
            .get(&id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    };
    for t in trios {
        println!(
            "  {} / {} / {}  ->  {:.1}% WR ({} games)",
            name(t.ids[0]),
            name(t.ids[1]),
            name(t.ids[2]),
            t.win_rate * 100.0,
            t.count
        );
    }
}

#[tokio::test]
async fn rta_core_dryrun() {
    let json_path = match std::env::var("RTA_CORE_TEST_JSON") {
        Ok(p) => p,
        Err(_) => {
            println!("RTA_CORE_TEST_JSON non défini : dry-run ignoré.");
            return;
        }
    };

    let rank_label = std::env::var("RTA_CORE_TEST_RANK").unwrap_or_else(|_| "G1-G3".to_string());
    let rank = rank_value(&rank_label);
    let min_games: u32 = std::env::var("RTA_CORE_TEST_MIN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);
    let monster_name = std::env::var("RTA_CORE_TEST_MONSTER").ok();
    let exclude_name = std::env::var("RTA_CORE_TEST_EXCLUDE").ok();

    let bytes = std::fs::read(&json_path).expect("lecture du fichier RTA_CORE_TEST_JSON");

    let monsters = get_monsters_from_json_bytes(&bytes, "monsters_elements.json")
        .expect("parsing de la box / monsters_elements.json");
    let player_box_ids: HashSet<u32> = monsters.iter().map(|m| m.unit_master_id).collect();

    let id_to_name: HashMap<u32, String> = MONSTER_MAP
        .iter()
        .map(|(name, id)| (*id, name.clone()))
        .collect();

    let season = get_latest_lucksack_season()
        .await
        .expect("récupération de la saison");
    let patch = get_latest_patch(season)
        .await
        .expect("récupération du patch");

    let filter_id: Option<u32> = match &monster_name {
        Some(n) => match MONSTER_MAP.get(n) {
            Some(id) => Some(*id),
            None => {
                println!("Monstre '{n}' introuvable dans MONSTER_MAP (utilise le nom exact).");
                return;
            }
        },
        None => None,
    };

    let exclude_id: Option<u32> = match &exclude_name {
        Some(n) => match MONSTER_MAP.get(n) {
            Some(id) => Some(*id),
            None => {
                println!(
                    "Monstre exclu '{n}' introuvable dans MONSTER_MAP (utilise le nom exact)."
                );
                return;
            }
        },
        None => None,
    };

    println!(
        "box={} monstres | season={season} patch={patch} rank={rank_label}({rank}) min_games={min_games}{}{}",
        player_box_ids.len(),
        monster_name
            .as_ref()
            .map(|n| format!(" | focus={n}"))
            .unwrap_or_default(),
        exclude_name
            .as_ref()
            .map(|n| format!(" | exclude={n}"))
            .unwrap_or_default()
    );

    let pool: Vec<TrioStat> = get_trios_cached(season, patch, rank, filter_id, min_games)
        .await
        .expect("récupération des trios Lucksack");
    println!("{} trios récupérés (avant filtre box)", pool.len());

    let playable: Vec<TrioStat> = pool
        .iter()
        .filter(|t| t.ids.iter().all(|id| player_box_ids.contains(id)))
        .filter(|t| exclude_id.is_none_or(|ex| !t.ids.contains(&ex)))
        .cloned()
        .collect();
    println!("{} trios jouables (box 3/3)", playable.len());

    let mut by_count = playable.clone();
    by_count.sort_by_key(|t| std::cmp::Reverse(t.count));
    let most_played: Vec<TrioStat> = by_count.into_iter().take(5).collect();
    print_section("Most Played", &most_played, &id_to_name);

    let mut by_wr = playable;
    by_wr.sort_by(|a, b| {
        b.win_rate
            .partial_cmp(&a.win_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let best_wr: Vec<TrioStat> = by_wr.into_iter().take(5).collect();
    print_section("Best Winrate", &best_wr, &id_to_name);

    // Compagnons (4e/5e picks) dérivés du pool brut pour le trio le plus joué.
    if let Some(top) = most_played.first() {
        let companions = derive_companions(&pool, top.ids, exclude_id, 4);
        print_companions(top, &companions, &player_box_ids, &id_to_name);
    }
}

fn print_companions(
    trio: &TrioStat,
    companions: &[Companion],
    box_ids: &HashSet<u32>,
    id_to_name: &HashMap<u32, String>,
) {
    let name = |id: u32| {
        id_to_name
            .get(&id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    };
    println!(
        "\n== Companions for {} / {} / {} ==",
        name(trio.ids[0]),
        name(trio.ids[1]),
        name(trio.ids[2])
    );
    if companions.is_empty() {
        println!("  (aucun compagnon fiable)");
        return;
    }
    let max_count = companions.iter().map(|c| c.count).max().unwrap_or(0);
    for c in companions {
        let badge = if box_ids.contains(&c.id) {
            "OWNED"
        } else {
            "MISSING"
        };
        let ratio = if max_count > 0 {
            c.count as f32 / max_count as f32
        } else {
            0.0
        };
        println!(
            "  {} [{}]  pop {:.0}% · {:.0}% WR ({} co-occ.)",
            name(c.id),
            badge,
            ratio * 100.0,
            c.win_rate * 100.0,
            c.count
        );
    }
}
