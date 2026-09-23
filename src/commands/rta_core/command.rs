use crate::commands::how_to_build::utils::get_latest_lucksack_season;
use crate::commands::mob_stats::command::autocomplete_monster;
use crate::commands::player_stats::utils::get_mob_emoji_collection;
use crate::commands::rta_core::cache::get_trios_cached;
use crate::commands::rta_core::models::{Rank, Sort, TrioStat};
use crate::commands::rta_core::utils::{
    get_emoji_from_id, get_latest_patch, get_monsters_from_json_bytes,
};
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::{get_server_name, send_log};
use crate::commands::shared::models::LoggerDocument;
use crate::Data;
use crate::MONSTER_MAP;
use poise::serenity_prelude::{Attachment, Error};
use std::collections::HashSet;

/// Envoie un embed d'erreur, planifie sa suppression et logge l'échec.
async fn fail(ctx: poise::ApplicationContext<'_, Data, Error>, message: &str) -> Result<(), Error> {
    let reply = ctx.send(create_embed_error(message)).await?;
    schedule_message_deletion(reply, ctx).await?;
    send_log(LoggerDocument::new(
        &ctx.author().name,
        "get_rta_core",
        &get_server_name(&ctx).await?,
        false,
        chrono::Utc::now().timestamp(),
    ))
    .await?;
    Ok(())
}

/// 📂 Displays best Trios to play for any given account JSON
#[poise::command(slash_command)]
pub async fn get_rta_core(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    file: Attachment,
    #[description = "Select the targeted rank"] rank: Rank,
    #[autocomplete = "autocomplete_monster"]
    #[description = "Monster to focus the cores on (optional)"]
    monster: Option<String>,
    #[autocomplete = "autocomplete_monster"]
    #[description = "Monster to exclude from the trios (optional)"]
    exclude: Option<String>,
    #[description = "Sort order (optional; default: 5 most played + 5 best winrate)"] sort: Option<
        Sort,
    >,
    #[description = "Minimum games per trio for reliability (optional, default 50)"]
    min_games: Option<u32>,
) -> Result<(), Error> {
    // Évite le timeout de 3 s
    ctx.defer().await?;

    // Vérification présence de fichier
    if file.url.is_empty() {
        return fail(ctx, "No file provided. Please attach a JSON file.").await;
    }

    // Vérification extension
    if !file.filename.to_lowercase().ends_with(".json") {
        return fail(ctx, "The provided file is not a JSON file.").await;
    }

    // Téléchargement
    let bytes = match file.download().await {
        Ok(b) => b,
        Err(e) => {
            return fail(ctx, &format!("Failed to download the file: {}", e)).await;
        }
    };

    // Extraction des monstres de la box
    let monsters = match get_monsters_from_json_bytes(&bytes, "monsters_elements.json") {
        Ok(m) => m,
        Err(e) => {
            return fail(ctx, &format!("Error: {}", e)).await;
        }
    };
    let player_box_ids: HashSet<u32> = monsters.iter().map(|m| m.unit_master_id).collect();

    // Conversion du nom de monstre optionnel en ID
    let filter_monster_id: Option<u32> = if let Some(ref name) = monster {
        match MONSTER_MAP.get(name) {
            Some(&id) => Some(id),
            None => {
                return fail(
                    ctx,
                    &format!(
                        "Cannot find '{}', please use the autocomplete feature for a perfect match.",
                        name
                    ),
                )
                .await;
            }
        }
    } else {
        None
    };

    // Conversion du nom de monstre à exclure optionnel en ID
    let exclude_id: Option<u32> = if let Some(ref name) = exclude {
        match MONSTER_MAP.get(name) {
            Some(&id) => Some(id),
            None => {
                return fail(
                    ctx,
                    &format!(
                        "Cannot find '{}', please use the autocomplete feature for a perfect match.",
                        name
                    ),
                )
                .await;
            }
        }
    } else {
        None
    };

    let min_games_val = min_games.unwrap_or(50);

    let season = match get_latest_lucksack_season().await {
        Ok(s) => s,
        Err(e) => {
            return fail(ctx, &e).await;
        }
    };

    let patch = match get_latest_patch(season).await {
        Ok(p) => p,
        Err(e) => {
            return fail(ctx, &e).await;
        }
    };

    let rank_val = rank.lucksack_rank();

    let collection = match get_mob_emoji_collection().await {
        Ok(c) => c,
        Err(_) => {
            return fail(ctx, "Database error while fetching emojis.").await;
        }
    };

    let pool: Vec<TrioStat> =
        match get_trios_cached(season, patch, rank_val, filter_monster_id, min_games_val).await {
            Ok(p) => p,
            Err(e) => {
                return fail(ctx, &e).await;
            }
        };

    // Filtre Box 3/3 : le trio entier doit être dans la box du joueur.
    let playable: Vec<TrioStat> = pool
        .into_iter()
        .filter(|t| t.ids.iter().all(|id| player_box_ids.contains(id)))
        .filter(|t| exclude_id.is_none_or(|ex| !t.ids.contains(&ex)))
        .collect();

    // Récupération du pseudo dans le fichier JSON uploadé (field wizard_info.wizard_name)
    let json_str = String::from_utf8_lossy(&bytes).to_string();
    let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap_or_default();
    let wizard_name = json_value
        .get("wizard_info")
        .and_then(|w| w.get("wizard_name"))
        .and_then(|w| w.as_str())
        .unwrap_or("Unknown");
    let wizard_name = if wizard_name.is_empty() {
        "Unknown"
    } else {
        wizard_name
    };

    // En-tête
    let mut msg = format!(
        "🎯 Trios for `{}` to play in `{}`",
        wizard_name,
        rank.label()
    );
    if let Some(ref name) = monster {
        msg.push_str(&format!(" focusing on `{}`", name));
    }
    if let Some(ref name) = exclude {
        msg.push_str(&format!(" excluding `{}`", name));
    }
    msg.push_str(":\n");

    // Sections à afficher : (titre, clé de tri, nombre)
    enum SortKey {
        Count,
        WinRate,
    }
    let sections: Vec<(&str, SortKey, usize)> = match sort {
        None => vec![
            ("Most Played", SortKey::Count, 5),
            ("Best Winrate", SortKey::WinRate, 5),
        ],
        Some(Sort::MostPlayed) => vec![("Most Played", SortKey::Count, 10)],
        Some(Sort::BestWinrate) => vec![("Best Winrate", SortKey::WinRate, 10)],
    };

    for (title, key, count) in sections {
        let title_line = match key {
            SortKey::Count => format!("\n**🔥 {}**\n", title),
            SortKey::WinRate => format!("\n**🏆 {}**\n", title),
        };
        msg.push_str(&title_line);

        let mut sorted = playable.clone();
        match key {
            SortKey::Count => sorted.sort_by_key(|t| std::cmp::Reverse(t.count)),
            SortKey::WinRate => sorted.sort_by(|a, b| {
                b.win_rate
                    .partial_cmp(&a.win_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }
        let top: Vec<TrioStat> = sorted.into_iter().take(count).collect();

        if top.is_empty() {
            msg.push_str("- No Trio Found\n");
        } else {
            for t in &top {
                let emojis = format!(
                    "{} {} {}",
                    get_emoji_from_id(&collection, t.ids[0])
                        .await
                        .unwrap_or_default(),
                    get_emoji_from_id(&collection, t.ids[1])
                        .await
                        .unwrap_or_default(),
                    get_emoji_from_id(&collection, t.ids[2])
                        .await
                        .unwrap_or_default()
                );
                msg.push_str(&format!(
                    "- {} → **{:.1} %** WR ({})\n",
                    emojis,
                    t.win_rate * 100.0,
                    t.count,
                ));
            }
        }
    }

    send_log(LoggerDocument::new(
        &ctx.author().name,
        "get_rta_core",
        &get_server_name(&ctx).await?,
        true,
        chrono::Utc::now().timestamp(),
    ))
    .await?;
    ctx.say(msg).await?;

    Ok(())
}
