// src/commands/rta_core/cache.rs
//! Cache asynchrone pour les appels highdata (get_monster_duos)
use crate::commands::rta_core::models::MonsterDuoStat;
use crate::commands::rta_core::utils::get_monster_duos;
use moka::future::Cache;
use once_cell::sync::Lazy;
use std::time::Duration;

/// Clé : (monster_id, season, version, level)
type DuoKey = (u32, i64, String, i32);

/// On stocke un Result<Vec<MonsterDuoStat>, String> pour propager l’erreur
static DUO_CACHE: Lazy<Cache<DuoKey, Result<Vec<MonsterDuoStat>, String>>> = Lazy::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(96 * 3600))
        .max_capacity(10_000)
        .build()
});

/// Wrapper : si absent ou expiré, appelle get_monster_duos et stocke le résultat
pub async fn get_monster_duos_cached(
    token: &str,
    season: i64,
    version: &str,
    monster_id: u32,
    level: i32,
) -> Result<Vec<MonsterDuoStat>, String> {
    let key = (monster_id, season, version.to_string(), level);

    // Loader qui renvoie un Result<…, String>
    let loader = {
        let token = token.to_string();
        async move {
            get_monster_duos(&token, season, &version, monster_id, level)
                .await
                .map_err(|e| format!("Erreur highdata: {}", e))
        }
    };

    // get_with charge si absent/expiré, et renvoie directement le Result
    DUO_CACHE.get_with(key, loader).await
}
