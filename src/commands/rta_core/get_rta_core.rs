use crate::commands::mob_stats::get_mob_stats::autocomplete_monster;
use crate::commands::mob_stats::utils::get_swrt_settings;
use crate::commands::player_stats::utils::get_mob_emoji_collection;
use crate::commands::rta_core::cache::get_monster_duos_cached;
use crate::commands::rta_core::models::MonstersFile;
use crate::commands::rta_core::models::{Mode, Rank, Trio};
use crate::commands::rta_core::utils::{
    filter_monster, get_emoji_from_id, get_monsters_from_json_bytes, get_tierlist_data, get_swrt_version,
};
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::send_log;
use crate::{Data, API_TOKEN};
use poise::serenity_prelude::{Attachment, Error};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;

// Import de la map globale
use crate::MONSTER_MAP;

/// 📂 Displays best Trios to play for any given account JSON
#[poise::command(slash_command)]
pub async fn get_rta_core(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    file: Attachment,
    #[description = "Select the targeted rank"] rank: Rank,
    #[autocomplete = "autocomplete_monster"]
    #[description = "Monster you want to get core for (optional)"]
    monster: Option<String>,
    #[description = "Mode of the core (Meta Slayer or Fun/Casual)"] mode: Mode,
) -> Result<(), Error> {
    // Évite le timeout de 3 s
    ctx.defer().await?;

    let token = {
        let guard = API_TOKEN.lock().unwrap();
        guard.clone().ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing API token",
            ))
        })?
    };

    // Vérification présence de fichier
    if file.url.is_empty() {
        let err = "No file provided. Please attach a JSON file.";
        let reply = ctx.send(create_embed_error(err)).await?;
        schedule_message_deletion(reply, ctx).await?;
        send_log(&ctx, "Command: /get_rta_core", false, err).await?;
        return Ok(());
    }

    // Vérification extension
    if !file.filename.to_lowercase().ends_with(".json") {
        let err = "The provided file is not a JSON file.";
        let reply = ctx.send(create_embed_error(err)).await?;
        schedule_message_deletion(reply, ctx).await?;
        send_log(&ctx, "Command: /get_rta_core", false, err).await?;
        return Ok(());
    }

    // Téléchargement
    let bytes = match file.download().await {
        Ok(b) => b,
        Err(e) => {
            let err_msg = format!("Impossible de télécharger : {}", e);
            let reply = ctx.send(create_embed_error(&err_msg)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(&ctx, "Command: /get_rta_core", false, &err_msg).await?;
            return Ok(());
        }
    };

    // 1️⃣ Charger le JSON statique "monsters_elements.json" pour connaître l'élément de chaque monstre
    let monsters_json_str = fs::read_to_string("monsters_elements.json")
        .expect("Impossible de lire monsters_elements.json");
    let all_monsters_file: MonstersFile = serde_json::from_str(&monsters_json_str)
        .expect("Impossible de parser monsters_elements.json");

    // 2️⃣ Construire la table id → élément
    let element_map: HashMap<u32, String> = all_monsters_file
        .monsters
        .into_iter()
        .map(|m| (m.com2us_id, m.element))
        .collect();

    // Extraction des monsters
    match get_monsters_from_json_bytes(&bytes, "monsters_elements.json") {
        Ok(monsters) => {
            // Si l'utilisateur a passé un nom de monstre, on le convertit en ID
            let filter_monster_id: Option<u32> = if let Some(ref name) = monster {
                match MONSTER_MAP.get(name) {
                    Some(&id) => Some(id),
                    None => {
                        let msg = format!("❌ Cannot find '{}', please use the autocomplete feature for a perfect match.", name);
                        let reply = ctx.send(create_embed_error(&msg)).await?;
                        schedule_message_deletion(reply, ctx).await?;
                        send_log(&ctx, "get_rta_core", false, &msg).await?;
                        return Ok(());
                    }
                }
            } else {
                None
            };

            // 1) Déterminer le paramètre `level` SWRanking selon le Rank choisi
            let api_level = match rank {
                Rank::C1 | Rank::C2 | Rank::C3 | Rank::P1 | Rank::P2 => 0,
                Rank::P3 | Rank::G1 | Rank::G2 => 1,
                Rank::G3 => 3,
            };

            let tierlist_data = match get_tierlist_data(api_level, &token).await {
                Ok(data) => data,
                Err(e) => {
                    let err_msg = format!("Impossible de récupérer les données : {}", e);
                    let reply = ctx.send(create_embed_error(&err_msg)).await?;
                    schedule_message_deletion(reply, ctx).await?;
                    send_log(&ctx, "Command: /get_rta_core", false, &err_msg).await?;
                    return Ok(());
                }
            };

            let mut late_pick_ratio_map = HashMap::new();
            for m in tierlist_data
                .sss_monster
                .iter()
                .chain(&tierlist_data.ss_monster)
                .chain(&tierlist_data.s_monster)
                .chain(&tierlist_data.a_monster)
                .chain(&tierlist_data.b_monster)
                .chain(&tierlist_data.c_monster)
            {
                let total = m.pick_total as f32;
                let late_sum = (m.third_pick_total
                    + m.fourth_pick_total
                    + m.fifth_pick_total
                    + m.last_pick_total) as f32;
                let ratio = if total > 0.0 { late_sum / total } else { 0.0 };
                late_pick_ratio_map.insert(m.monster_id, ratio);
            }

            // Filtrage des monstres
            let filtered_tierlist = filter_monster(&tierlist_data, &monsters);

            // Récupération de la saison
            let season = match get_swrt_settings(&token).await {
                Ok(season) => season,
                Err(e) => {
                    let err = format!("Impossible de récupérer la saison : {}", e);
                    ctx.send(create_embed_error(&err)).await.ok();
                    send_log(&ctx, "get_rta_core", false, &err).await.ok();
                    0
                }
            };

            // Récupération de la version SWRT
            let version = match get_swrt_version(&token).await {
                Ok(version) => version,
                Err(e) => {
                    let err = format!("Impossible de récupérer la version : {}", e);
                    ctx.send(create_embed_error(&err)).await.ok();
                    send_log(&ctx, "get_rta_core", false, &err).await.ok();
                    return Ok(());
                }
            };

            println!("Version SWRT: {}", version);

            // Préparation des IDs de monstres de la box
            let player_box_ids: HashSet<u32> = monsters.iter().map(|m| m.unit_master_id).collect();
            // Préparation des IDs core
            let mut core_ids = std::collections::HashSet::new();
            for m in &filtered_tierlist.sss_monster {
                core_ids.insert(m.monster_id);
            }
            for m in &filtered_tierlist.ss_monster {
                core_ids.insert(m.monster_id);
            }
            for m in &filtered_tierlist.s_monster {
                core_ids.insert(m.monster_id);
            }
            for m in &filtered_tierlist.a_monster {
                core_ids.insert(m.monster_id);
            }
            for m in &filtered_tierlist.b_monster {
                core_ids.insert(m.monster_id);
            }
            for m in &filtered_tierlist.c_monster {
                core_ids.insert(m.monster_id);
            }

            // Collecte des trios
            let mut seen_trios = HashSet::<(u32, u32, u32)>::new();
            let mut trios: Vec<Trio> = Vec::new();

            for base in filtered_tierlist
                .sss_monster
                .iter()
                .chain(&filtered_tierlist.ss_monster)
                .chain(&filtered_tierlist.s_monster)
                .chain(&filtered_tierlist.a_monster)
                .chain(&filtered_tierlist.b_monster)
                .chain(&filtered_tierlist.c_monster)
            {
                let rank_duos = match rank {
                    Rank::C1 | Rank::C2 | Rank::C3 => 0,
                    Rank::P1 | Rank::P2 | Rank::P3 => 4,
                    Rank::G1 | Rank::G2 => 1,
                    Rank::G3 => 3,
                };
                if let Ok(duos) =
                    get_monster_duos_cached(&token, season, &version, base.monster_id, rank_duos).await
                {
                    for duo in duos {
                        let (b, o, t) = (base.monster_id, duo.team_one_id, duo.team_two_id);

                        // 👉 Si on a un filtre de monstre, on skip ceux qui ne le contiennent pas
                        if let Some(filter_id) = filter_monster_id {
                            if b != filter_id && o != filter_id && t != filter_id {
                                continue;
                            }
                        }

                        let mode_id = match mode {
                            Mode::MetaSlayer => 0,
                            Mode::FunAndCasual => 1,
                        };

                        if mode_id == 0 {
                            // on ne garde que les trios 100% « core »
                            if !core_ids.contains(&o) || !core_ids.contains(&t) {
                                continue;
                            }
                        } else {
                            // on ne garde que les trios où o et t sont dans la box du joueur
                            if !player_box_ids.contains(&o) || !player_box_ids.contains(&t) {
                                continue;
                            }
                        }

                        // clé triée pour être indépendante de l’ordre
                        let mut key = [b, o, t];
                        key.sort_unstable();
                        let key = (key[0], key[1], key[2]);
                        // si on l’a déjà vu, on skip
                        if !seen_trios.insert(key) {
                            continue;
                        }
                        // sinon on calcule score et on push

                        if let Ok(rate) = duo.win_rate.parse::<f32>() {
                            let picks = duo.pick_total;
                            // score de base
                            let mut score = rate * (1.0 + (picks as f32).ln());

                            // on calcule le late-pick ratio max sur les 3 membres
                            let max_late = [b, o, t]
                                .iter()
                                .map(|id| *late_pick_ratio_map.get(id).unwrap_or(&0.0))
                                .fold(0.0_f32, f32::max);

                            // penalty = 1 − late_ratio : si late_ratio=1.0, tout est supprimé
                            let penalty = 1.0 - max_late;
                            score *= penalty;

                            // Détection Light/Dark avec exclusions et boost variable
                            let excluded_ids: HashSet<u32> =
                                [21814, 28915, 19214, 19215].iter().cloned().collect();

                            let light_dark_count = [b, o, t]
                                .iter()
                                .filter(|&&id| {
                                    // on exclut d'abord certains IDs
                                    !excluded_ids.contains(&id)
                                    // puis on ne garde que Light ou Dark
                                    && matches!(
                                        element_map.get(&id).map(String::as_str),
                                        Some("Light") | Some("Dark")
                                    )
                                })
                                .count();

                            // si on a au moins 1 monstre Light/Dark (hors exclusions), on calcule le boost
                            if light_dark_count > 0 {
                                let boost = match light_dark_count {
                                    1 => 0.4,
                                    2 => 0.6,
                                    3 => 0.8,
                                    _ => 0.0, // dans l'improbable cas de >3, pas de boost supplémentaire
                                };
                                score *= 1.0 + boost;
                            }

                            trios.push(Trio {
                                base: b,
                                one: o,
                                two: t,
                                win_rate: rate,
                                pick_total: picks,
                                weighted_score: score,
                                emojis: None,
                            });
                        }
                    }
                } else {
                    send_log(&ctx, "get_rta_core", false, "Erreur highdata").await?;
                }
            }

            // Tri et top
            trios.sort_by(|a, b| b.weighted_score.partial_cmp(&a.weighted_score).unwrap());
            let mut top = trios.into_iter().take(15).collect::<Vec<_>>();

            // Récupération des emojis
            let collection = get_mob_emoji_collection().await.map_err(|_| {
                Error::from(std::io::Error::new(std::io::ErrorKind::Other, "DB error"))
            })?;
            for t in &mut top {
                let emojis_string = format!(
                    "{} {} {}",
                    get_emoji_from_id(&collection, t.base)
                        .await
                        .unwrap_or_default(),
                    get_emoji_from_id(&collection, t.one)
                        .await
                        .unwrap_or_default(),
                    get_emoji_from_id(&collection, t.two)
                        .await
                        .unwrap_or_default()
                );
                t.emojis = Some(emojis_string);
            }

            // Récupération en string du rank sélectionné
            let rank_str = match rank {
                Rank::C1 => "C1",
                Rank::C2 => "C2",
                Rank::C3 => "C3",
                Rank::P1 => "P1",
                Rank::P2 => "P2",
                Rank::P3 => "P3",
                Rank::G1 => "G1",
                Rank::G2 => "G2",
                Rank::G3 => "G3",
            };

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

            // Affichage final unique
            let mode_str = match mode {
                Mode::MetaSlayer => "Meta Slayer",
                Mode::FunAndCasual => "Fun and Casual",
            };
            let mut msg = if let Some(ref name) = monster {
                format!(
                    "🎯 Trios for `{}` to play in `{}` (Mode `{}`) focusing on `{}`:\n",
                    wizard_name, rank_str, mode_str, name
                )
            } else {
                format!(
                    "🎯 Trios for `{}` to play in `{}` (Mode `{}`):\n",
                    wizard_name, rank_str, mode_str
                )
            };
            if top.is_empty() {
                msg.push_str("– No Trio Found\n");
            } else {
                for t in &top {
                    msg.push_str(&format!(
                        "- {} → **{:.1} %** WR ({})\n",
                        t.emojis.clone().unwrap_or_default(),
                        t.win_rate * 100.0,
                        t.pick_total,
                    ));
                }
            }
            ctx.say(msg).await?;
        }
        Err(e) => {
            let err_msg = format!("Erreur : {}", e);
            let reply = ctx.send(create_embed_error(&err_msg)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(&ctx, "Command: /get_rta_core", false, &err_msg).await?;
        }
    }

    Ok(())
}
