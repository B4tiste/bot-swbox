use crate::commands::how_to_build::utils::{
    create_lucksack_rank_buttons, get_latest_lucksack_season,
};
use crate::commands::mob_stats::command::autocomplete_monster;
use crate::commands::player_stats::utils::get_mob_emoji_collection;
use crate::commands::rta_core::cache::get_trios_cached;
use crate::commands::rta_core::models::{Rank, Sort, TrioStat};
use crate::commands::rta_core::utils::{
    build_rta_core_embed, build_sections, build_trio_select_menu, derive_companions,
    get_latest_patch, get_monsters_from_json_bytes, load_all_mob_emojis,
};
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::{get_server_name, send_log};
use crate::commands::shared::models::LoggerDocument;
use crate::Data;
use crate::MONSTER_MAP;
use poise::serenity_prelude::builder::EditInteractionResponse;
use poise::serenity_prelude::{
    self as serenity, Attachment, ComponentInteractionDataKind, CreateInteractionResponse,
    CreateInteractionResponseMessage, Error,
};
use poise::CreateReply;
use std::collections::{HashMap, HashSet};

/// Nombre maximal de compagnons (4e/5e picks) affichés pour un trio.
const MAX_COMPANIONS: usize = 4;

/// Libellé d'un rang Lucksack pour l'affichage (le bouton courant peut différer du param initial).
fn rank_label_for(rank: i32) -> &'static str {
    match rank {
        11 => "P1",
        103 => "P2-P3",
        102 => "G1-G3",
        16 => "G3",
        _ => "?",
    }
}

/// Filtre Box 3/3 : le trio entier doit être dans la box, hors monstre exclu.
fn compute_playable(
    pool: &[TrioStat],
    box_ids: &HashSet<u32>,
    exclude_id: Option<u32>,
) -> Vec<TrioStat> {
    pool.iter()
        .filter(|t| t.ids.iter().all(|id| box_ids.contains(id)))
        .filter(|t| exclude_id.is_none_or(|ex| !t.ids.contains(&ex)))
        .cloned()
        .collect()
}

/// Trios affichés (union dédupliquée des sections), pour alimenter le menu de sélection.
fn flatten_sections(sections: &[(&'static str, &'static str, Vec<TrioStat>)]) -> Vec<TrioStat> {
    sections
        .iter()
        .flat_map(|(_, _, v)| v.iter().cloned())
        .collect()
}

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

    let mut current_rank = rank.lucksack_rank();

    // Collection emojis chargée une seule fois (re-rendus interactifs rapides).
    let collection = match get_mob_emoji_collection().await {
        Ok(c) => c,
        Err(_) => {
            return fail(ctx, "Database error while fetching emojis.").await;
        }
    };
    let emoji_map = load_all_mob_emojis(&collection).await;

    // id -> nom (depuis MONSTER_MAP) pour les libellés du menu et les fallbacks.
    let id_to_name: HashMap<u32, String> = MONSTER_MAP
        .iter()
        .map(|(name, id)| (*id, name.clone()))
        .collect();

    // Pseudo depuis le fichier JSON uploadé (field wizard_info.wizard_name)
    let json_str = String::from_utf8_lossy(&bytes).to_string();
    let json_value: serde_json::Value = serde_json::from_str(&json_str).unwrap_or_default();
    let wizard_name = json_value
        .get("wizard_info")
        .and_then(|w| w.get("wizard_name"))
        .and_then(|w| w.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Unknown")
        .to_string();

    // Récupération initiale du pool + dérivation de la vue.
    let mut pool = match get_trios_cached(
        season,
        patch,
        current_rank,
        filter_monster_id,
        min_games_val,
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            return fail(ctx, &e).await;
        }
    };
    let mut playable = compute_playable(&pool, &player_box_ids, exclude_id);
    let mut sections = build_sections(&playable, sort.as_ref());
    let mut displayed = flatten_sections(&sections);

    let embed = build_rta_core_embed(
        &wizard_name,
        rank_label_for(current_rank),
        monster.as_deref(),
        exclude.as_deref(),
        playable.len(),
        &sections,
        &emoji_map,
        &id_to_name,
        &player_box_ids,
        None,
    );
    let mut components = vec![create_lucksack_rank_buttons(current_rank, false)];
    if let Some(menu) = build_trio_select_menu(&displayed, &emoji_map, &id_to_name) {
        components.push(menu);
    }

    let reply = ctx
        .send(CreateReply {
            embeds: vec![embed],
            components: Some(components),
            ..Default::default()
        })
        .await?;

    let message_id = reply.message().await?.id;
    let channel_id = ctx.channel_id();
    let user_id = ctx.author().id;

    while let Some(interaction) =
        serenity::ComponentInteractionCollector::new(&ctx.serenity_context.shard)
            .channel_id(channel_id)
            .message_id(message_id)
            .filter(move |i| i.user.id == user_id)
            .timeout(std::time::Duration::from_secs(600))
            .await
    {
        match interaction.data.custom_id.as_str() {
            "rank_p1" | "rank_p2p3" | "rank_g1g2g3" | "rank_g3" => {
                let new_rank = match interaction.data.custom_id.as_str() {
                    "rank_g3" => 16,
                    "rank_g1g2g3" => 102,
                    "rank_p2p3" => 103,
                    "rank_p1" => 11,
                    _ => current_rank,
                };

                if new_rank == current_rank {
                    interaction
                        .create_response(
                            &ctx.serenity_context,
                            CreateInteractionResponse::Acknowledge,
                        )
                        .await?;
                    continue;
                }
                current_rank = new_rank;

                interaction
                    .create_response(
                        &ctx.serenity_context,
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .content("🔄 Loading...")
                                .components(vec![create_lucksack_rank_buttons(current_rank, true)]),
                        ),
                    )
                    .await?;

                pool = match get_trios_cached(
                    season,
                    patch,
                    current_rank,
                    filter_monster_id,
                    min_games_val,
                )
                .await
                {
                    Ok(p) => p,
                    Err(e) => {
                        interaction
                            .edit_response(
                                &ctx.serenity_context.http,
                                EditInteractionResponse::new()
                                    .content(format!("❌ {}", e))
                                    .embeds(vec![])
                                    .components(vec![create_lucksack_rank_buttons(
                                        current_rank,
                                        false,
                                    )]),
                            )
                            .await?;
                        continue;
                    }
                };
                playable = compute_playable(&pool, &player_box_ids, exclude_id);
                sections = build_sections(&playable, sort.as_ref());
                displayed = flatten_sections(&sections);

                let embed = build_rta_core_embed(
                    &wizard_name,
                    rank_label_for(current_rank),
                    monster.as_deref(),
                    exclude.as_deref(),
                    playable.len(),
                    &sections,
                    &emoji_map,
                    &id_to_name,
                    &player_box_ids,
                    None,
                );
                let mut components = vec![create_lucksack_rank_buttons(current_rank, false)];
                if let Some(menu) = build_trio_select_menu(&displayed, &emoji_map, &id_to_name) {
                    components.push(menu);
                }
                interaction
                    .edit_response(
                        &ctx.serenity_context.http,
                        EditInteractionResponse::new()
                            .content("")
                            .embeds(vec![embed])
                            .components(components),
                    )
                    .await?;
            }
            "rta_core_trio_select" => {
                let value = match &interaction.data.kind {
                    ComponentInteractionDataKind::StringSelect { values } => {
                        values.first().cloned()
                    }
                    _ => None,
                };
                let ids: Vec<u32> = value
                    .as_deref()
                    .map(|v| v.split('_').filter_map(|s| s.parse().ok()).collect())
                    .unwrap_or_default();
                if ids.len() != 3 {
                    interaction
                        .create_response(
                            &ctx.serenity_context,
                            CreateInteractionResponse::Acknowledge,
                        )
                        .await?;
                    continue;
                }
                let trio = [ids[0], ids[1], ids[2]];
                let companions = derive_companions(&pool, trio, exclude_id, MAX_COMPANIONS);
                let selected = (trio, companions);

                let embed = build_rta_core_embed(
                    &wizard_name,
                    rank_label_for(current_rank),
                    monster.as_deref(),
                    exclude.as_deref(),
                    playable.len(),
                    &sections,
                    &emoji_map,
                    &id_to_name,
                    &player_box_ids,
                    Some(&selected),
                );
                let mut components = vec![create_lucksack_rank_buttons(current_rank, false)];
                if let Some(menu) = build_trio_select_menu(&displayed, &emoji_map, &id_to_name) {
                    components.push(menu);
                }
                interaction
                    .create_response(
                        &ctx.serenity_context,
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::new()
                                .embeds(vec![embed])
                                .components(components),
                        ),
                    )
                    .await?;
            }
            _ => continue,
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

    Ok(())
}
