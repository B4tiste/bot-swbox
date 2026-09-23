// src/commands/rta_core/cache.rs
//! Cache asynchrone pour les appels Lucksack (trios globaux ou par monstre)
use crate::commands::rta_core::models::TrioStat;
use crate::commands::rta_core::utils::{fetch_global_trios, fetch_monster_trios};
use moka::future::Cache;
use once_cell::sync::Lazy;
use std::time::Duration;

/// Clé : (season, patch, rank, monster_id_or_0, min_games)
type TrioKey = (i32, i32, i32, u32, u32);

/// On stocke un Result<Vec<TrioStat>, String> pour propager l’erreur
static TRIO_CACHE: Lazy<Cache<TrioKey, Result<Vec<TrioStat>, String>>> = Lazy::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(6 * 3600))
        .max_capacity(1_000)
        .build()
});

/// Wrapper : si absent ou expiré, charge les trios (globaux ou par monstre) et stocke le résultat
pub async fn get_trios_cached(
    season: i32,
    patch: i32,
    rank: i32,
    monster_id: Option<u32>,
    min_games: u32,
) -> Result<Vec<TrioStat>, String> {
    let key = (season, patch, rank, monster_id.unwrap_or(0), min_games);

    // Loader qui renvoie un Result<…, String>
    let loader = async move {
        if let Some(mid) = monster_id {
            fetch_monster_trios(season, patch, rank, mid, min_games).await
        } else {
            fetch_global_trios(season, patch, rank, min_games).await
        }
    };

    // get_with charge si absent/expiré, et renvoie directement le Result
    TRIO_CACHE.get_with(key, loader).await
}
