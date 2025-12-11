use ab_glyph::{FontArc, PxScale, Font, ScaleFont};
use anyhow::{anyhow, Context, Result};
use image::GenericImage;
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use mongodb::{bson::doc, Client, Collection};
use poise::serenity_prelude as serenity;
use rand::seq::IndexedRandom;
use serde::Deserialize;
use serde_json::Value;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use chrono::NaiveDateTime;

use crate::commands::mob_stats::utils::remap_monster_id;
use crate::commands::ranks::utils::get_rank_info;
use crate::commands::shared::player_alias::PLAYER_ALIAS_MAP;
use crate::MONGO_URI;

#[derive(Debug, Deserialize)]
pub struct Player {
    #[serde(rename = "playerName")]
    pub name: String,

    #[serde(rename = "swrtPlayerId")]
    pub swrt_player_id: i64,

    #[serde(rename = "playerCountry")]
    pub player_country: String,

    #[serde(rename = "playerScore")]
    pub player_score: Option<i32>,

    #[serde(rename = "playerServer")]
    pub player_server: i32, // 1..6 : KR, JP, CN, GB, AS, EU
}


#[derive(Debug, Deserialize)]
struct SearchResponse {
    data: Option<SearchData>,
    #[serde(rename = "enMessage")]
    en_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchData {
    list: Vec<Player>,
}

#[derive(Debug, Deserialize)]
pub struct PlayerDetail {
    #[serde(rename = "playerName")]
    pub name: String,
    #[serde(rename = "playerScore")]
    pub player_score: Option<i32>,
    #[serde(rename = "playerRank")]
    pub player_rank: Option<i32>,
    #[serde(rename = "winRate")]
    pub win_rate: Option<f32>,
    #[serde(rename = "headImg")]
    pub head_img: Option<String>,
    #[serde(rename = "playerMonsters")]
    pub player_monsters: Option<Vec<PlayerMonster>>,
    // #[serde(rename = "monsterSimpleImgs")]
    // pub monster_simple_imgs: Option<Vec<String>>,
    #[serde(rename = "monsterLDImgs")]
    pub monster_ld_imgs: Option<Vec<String>>,
    #[serde(rename = "seasonCount")]
    pub season_count: Option<i32>,
    #[serde(rename = "playerCountry")]
    pub player_country: String,
    #[serde(rename = "swrtPlayerId")]
    pub swrt_player_id: i64,
    #[serde(rename = "playerId")]
    pub player_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct PlayerMonster {
    // #[serde(rename = "monsterId")]
    // pub monster_id: i32,
    #[serde(rename = "monsterImg")]
    pub monster_img: String,
    #[serde(rename = "winRate")]
    pub win_rate: f32,
    #[serde(rename = "pickTotal")]
    pub pick_total: i32,
}

#[derive(Debug, Deserialize)]
struct DetailResponse {
    data: Option<PlayerDetailWrapper>,
    #[serde(rename = "enMessage")]
    en_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlayerDetailWrapper {
    player: PlayerDetail,
    #[serde(rename = "playerMonsters")]
    player_monsters: Option<Vec<PlayerMonster>>,
    // #[serde(rename = "monsterSimpleImgs")]
    // monster_simple_imgs: Option<Vec<String>>,
    #[serde(rename = "monsterLDImgs")]
    monster_ld_imgs: Option<Vec<String>>,
    #[serde(rename = "seasonCount")]
    season_count: Option<i32>,
}

// Replay
#[derive(Debug, Deserialize)]
struct Root {
    data: DataWrapper,
}

#[derive(Debug, Deserialize)]
struct DataWrapper {
    page: PageData,
}

#[derive(Debug, Deserialize)]
struct PageData {
    list: Vec<Replay>,
}

#[derive(Debug, Deserialize)]
pub struct Replay {
    #[serde(rename = "playerOne")]
    pub player_one: ReplayPlayer,

    #[serde(rename = "playerTwo")]
    pub player_two: ReplayPlayer,

    #[serde(rename = "firstPick")]
    first_pick: u32,

    #[serde(rename = "status")]
    pub status: u32,

    #[serde(rename = "createDate")]
    pub date: String,
}

#[derive(Debug, Deserialize)]
pub struct ReplayPlayer {
    #[serde(rename = "monsterInfoList")]
    pub monster_info_list: Vec<ReplayMonster>,

    #[serde(rename = "banMonsterId")]
    ban_monster_id: u32,

    #[serde(rename = "leaderMonsterId")]
    leader_monster_id: u32,

    #[serde(rename = "playerId")]
    player_id: u32,

    #[serde(rename = "playerName")]
    pub player_name: String,

    #[serde(rename = "playerScore")]
    player_score: u32,
}

#[derive(Debug, Deserialize)]
pub struct ReplayMonster {
    #[serde(rename = "imageFilename")]
    image_filename: String,

    #[serde(rename = "monsterId")]
    pub monster_id: u32,
}

pub async fn get_user_detail(token: &str, player_id: &i64) -> Result<PlayerDetail> {
    let url = format!(
        "https://m.swranking.com/api/player/detail?swrtPlayerId={}",
        player_id
    );
    let client = reqwest::Client::new();

    let res = client
        .get(&url)
        .header("Authentication", token)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let status = res.status();
    let resp_json: DetailResponse = res.json().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "Error status {}: {:?}",
            status,
            resp_json.en_message
        ));
    }

    resp_json
        .data
        .map(|d| PlayerDetail {
            name: d.player.name,
            player_score: d.player.player_score,
            player_rank: d.player.player_rank,
            win_rate: d.player.win_rate,
            head_img: d.player.head_img,
            player_monsters: d.player_monsters,
            player_country: d.player.player_country,
            swrt_player_id: d.player.swrt_player_id,
            player_id: d.player.player_id,
            // monster_simple_imgs: d.monster_simple_imgs,
            monster_ld_imgs: d.monster_ld_imgs,
            season_count: d.season_count,
        })
        .ok_or_else(|| anyhow!("Player details not found"))
}

pub async fn search_users(token: &str, username: &str) -> Result<Vec<Player>> {
    let url = "https://m.swranking.com/api/player/list";
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "pageNum": 1,
        "pageSize": 15,
        "playerName": username,
        "online": false,
        "level": null,
        "playerMonsters": []
    });

    let res = client
        .post(url)
        .json(&body)
        .header("Authentication", token)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let status = res.status();
    let resp_json: SearchResponse = res.json().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "Error status {}: {:?}",
            status,
            resp_json.en_message
        ));
    }

    Ok(resp_json.data.map(|d| d.list).unwrap_or_default())
}

pub async fn get_mob_emoji_collection() -> Result<Collection<mongodb::bson::Document>> {
    let mongo_uri = {
        let uri_guard = MONGO_URI.lock().unwrap();
        uri_guard.clone()
    };

    let client = Client::with_uri_str(&mongo_uri).await?;
    Ok(client
        .database("bot-swbox-db")
        .collection::<mongodb::bson::Document>("mob-emoji"))
}

pub async fn get_emoji_from_filename(
    collection: &Collection<mongodb::bson::Document>,
    filename: &str,
) -> Option<String> {
    let name_no_ext = filename.replace(".png", "").replace("unit_icon_", "");

    let emoji_doc = collection
        .find_one(doc! { "name": &name_no_ext })
        .await
        .ok()??;

    let id = emoji_doc.get_str("id").ok()?;
    Some(format!("<:{}:{}>", name_no_ext, id))
}

// Utilitaire global ou lazy_static
fn get_filename_to_id_map() -> HashMap<String, i32> {
    let file = fs::read_to_string("monsters_elements.json").unwrap();
    let v: Value = serde_json::from_str(&file).unwrap();
    let arr = v["monsters"].as_array().unwrap();

    let mut map = HashMap::new();
    for m in arr {
        let obtainable = m["obtainable"].as_bool().unwrap_or(false);
        if obtainable {
            let filename = m["image_filename"].as_str().unwrap().to_string();
            let com2us_id = m["com2us_id"].as_i64().unwrap() as i32;
            map.insert(filename, com2us_id);
        }
    }
    map
}

pub async fn format_player_ld_monsters_emojis(details: &PlayerDetail) -> Vec<String> {
    let mut emojis = vec![];

    let mut files = vec![];
    if let Some(ld) = &details.monster_ld_imgs {
        files.extend(ld.clone());
    }

    files.sort();
    files.dedup();

    let filename_to_id = get_filename_to_id_map(); // ← charge le mapping ici

    if let Ok(collection) = get_mob_emoji_collection().await {
        for file in files {
            // 1. Trouver l’id depuis le filename
            if let Some(&monster_id) = filename_to_id.get(&file) {
                // 2. Remap l’id
                let remapped_id = remap_monster_id(monster_id);

                // 3. Chercher le bon emoji avec ce com2us_id
                let emoji_doc = collection
                    .find_one(doc! { "com2us_id": remapped_id })
                    .await
                    .ok()
                    .flatten();

                if let Some(emoji_doc) = emoji_doc {
                    let natural_stars = emoji_doc.get_i32("natural_stars").unwrap_or(0);
                    if natural_stars < 5 {
                        continue;
                    }
                    if let Ok(id) = emoji_doc.get_str("id") {
                        let name = emoji_doc.get_str("name").unwrap_or("unit");
                        emojis.push(format!("<:{}:{}>", name, id));
                    }
                }
            }
        }
    }

    emojis
}

pub async fn format_player_monsters(details: &PlayerDetail) -> Vec<String> {
    let mut output = vec![];

    let collection = match get_mob_emoji_collection().await {
        Ok(c) => c,
        Err(_) => return output,
    };

    let filename_to_id = get_filename_to_id_map(); // ← mapping filename → id

    if let Some(monsters) = &details.player_monsters {
        for (index, m) in monsters.iter().enumerate() {
            // 1. Trouver l’id depuis le filename
            if let Some(&monster_id) = filename_to_id.get(&m.monster_img) {
                // 2. Remap l’id
                let remapped_id = remap_monster_id(monster_id);

                // Debug
                // println!(
                //     "DEBUG: Processing monster_img='{}', monster_id={}, remapped_id={}",
                //     m.monster_img, monster_id, remapped_id
                // );

                // 3. Chercher le bon emoji avec ce com2us_id
                let emoji_doc = collection
                    .find_one(doc! { "com2us_id": remapped_id })
                    .await
                    .ok()
                    .flatten();

                // Debug
                // println!(
                //     "DEBUG: Emoji document for remapped_id {}: {:?}",
                //     remapped_id, emoji_doc
                // );

                if let Some(emoji_doc) = emoji_doc {
                    if let Ok(id) = emoji_doc.get_str("id") {
                        let name = emoji_doc.get_str("name").unwrap_or("unit");
                        let emoji = format!("<:{}:{}>", name, id);

                        // Debug
                        // println!(
                        //     "DEBUG: Created emoji for monster_img='{}', monster_id={}, remapped_id={}, name='{}', id='{}', emoji='{}'",
                        //     m.monster_img, monster_id, remapped_id, name, id, emoji
                        // );

                        let pick_display = if m.pick_total >= 1000 {
                            let k = m.pick_total / 1000;
                            let remainder = (m.pick_total % 1000) / 100;
                            if remainder == 0 {
                                format!("{}k", k)
                            } else {
                                format!("{}k{}", k, remainder)
                            }
                        } else {
                            m.pick_total.to_string()
                        };

                        let entry = format!(
                            "{}. {} {} picks, **{:.1} %** WR\n",
                            index + 1,
                            emoji,
                            pick_display,
                            m.win_rate
                        );
                        output.push(entry);
                    }
                }
            }
        }
    }

    output
}

/// Creates an embed for player info display
pub fn create_player_embed(
    details: &PlayerDetail,
    ld_emojis: Vec<String>,
    top_monsters: Vec<String>,
    rank_emojis: String,
    has_image: i32,
) -> CreateEmbed {
    let format_emojis_with_split = |list: Vec<String>| -> Vec<(String, String)> {
        let full_text = list.join(" ");

        if full_text.len() <= 1020 {
            let display = if full_text.is_empty() {
                "None".to_string()
            } else {
                full_text
            };
            return vec![("".to_string(), display)];
        }

        // Split into two parts
        let mut part1 = Vec::new();
        let mut part2 = Vec::new();
        let mid = list.len() / 2;

        for (i, item) in list.iter().enumerate() {
            if i < mid {
                part1.push(item.clone());
            } else {
                part2.push(item.clone());
            }
        }

        let mut part1_text = part1.join(" ");
        let mut part2_text = part2.join(" ");

        // Trim if still too long
        while part1_text.len() > 1020 && !part1.is_empty() {
            part1.pop();
            part1_text = part1.join(" ");
        }
        if part1_text.len() >= 1020 {
            part1_text.push_str(" …");
        }

        while part2_text.len() > 1020 && !part2.is_empty() {
            part2.pop();
            part2_text = part2.join(" ");
        }
        if part2_text.len() >= 1020 {
            part2_text.push_str(" …");
        }

        vec![
            ("(1/2)".to_string(), part1_text),
            ("(2/2)".to_string(), part2_text),
        ]
    };

    let ld_fields = format_emojis_with_split(ld_emojis);
    let top_fields = format_emojis_with_split(top_monsters);

    // Créer une liste de gif qui seront choisis aléatoirement pour l'image de fond
    let gifs = vec![
        "https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExczN3N3YxcjAzc3g5bWpqY2VleXA2MHN0bm9rcDVvaG00MGZrbHoweSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/2WjpfxAI5MvC9Nl8U7/giphy.gif",
        "https://media3.giphy.com/media/v1.Y2lkPTc5MGI3NjExeXRmY2locjR2cnJ5d2JvdWF5djN5cTRlajdna3JxeTA4d2RsdzVxciZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/rGDZbxkkjo0hfLe4EA/giphy.gif",
        "https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExbTRsODVtNThvbTl2bW50NnhzYjB5MWN3aHF5dW40NTIwMmpoaGk0ayZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/WiIuC6fAOoXD2/giphy.gif",
        "https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExZHFreWtobWUwdmx4MGlpYXZvZjVubDd4ejBuOTcweTh1d3IyaGtzeiZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/KDZdynDNJUrrp7EjTM/giphy.gif"
    ];

    let random_gif = gifs.choose(&mut rand::rng()).unwrap_or(&gifs[0]);

    let mut embed = CreateEmbed::default()
        .title(format!(
            ":flag_{}: {} (id: {}) {} RTA Statistics (Regular Season only)",
            details.player_country.to_lowercase(),
            details.name,
            details.player_id,
            PLAYER_ALIAS_MAP
                .get(&details.swrt_player_id)
                .map(|alias| format!("(aka. {})", alias))
                .unwrap_or_default()
        ))
        .thumbnail(details.head_img.clone().unwrap_or_default())
        .color(serenity::Colour::from_rgb(0, 180, 255))
        .description("⚠️ Stats are not 100% accurate ➡️ The very last battle is not included in the elo/rank, and people under/around 1300 (C1) elo will have weird stats (missing games, weird winrates) ⚠️")
        .field("WinRate", format!("{:.1} %", details.win_rate.unwrap_or(0.0) * 100.0), true)
        .field("Elo", details.player_score.unwrap_or(0).to_string(), true)
        .field("Rank", details.player_rank.unwrap_or(0).to_string(), true)
        .field("🏆 Approx. Rank", rank_emojis, true)
        .field("Matches Played", details.season_count.unwrap_or(0).to_string(), true);

    // Add LD fields
    for (suffix, text) in ld_fields {
        let field_name = if suffix.is_empty() {
            "✨ LD Monsters (RTA only)".to_string()
        } else {
            format!("✨ LD Monsters (RTA only) {}", suffix)
        };
        embed = embed.field(field_name, text, false);
    }

    // Add top monsters fields
    for (suffix, text) in top_fields {
        let field_name = if suffix.is_empty() {
            "🔥 Most Used Units Winrate".to_string()
        } else {
            format!("🔥 Most Used Units Winrate {}", suffix)
        };
        embed = embed.field(field_name, text, false);
    }

    embed
        .image(
            if has_image == 1 {
                "attachment://replay.png"
            } else if has_image  == 0 {
                random_gif
            }
            else {
                ""
            }
        )
        .footer(CreateEmbedFooter::new(
            "Data is gathered from m.swranking.com",
        ))
}

pub async fn get_rank_emojis_for_score(score: i32) -> Result<String> {
    let rank_data = get_rank_info().await.map_err(|e| anyhow!(e))?;

    for (emoji, threshold) in rank_data.iter().rev() {
        if score >= *threshold {
            return Ok(emoji.clone());
        }
    }

    Ok("Unranked".to_string())
}

pub async fn get_recent_replays(token: &str, player_id: &i64) -> Result<Vec<Replay>> {
    let url = "https://m.swranking.com/api/player/replaylist";
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "swrtPlayerId": player_id.to_string(),
        "result": 2,
        "pageNum": 1,
        "pageSize": 6,
    });

    let res = client
        .post(url)
        .json(&body)
        .header("Authentication", token)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let status = res.status();
    let json: Root = res.json().await?;

    if !status.is_success() {
        return Err(anyhow!(
            "Error status {}: {:?}",
            status,
            json.data.page.list
        ));
    }

    Ok(json.data.page.list)
}

pub async fn create_replay_image(
    recent_replays: Vec<Replay>,
    rows: i32,
    cols: i32,
) -> Result<PathBuf> {
    let nb_battles = recent_replays.len();

    let mut sections: Vec<RgbaImage> = Vec::new();
    let mut max_width = 0;

    let mut image_cache: HashMap<String, DynamicImage> = HashMap::new();

    for battle in recent_replays.iter().take(nb_battles) {
        let actual_first_pick_id = battle.first_pick;

        let urls_player_one: Vec<String> = battle
            .player_one
            .monster_info_list
            .iter()
            .map(|m| m.image_filename.clone())
            .collect();

        let urls_player_two: Vec<String> = battle
            .player_two
            .monster_info_list
            .iter()
            .map(|m| m.image_filename.clone())
            .collect();

        let monster_ids_one: Vec<u32> = battle
            .player_one
            .monster_info_list
            .iter()
            .map(|m| m.monster_id)
            .collect();

        let monster_ids_two: Vec<u32> = battle
            .player_two
            .monster_info_list
            .iter()
            .map(|m| m.monster_id)
            .collect();

        let is_p1_first_pick = actual_first_pick_id == battle.player_one.player_id;

        let img1 = create_team_collage_custom_layout(
            &urls_player_one,
            &monster_ids_one,
            battle.player_one.ban_monster_id,
            battle.player_one.leader_monster_id,
            is_p1_first_pick,
            &mut image_cache,
        )
        .await?;

        let img2 = create_team_collage_custom_layout(
            &urls_player_two,
            &monster_ids_two,
            battle.player_two.ban_monster_id,
            battle.player_two.leader_monster_id,
            !is_p1_first_pick,
            &mut image_cache,
        )
        .await?;

        let image_width = img1.width() / 3;
        let spacing = image_width / 2;
        let combined_width = img1.width() + img2.width() + spacing;
        let height = img1.height().max(img2.height());

        let mut final_image = ImageBuffer::new(combined_width, height);
        final_image.copy_from(&img1, 0, 0).unwrap();
        final_image
            .copy_from(&img2, img1.width() + spacing, 0)
            .unwrap();

        // Left text: score + player 1 name
        let left_text = format!(
            "{} - {}",
            battle.player_one.player_score,
            battle.player_one.player_name
        );

        // Right text: player 2 name + score
        let right_text = format!(
            "{} - {}",
            battle.player_two.player_name,
            battle.player_two.player_score
        );

        // Center text: date only, formatted as DD-MM-YYYY
        let date_text = NaiveDateTime::parse_from_str(&battle.date, "%Y-%m-%d %H:%M:%S")
            .map(|dt| dt.date().format("%d-%m-%Y").to_string())
            .unwrap_or_else(|_| battle.date.clone()); // fallback if parsing fails

        let match_banner = create_match_banner(
            &left_text,
            &date_text,
            &right_text,
            combined_width,
            Rgba([0, 0, 0, 0]),
        );

        let banner_height = match_banner.height();
        let section_inner_height = banner_height + final_image.height();
        let section_inner_width = combined_width;

        let border_thickness = 10;
        let section_total_width = section_inner_width + 2 * border_thickness;
        let section_total_height = section_inner_height + 2 * border_thickness;

        let bg_color = match battle.status {
            1 => Rgba([0, 255, 0, 100]), // win
            2 => Rgba([255, 0, 0, 100]), // lose
            _ => Rgba([0, 0, 0, 100]),   // unknown
        };

        let mut section =
            ImageBuffer::from_pixel(section_total_width, section_total_height, bg_color);

        // Créer zone intérieure transparente
        let mut inner = ImageBuffer::from_pixel(
            section_inner_width,
            section_inner_height,
            Rgba([0, 0, 0, 0]),
        );

        inner.copy_from(&match_banner, 0, 0).unwrap();
        inner.copy_from(&final_image, 0, banner_height).unwrap();

        // Intégrer zone intérieure centrée dans la bordure
        section
            .copy_from(&inner, border_thickness, border_thickness)
            .unwrap();

        max_width = max_width.max(section_total_width);
        sections.push(section);
    }

    // Créer l'image finale 2 colonnes × 3 lignes
    let rows = rows as u32;
    let columns = cols as u32;
    let padding = 10;

    let section_width = sections.first().map(|img| img.width()).unwrap_or(0);
    let section_height = sections.first().map(|img| img.height()).unwrap_or(0);

    let full_width = columns * section_width + (columns - 1) * padding;
    let full_height = rows * section_height + (rows - 1) * padding;

    let mut final_image = ImageBuffer::new(full_width, full_height);

    for (i, section) in sections.iter().enumerate() {
        let col = (i % columns as usize) as u32;
        let row = (i / columns as usize) as u32;

        let x = col * (section_width + padding);
        let y = row * (section_height + padding);

        final_image.copy_from(section, x, y)?;
    }

    // Enregistrer l'image finale
    let output_path = PathBuf::from("replay.png");

    let output_path_clone = output_path.clone();
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all("/tmp")?;
        final_image.save(&output_path_clone)?;
        Ok::<_, anyhow::Error>(output_path_clone)
    })
    .await??;

    Ok(output_path)
}

async fn create_team_collage_custom_layout(
    image_filenames: &[String], // <- renommé pour plus de clarté
    monster_ids: &[u32],
    ban_id: u32,
    leader_id: u32,
    first_pick: bool,
    cache: &mut HashMap<String, DynamicImage>,
) -> Result<RgbaImage> {
    let mut images = Vec::new();
    for filename in image_filenames {
        let img = load_image_local(filename, cache).await?; // <-- on charge en local maintenant
        images.push(img);
    }

    let width = images[0].width();
    let height = images[0].height();
    let mut collage = ImageBuffer::new(width * 3, height * 2);

    const CROSS_BYTES: &[u8] = include_bytes!("cross.png");
    let cross = image::load_from_memory(CROSS_BYTES)
        .expect("Erreur lors du chargement de cross.png")
        .resize_exact(width, height, image::imageops::FilterType::Lanczos3)
        .to_rgba8();

    let mut grid_slots = vec![(0, 0); 5];

    if first_pick {
        grid_slots[1] = (1, 0);
        grid_slots[2] = (1, 1);
        grid_slots[3] = (2, 0);
        grid_slots[4] = (2, 1);
        grid_slots[0] = (0, 1);
    } else {
        grid_slots[0] = (0, 0);
        grid_slots[1] = (0, 1);
        grid_slots[2] = (1, 0);
        grid_slots[3] = (1, 1);
        grid_slots[4] = (2, 1);
    }

    for (i, (img, &monster_id)) in images.iter().zip(monster_ids).enumerate() {
        let mut rgba = img.to_rgba8();

        if monster_id == leader_id {
            let border_color = Rgba([255, 215, 0, 255]);
            let thickness = 5;
            for x in 0..width {
                for t in 0..thickness {
                    rgba.put_pixel(x, t, border_color);
                    rgba.put_pixel(x, height - 1 - t, border_color);
                }
            }
            for y in 0..height {
                for t in 0..thickness {
                    rgba.put_pixel(t, y, border_color);
                    rgba.put_pixel(width - 1 - t, y, border_color);
                }
            }
        }

        if monster_id == ban_id {
            image::imageops::overlay(&mut rgba, &cross, 0, 0);
        }

        let (grid_x, grid_y) = grid_slots[i];
        let x = grid_x as u32 * width;
        let y = if (first_pick && i == 0) || (!first_pick && i == 4) {
            ((collage.height() - height) / 2).min(collage.height() - height)
        } else {
            grid_y as u32 * height
        };

        collage.copy_from(&rgba, x, y).unwrap();
    }

    Ok(collage)
}

async fn load_image_local(
    filename: &str,
    cache: &mut HashMap<String, DynamicImage>,
) -> Result<DynamicImage> {
    if let Some(img) = cache.get(filename) {
        return Ok(img.clone());
    }

    let path = format!("assets/monster_images/{}", filename);
    let filename_string = filename.to_string();

    let img: DynamicImage;

    // Essayez de lire localement
    let local_result = tokio::task::spawn_blocking(move || -> Result<DynamicImage> {
        let data = std::fs::read(&path)
            .with_context(|| format!("Failed to read image file: {}", filename_string))?;
        Ok(image::load_from_memory(&data)
            .with_context(|| format!("Failed to decode image: {}", filename_string))?)
    })
    .await;

    match local_result {
        Ok(Ok(image)) => {
            img = image;
        }
        // Si le fichier n'est pas trouvé, on télécharge sans le sauvegarder
        Ok(Err(e))
            if e.downcast_ref::<std::io::Error>().map(|io| io.kind())
                == Some(std::io::ErrorKind::NotFound) =>
        {
            let url = format!(
                "https://swarfarm.com/static/herders/images/monsters/{}",
                filename
            );
            let bytes = reqwest::get(&url)
                .await
                .with_context(|| format!("Failed to download image from: {}", url))?
                .bytes()
                .await
                .with_context(|| {
                    format!("Failed to read downloaded bytes for file: {}", filename)
                })?;

            img = image::load_from_memory(&bytes)
                .with_context(|| format!("Failed to decode downloaded image: {}", filename))?;
        }
        Ok(Err(e)) => return Err(e),
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Blocking task failed for file: {}: {}",
                filename,
                e
            ))
        }
    }

    // After loading the image, force resize:
    let img = img.resize_exact(100, 100, image::imageops::FilterType::Lanczos3);

    cache.insert(filename.to_string(), img.clone());
    Ok(img)
}

fn create_match_banner(
    left_text: &str,
    center_text: &str,
    right_text: &str,
    width: u32,
    color: Rgba<u8>,
) -> RgbaImage {
    let height = 40;
    let mut image = ImageBuffer::from_pixel(width, height, color);

    const FONT_BYTES: &[u8] = include_bytes!("NotoSansCJK-Regular.otf");
    let font = FontArc::try_from_vec(FONT_BYTES.to_vec()).expect("Police invalide");

    let scale = PxScale::from(26.0);
    let margin = 8.0_f32;
    let width_f = width as f32;

    // On découpe grossièrement en 3 zones (gauche / centre / droite)
    let max_left_width = width_f / 3.0 - margin * 2.0;
    let max_center_width = width_f / 3.0 - margin * 2.0;
    let max_right_width = width_f / 3.0 - margin * 2.0;

    // Texte ajusté pour ne pas dépasser la zone
    let left = fit_text_to_width(&font, scale, left_text, max_left_width);
    let center = fit_text_to_width(&font, scale, center_text, max_center_width);
    let right = fit_text_to_width(&font, scale, right_text, max_right_width);

    let center_w = text_width(&font, scale, &center);
    let right_w = text_width(&font, scale, &right);

    let y = 8;

    // LEFT : collé à gauche
    let left_x: i32 = margin as i32;
    draw_bold_text_mut(
        &mut image,
        Rgba([255, 255, 255, 255]),
        left_x,
        y,
        scale,
        &font,
        &left,
    );

    // CENTER : centré
    let center_x: i32 = ((width_f - center_w) / 2.0).round() as i32;
    draw_bold_text_mut(
        &mut image,
        Rgba([255, 255, 255, 255]),
        center_x,
        y,
        scale,
        &font,
        &center,
    );

    // RIGHT : aligné à droite
    let right_x: i32 = (width_f - right_w - margin).round() as i32;
    draw_bold_text_mut(
        &mut image,
        Rgba([255, 255, 255, 255]),
        right_x,
        y,
        scale,
        &font,
        &right,
    );

    image
}


fn draw_bold_text_mut(
    image: &mut RgbaImage,
    color: Rgba<u8>,
    x: i32,
    y: i32,
    scale: PxScale,
    font: &FontArc,
    text: &str,
) {
    // Offsets pour simuler le gras
    let offsets = [(0, 0), (1, 0), (0, 1), (1, 1)];

    for (dx, dy) in offsets.iter() {
        draw_text_mut(image, color, x + dx, y + dy, scale, font, text);
    }
}

fn text_width(font: &FontArc, scale: PxScale, text: &str) -> f32 {
    let scaled = font.as_scaled(scale);
    let mut pen = 0.0f32;
    let mut prev_id = None;

    for ch in text.chars() {
        let g = scaled.glyph_id(ch);
        if let Some(pid) = prev_id {
            pen += scaled.kern(pid, g);
        }
        pen += scaled.h_advance(g);
        prev_id = Some(g);
    }

    pen
}

fn fit_text_to_width(font: &FontArc, scale: PxScale, text: &str, max_width: f32) -> String {
    let s: String = text.to_string();

    if text_width(font, scale, &s) <= max_width {
        return s;
    }

    // On rogne progressivement jusqu'à ce que ça rentre, puis on ajoute "…"
    let mut chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return s;
    }

    // Garder au moins quelques caractères
    while !chars.is_empty() && text_width(font, scale, &chars.iter().collect::<String>()) > max_width {
        chars.pop();
    }

    let mut result: String = chars.iter().collect();
    if !result.is_empty() {
        result.push('…');
    }

    result
}

