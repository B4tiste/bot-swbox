use crate::commands::mob_stats::utils::remap_monster_id;
use crate::commands::rta_core::models::{
    Companion, LucksackPatch, LucksackTrioRecord, LucksackTrioResponse, LucksackWithTrioResponse,
    Monster, MonsterEntry, MonstersFile, Sort, TierListData, TrioStat,
};
use crate::commands::shared::clients::http_client;
use anyhow::{Context, Result};
use chrono::NaiveDate;
use futures::stream::TryStreamExt;
use mongodb::{bson::doc, Collection};
use poise::serenity_prelude as serenity;
use reqwest::Client;
use serde_json::Value;
use serenity::builder::{
    CreateActionRow, CreateEmbed, CreateEmbedFooter, CreateSelectMenu, CreateSelectMenuOption,
};
use serenity::CreateSelectMenuKind;
use std::collections::{HashMap, HashSet};
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

/// Charge toute la collection mob-emoji en `HashMap<com2us_id, "<:name:id>">`,
/// pour éviter un `find_one` par monstre à chaque rendu interactif.
pub async fn load_all_mob_emojis(
    collection: &Collection<mongodb::bson::Document>,
) -> HashMap<u32, String> {
    let mut map = HashMap::new();
    let Ok(mut cursor) = collection.find(doc! {}).await else {
        return map;
    };
    while let Ok(Some(d)) = cursor.try_next().await {
        let com2us_id = d
            .get_i64("com2us_id")
            .ok()
            .map(|v| v as u32)
            .or_else(|| d.get_i32("com2us_id").ok().map(|v| v as u32));
        let (Some(id), Ok(emoji_id), Ok(emoji_name)) =
            (com2us_id, d.get_str("id"), d.get_str("name"))
        else {
            continue;
        };
        map.insert(id, format!("<:{}:{}>", emoji_name, emoji_id));
    }
    map
}

/// Dérive les compagnons (4e/5e picks) d'un trio à partir du pool déjà récupéré :
/// un trio du pool partageant exactement 2 membres avec `{A,B,C}` révèle son 3e
/// membre comme compagnon. Aucun appel réseau. Exclut les membres du trio et
/// le monstre `exclude` éventuel. `count` = somme des co-occurrences.
pub fn derive_companions(
    pool: &[TrioStat],
    trio: [u32; 3],
    exclude: Option<u32>,
    max: usize,
) -> Vec<Companion> {
    let mut agg: HashMap<u32, (u64, f64)> = HashMap::new();
    for t in pool {
        let shared = t.ids.iter().filter(|id| trio.contains(id)).count();
        if shared != 2 {
            continue;
        }
        let Some(other) = t.ids.iter().copied().find(|id| !trio.contains(id)) else {
            continue;
        };
        if Some(other) == exclude {
            continue;
        }
        let entry = agg.entry(other).or_insert((0, 0.0));
        entry.0 += t.count as u64;
        entry.1 += t.win_rate as f64 * t.count as f64;
    }

    let mut companions: Vec<Companion> = agg
        .into_iter()
        .map(|(id, (count, wr_sum))| Companion {
            id,
            count: count.min(u32::MAX as u64) as u32,
            win_rate: if count > 0 {
                (wr_sum / count as f64) as f32
            } else {
                0.0
            },
        })
        .collect();
    companions.sort_by_key(|c| std::cmp::Reverse(c.count));
    companions.truncate(max);
    companions
}

/// Sections affichées selon le tri demandé : sans tri, 5 Most Played + 5 Best Winrate ;
/// avec tri, une seule section de 10. Tuple = (titre, icône, trios).
pub fn build_sections(
    playable: &[TrioStat],
    sort: Option<&Sort>,
) -> Vec<(&'static str, &'static str, Vec<TrioStat>)> {
    match sort {
        None => vec![
            ("Most Played", "🔥", top_by_count(playable, 5)),
            ("Best Winrate", "🏆", top_by_wr(playable, 5)),
        ],
        Some(Sort::MostPlayed) => vec![("Most Played", "🔥", top_by_count(playable, 10))],
        Some(Sort::BestWinrate) => vec![("Best Winrate", "🏆", top_by_wr(playable, 10))],
    }
}

fn top_by_count(playable: &[TrioStat], n: usize) -> Vec<TrioStat> {
    let mut v = playable.to_vec();
    v.sort_by_key(|t| std::cmp::Reverse(t.count));
    v.truncate(n);
    v
}

fn top_by_wr(playable: &[TrioStat], n: usize) -> Vec<TrioStat> {
    let mut v = playable.to_vec();
    v.sort_by(|a, b| {
        b.win_rate
            .partial_cmp(&a.win_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    v.truncate(n);
    v
}

fn short_name(full: &str) -> String {
    full.split(" - ").next().unwrap_or(full).to_string()
}

fn emoji_or_fallback(
    emoji_map: &HashMap<u32, String>,
    id_to_name: &HashMap<u32, String>,
    id: u32,
) -> String {
    if let Some(e) = emoji_map.get(&id) {
        e.clone()
    } else if let Some(name) = id_to_name.get(&id) {
        short_name(name)
    } else {
        "❓".to_string()
    }
}

/// Étoiles de popularité relatives au compagnon le plus fréquent (5★ = max).
fn companion_stars(count: u32, max_count: u32) -> String {
    if max_count == 0 {
        return "☆☆☆☆☆".to_string();
    }
    let ratio = (count as f32 / max_count as f32).clamp(0.0, 1.0);
    let full = ((ratio * 5.0).round() as usize).clamp(1, 5);
    format!("{}{}", "★".repeat(full), "☆".repeat(5 - full))
}

/// Convertit "<:name:id>" en ReactionType::Custom pour l'emoji d'une option de menu.
fn custom_emoji_reaction(render: &str) -> Option<serenity::ReactionType> {
    let inner = render.strip_prefix("<:")?.strip_suffix('>')?;
    let (name, id) = inner.rsplit_once(':')?;
    let id: u64 = id.parse().ok()?;
    Some(serenity::ReactionType::Custom {
        animated: false,
        id: id.into(),
        name: Some(name.to_string()),
    })
}

/// Joint des lignes en respectant un budget de caractères (limite de champ Discord).
fn join_within(lines: &[String], max: usize) -> String {
    let mut out = String::new();
    for line in lines {
        let extra = line.chars().count() + usize::from(!out.is_empty());
        if out.chars().count() + extra > max {
            break;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
    }
    out
}

/// Construit l'embed principal de /get_rta_core. Rendu synchrone (emoji_map préchargé).
#[allow(clippy::too_many_arguments)]
pub fn build_rta_core_embed(
    wizard_name: &str,
    rank_label: &str,
    focus_name: Option<&str>,
    exclude_name: Option<&str>,
    playable_count: usize,
    sections: &[(&'static str, &'static str, Vec<TrioStat>)],
    emoji_map: &HashMap<u32, String>,
    id_to_name: &HashMap<u32, String>,
    box_ids: &HashSet<u32>,
    selected: Option<&([u32; 3], Vec<Companion>)>,
) -> CreateEmbed {
    let mut embed = CreateEmbed::default()
        .title(format!("🎯 Trios for {} — {}", wizard_name, rank_label))
        .color(serenity::Colour::from_rgb(120, 153, 255))
        .footer(CreateEmbedFooter::new(format!(
            "Data from lucksack.gg · {} playable trios",
            playable_count
        )));

    let mut desc = String::new();
    if let Some(f) = focus_name {
        desc.push_str(&format!("🔎 Focusing on **{}**\n", short_name(f)));
    }
    if let Some(e) = exclude_name {
        desc.push_str(&format!("🚫 Excluding **{}**\n", short_name(e)));
    }
    if !desc.is_empty() {
        embed = embed.description(desc);
    }

    for (title, icon, trios) in sections {
        let value = if trios.is_empty() {
            "- No Trio Found".to_string()
        } else {
            let lines: Vec<String> = trios
                .iter()
                .map(|t| {
                    format!(
                        "{} {} {} → **{:.1} %** WR ({})",
                        emoji_or_fallback(emoji_map, id_to_name, t.ids[0]),
                        emoji_or_fallback(emoji_map, id_to_name, t.ids[1]),
                        emoji_or_fallback(emoji_map, id_to_name, t.ids[2]),
                        t.win_rate * 100.0,
                        t.count
                    )
                })
                .collect();
            join_within(&lines, 1024)
        };
        embed = embed.field(format!("{} {}", icon, title), value, false);
    }

    if let Some((trio, companions)) = selected {
        let header = format!(
            "🧩 Companions for {} {} {}",
            emoji_or_fallback(emoji_map, id_to_name, trio[0]),
            emoji_or_fallback(emoji_map, id_to_name, trio[1]),
            emoji_or_fallback(emoji_map, id_to_name, trio[2]),
        );
        let value = if companions.is_empty() {
            "No reliable companion data for this trio.".to_string()
        } else {
            let max_count = companions.iter().map(|c| c.count).max().unwrap_or(0);
            let mut lines: Vec<String> = companions
                .iter()
                .map(|c| {
                    let badge = if box_ids.contains(&c.id) {
                        "✅"
                    } else {
                        "❌"
                    };
                    format!(
                        "{} {}  {} · {:.0}% WR",
                        emoji_or_fallback(emoji_map, id_to_name, c.id),
                        badge,
                        companion_stars(c.count, max_count),
                        c.win_rate * 100.0
                    )
                })
                .collect();
            lines.push("✅ = in your box · ❌ = missing".to_string());
            join_within(&lines, 1024)
        };
        embed = embed.field(header, value, false);
    }

    embed
}

/// Menu de sélection listant les trios affichés (dédupliqués). Sélectionner un trio
/// déclenche l'affichage de ses compagnons. Valeur de l'option = "id_id_id".
pub fn build_trio_select_menu(
    displayed: &[TrioStat],
    emoji_map: &HashMap<u32, String>,
    id_to_name: &HashMap<u32, String>,
) -> Option<CreateActionRow> {
    if displayed.is_empty() {
        return None;
    }
    let mut seen: HashSet<[u32; 3]> = HashSet::new();
    let mut options: Vec<CreateSelectMenuOption> = Vec::new();
    for t in displayed {
        let mut key = t.ids;
        key.sort_unstable();
        if !seen.insert(key) {
            continue;
        }
        let value = format!("{}_{}_{}", t.ids[0], t.ids[1], t.ids[2]);
        let label: String = format!(
            "{} + {} + {}",
            short_name(id_to_name.get(&t.ids[0]).map(|s| s.as_str()).unwrap_or("?")),
            short_name(id_to_name.get(&t.ids[1]).map(|s| s.as_str()).unwrap_or("?")),
            short_name(id_to_name.get(&t.ids[2]).map(|s| s.as_str()).unwrap_or("?")),
        )
        .chars()
        .take(100)
        .collect();

        let mut option = CreateSelectMenuOption::new(label, value);
        if let Some(reaction) = emoji_map
            .get(&t.ids[0])
            .and_then(|e| custom_emoji_reaction(e))
        {
            option = option.emoji(reaction);
        }
        options.push(option);
        if options.len() >= 25 {
            break;
        }
    }

    let menu = CreateSelectMenu::new(
        "rta_core_trio_select",
        CreateSelectMenuKind::String { options },
    )
    .placeholder("Pick a trio to reveal its 4th/5th picks");

    Some(CreateActionRow::SelectMenu(menu))
}
