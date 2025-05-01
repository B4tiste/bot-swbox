use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::send_log;
use crate::{Data, API_TOKEN};
use poise::serenity_prelude::{Attachment, Error};

use crate::commands::mob_stats::utils::get_swrt_settings;
use crate::commands::rta_core::models::{Rank, Trio};
use crate::commands::rta_core::utils::{
    filter_monster, get_monster_duos, get_monsters_from_json_bytes, get_tierlist_data, get_emoji_from_id
};
use crate::commands::player_stats::utils::get_mob_emoji_collection;

/// Commande get_rta_core
#[poise::command(slash_command)]
pub async fn get_rta_core(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    file: Attachment,
    #[description = "Select the targeted rank"] rank: Rank,
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

    // Extraction des monsters
    match get_monsters_from_json_bytes(&bytes, "monsters.json") {
        Ok(monsters) => {
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

            // Collecte des trios
            let mut trios: Vec<Trio> = Vec::new();
            for base in filtered_tierlist
                .sss_monster
                .iter()
                .chain(&filtered_tierlist.ss_monster)
                .chain(&filtered_tierlist.s_monster)
                .chain(&filtered_tierlist.a_monster)
            {
                let rank_duos = match rank {
                    Rank::C1 | Rank::C2 | Rank::C3 => 0,
                    Rank::P1 | Rank::P2 | Rank::P3 => 4,
                    Rank::G1 | Rank::G2 => 1,
                    Rank::G3 => 3,
                };
                if let Ok(duos) = get_monster_duos(&token, season, base.monster_id, rank_duos).await {
                    for duo in duos {
                        if core_ids.contains(&duo.team_one_id) && core_ids.contains(&duo.team_two_id) {
                            if let Ok(rate) = duo.win_rate.parse::<f32>() {
                                let picks = duo.pick_total;
                                let score = rate * (picks as f32);
                                trios.push(Trio {
                                    base: base.monster_id,
                                    one: duo.team_one_id,
                                    two: duo.team_two_id,
                                    win_rate: rate,
                                    pick_total: picks,
                                    weighted_score: score,
                                    emojis: None,
                                });
                            }
                        }
                    }
                } else {
                    send_log(&ctx, "get_rta_core", false, "Erreur highdata").await?;
                }
            }

            // Tri et top 10
            trios.sort_by(|a, b| b.weighted_score.partial_cmp(&a.weighted_score).unwrap());
            let mut top10 = trios.into_iter().take(10).collect::<Vec<_>>();

            // Récupération des emojis
            let collection = get_mob_emoji_collection()
                .await
                .map_err(|_| Error::from(std::io::Error::new(std::io::ErrorKind::Other, "DB error")))?;
            for t in &mut top10 {
                let emojis_string = format!(
                    "{} {} {}",
                    get_emoji_from_id(&collection, t.base).await.unwrap_or_default(),
                    get_emoji_from_id(&collection, t.one).await.unwrap_or_default(),
                    get_emoji_from_id(&collection, t.two).await.unwrap_or_default()
                );
                t.emojis = Some(emojis_string);
            }

            // Affichage final unique
            let mut msg = String::from("🎯 Top 10 des trios “core” pondérés (win_rate × pick_total) :\n");
            if top10.is_empty() {
                msg.push_str("– Aucun trio trouvé, voir les logs DEBUG pour plus de détails.\n");
            } else {
                for t in &top10 {
                    msg.push_str(&format!(
                        "• {} → {:.2}% sur {} picks (score {:.0})\n",
                        t.emojis.clone().unwrap_or_default(),
                        t.win_rate * 100.0,
                        t.pick_total,
                        t.weighted_score,
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
