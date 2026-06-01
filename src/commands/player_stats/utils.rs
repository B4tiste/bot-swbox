use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use anyhow::{anyhow, Context, Result};
use chrono::NaiveDateTime;
use image::GenericImage;
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use mongodb::{bson::doc, Collection};
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use serde_json::Value;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use crate::commands::shared::clients::{http_client, mongo_client};
use crate::commands::shared::player_alias::PLAYER_ALIAS_MAP;
use crate::{CONQUEROR_EMOJI_ID, GUARDIAN_EMOJI_ID, PUNISHER_EMOJI_ID};

/* ------------------ Replay types ------------------ */
#[derive(Debug, Deserialize)]
pub struct Replay {
    #[serde(rename = "playerOne")]
    pub player_one: ReplayPlayer,
    #[serde(rename = "playerTwo")]
    pub player_two: ReplayPlayer,
    #[serde(rename = "firstPick")]
    pub first_pick: u32,
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
    pub ban_monster_id: u32,
    #[serde(rename = "leaderMonsterId")]
    pub leader_monster_id: u32,
    #[serde(rename = "playerId")]
    pub player_id: u32,
    #[serde(rename = "playerName")]
    pub player_name: String,
    #[serde(rename = "playerScore")]
    pub player_score: u32,
}

#[derive(Debug, Deserialize)]
pub struct ReplayMonster {
    #[serde(rename = "imageFilename")]
    pub image_filename: String,
    #[serde(rename = "monsterId")]
    pub monster_id: u32,
}

/* ------------------ Mongo / emojis helpers ------------------ */

pub async fn get_mob_emoji_collection() -> Result<Collection<mongodb::bson::Document>> {
    let client = mongo_client()?;
    Ok(client
        .database("bot-swbox-db")
        .collection::<mongodb::bson::Document>("mob-emoji"))
}

/* ------------------ Replay image generation ------------------ */

static GLOBAL_MONSTER_IMAGE_CACHE: OnceLock<RwLock<HashMap<String, DynamicImage>>> =
    OnceLock::new();
static CROSS_IMAGE_100: OnceLock<RgbaImage> = OnceLock::new();
static BANNER_FONT: OnceLock<FontArc> = OnceLock::new();
static LUCKSACK_REPLAY_PATH_CACHE: OnceLock<RwLock<HashMap<u64, PathBuf>>> = OnceLock::new();

fn global_monster_image_cache() -> &'static RwLock<HashMap<String, DynamicImage>> {
    GLOBAL_MONSTER_IMAGE_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn cross_image_100() -> &'static RgbaImage {
    CROSS_IMAGE_100.get_or_init(|| {
        const CROSS_BYTES: &[u8] = include_bytes!("cross.png");
        image::load_from_memory(CROSS_BYTES)
            .expect("Erreur lors du chargement de cross.png")
            .resize_exact(100, 100, image::imageops::FilterType::Triangle)
            .to_rgba8()
    })
}

fn banner_font() -> &'static FontArc {
    BANNER_FONT.get_or_init(|| {
        const FONT_BYTES: &[u8] = include_bytes!("NotoSansCJK-Regular.otf");
        FontArc::try_from_vec(FONT_BYTES.to_vec()).expect("Police invalide")
    })
}

fn lucksack_replay_path_cache() -> &'static RwLock<HashMap<u64, PathBuf>> {
    LUCKSACK_REPLAY_PATH_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn lucksack_matches_cache_key(matches: &[LucksackMatch]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();

    for m in matches.iter().take(6) {
        m.won.hash(&mut hasher);
        m.had_first_pick.hash(&mut hasher);
        m.battle_time.hash(&mut hasher);

        m.my_monsters.hash(&mut hasher);
        m.my_leader.hash(&mut hasher);
        m.my_bans.hash(&mut hasher);
        m.my_username.hash(&mut hasher);
        m.my_score.hash(&mut hasher);

        m.opponent_monsters.hash(&mut hasher);
        m.opponent_leader.hash(&mut hasher);
        m.opponent_bans.hash(&mut hasher);
        m.opponent_username.hash(&mut hasher);
        m.opponent_score.hash(&mut hasher);
    }

    hasher.finish()
}

pub async fn create_replay_image(
    recent_replays: Vec<Replay>,
    rows: i32,
    cols: i32,
) -> Result<PathBuf> {
    let nb_battles = recent_replays.len();

    let mut sections: Vec<RgbaImage> = Vec::new();
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

        let left_text = if battle.player_one.player_score == 0 {
            battle.player_one.player_name.clone()
        } else {
            format!(
                "{} - {}",
                battle.player_one.player_score, battle.player_one.player_name
            )
        };

        let right_text = if battle.player_two.player_score == 0 {
            battle.player_two.player_name.clone()
        } else {
            format!(
                "{} - {}",
                battle.player_two.player_name, battle.player_two.player_score
            )
        };

        let date_text = NaiveDateTime::parse_from_str(&battle.date, "%Y-%m-%d %H:%M:%S")
            .map(|dt| dt.date().format("%d-%m-%Y").to_string())
            .unwrap_or_else(|_| battle.date.clone());

        let match_banner = create_match_banner(
            &left_text,
            &date_text,
            &right_text,
            combined_width,
            Rgba([0, 0, 0, 0]),
        );
        let banner_height = match_banner.height();

        let border_thickness = 10;
        let section_inner_width = combined_width;
        let section_inner_height = banner_height + final_image.height();

        let section_total_width = section_inner_width + 2 * border_thickness;
        let section_total_height = section_inner_height + 2 * border_thickness;

        let bg_color = match battle.status {
            1 => Rgba([0, 255, 0, 100]),
            2 => Rgba([255, 0, 0, 100]),
            _ => Rgba([0, 0, 0, 100]),
        };

        let mut section =
            ImageBuffer::from_pixel(section_total_width, section_total_height, bg_color);

        let mut inner = ImageBuffer::from_pixel(
            section_inner_width,
            section_inner_height,
            Rgba([0, 0, 0, 0]),
        );

        inner.copy_from(&match_banner, 0, 0).unwrap();
        inner.copy_from(&final_image, 0, banner_height).unwrap();

        section
            .copy_from(&inner, border_thickness, border_thickness)
            .unwrap();
        sections.push(section);
    }

    let rows = rows as u32;
    let cols = cols as u32;
    let padding = 10;

    let section_width = sections.first().map(|img| img.width()).unwrap_or(0);
    let section_height = sections.first().map(|img| img.height()).unwrap_or(0);

    let full_width = cols * section_width + (cols - 1) * padding;
    let full_height = rows * section_height + (rows - 1) * padding;

    let mut final_image = ImageBuffer::new(full_width, full_height);

    for (i, section) in sections.iter().enumerate() {
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;

        let x = col * (section_width + padding);
        let y = row * (section_height + padding);

        final_image.copy_from(section, x, y)?;
    }

    // IMPORTANT: Save to /tmp and keep the filename replay.png to match "attachment://replay.png"
    let output_path = PathBuf::from("/tmp/replay.png");

    let output_path_clone = output_path.clone();
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all("/tmp")?;
        final_image.save(&output_path_clone)?;
        Ok::<_, anyhow::Error>(output_path_clone)
    })
    .await??;

    Ok(output_path)
}

pub async fn create_lucksack_replay_image(matches: &[LucksackMatch]) -> Result<PathBuf> {
    let key = lucksack_matches_cache_key(matches);
    let latest_path = PathBuf::from("/tmp/replay.png");

    let cached_path = if let Ok(read_guard) = lucksack_replay_path_cache().read() {
        read_guard.get(&key).cloned()
    } else {
        None
    };

    if let Some(cached_path) = cached_path.filter(|p| p.exists()) {
        let latest_path_clone = latest_path.clone();
        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all("/tmp")?;
            std::fs::copy(&cached_path, &latest_path_clone)?;
            Ok::<_, anyhow::Error>(latest_path_clone)
        })
        .await??;
        return Ok(latest_path);
    }

    let img_map = get_comid_to_image_map();
    let mut sections: Vec<RgbaImage> = Vec::new();
    let mut image_cache: HashMap<String, DynamicImage> = HashMap::new();

    for m in matches.iter().take(6) {
        let filenames_mine: Vec<String> = m
            .my_monsters
            .iter()
            .filter_map(|&id| img_map.get(&(id as i32)).cloned())
            .collect();
        let filenames_opp: Vec<String> = m
            .opponent_monsters
            .iter()
            .filter_map(|&id| img_map.get(&(id as i32)).cloned())
            .collect();

        if filenames_mine.is_empty() || filenames_opp.is_empty() {
            continue;
        }

        let ids_mine: Vec<u32> = m.my_monsters.iter().map(|&id| id as u32).collect();
        let ids_opp: Vec<u32> = m.opponent_monsters.iter().map(|&id| id as u32).collect();

        let img_me = create_team_collage_custom_layout(
            &filenames_mine,
            &ids_mine,
            m.my_bans as u32,
            m.my_leader as u32,
            m.had_first_pick,
            &mut image_cache,
        )
        .await?;

        let img_opp = create_team_collage_custom_layout(
            &filenames_opp,
            &ids_opp,
            m.opponent_bans as u32,
            m.opponent_leader as u32,
            !m.had_first_pick,
            &mut image_cache,
        )
        .await?;

        let image_width = img_me.width() / 3;
        let spacing = image_width / 2;
        let combined_width = img_me.width() + img_opp.width() + spacing;
        let height = img_me.height().max(img_opp.height());

        let mut combined = ImageBuffer::new(combined_width, height);
        combined.copy_from(&img_me, 0, 0).unwrap();
        combined
            .copy_from(&img_opp, img_me.width() + spacing, 0)
            .unwrap();

        // Parse the match timestamp → "DD/MM - HH:MM"
        let time_text = chrono::DateTime::parse_from_rfc3339(&m.battle_time)
            .map(|dt| dt.format("%d/%m - %H:%M").to_string())
            .unwrap_or_else(|_| m.battle_time.get(..16).unwrap_or("").replace('-', "/"));

        let left_text = format!("{} - {}", m.my_score, m.my_username);
        let right_text = format!("{} - {}", m.opponent_username, m.opponent_score);

        let banner = create_match_banner(
            &left_text,
            &time_text,
            &right_text,
            combined_width,
            Rgba([0, 0, 0, 0]),
        );
        let banner_height = banner.height();

        let border = 10u32;
        let inner_w = combined_width;
        let inner_h = banner_height + combined.height();
        let total_w = inner_w + 2 * border;
        let total_h = inner_h + 2 * border;

        let bg_color = if m.won {
            Rgba([0, 255, 0, 100])
        } else {
            Rgba([255, 0, 0, 100])
        };

        let mut section = ImageBuffer::from_pixel(total_w, total_h, bg_color);
        let mut inner = ImageBuffer::from_pixel(inner_w, inner_h, Rgba([0, 0, 0, 0]));
        inner.copy_from(&banner, 0, 0).unwrap();
        inner.copy_from(&combined, 0, banner_height).unwrap();
        section.copy_from(&inner, border, border).unwrap();
        sections.push(section);
    }

    if sections.is_empty() {
        return Err(anyhow!("No valid matches to render"));
    }

    let cols = 2u32;
    let rows = (sections.len() as u32).div_ceil(cols);
    let padding = 10u32;
    let sw = sections[0].width();
    let sh = sections[0].height();
    let full_w = cols * sw + (cols - 1) * padding;
    let full_h = rows * sh + (rows - 1) * padding;

    let mut canvas = ImageBuffer::new(full_w, full_h);
    for (i, section) in sections.iter().enumerate() {
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        canvas.copy_from(section, col * (sw + padding), row * (sh + padding))?;
    }

    let output_path = PathBuf::from(format!("/tmp/replay-{}.png", key));
    let output_path_clone = output_path.clone();
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all("/tmp")?;
        canvas.save(&output_path_clone)?;
        Ok::<_, anyhow::Error>(output_path_clone)
    })
    .await??;

    let output_path_clone = output_path.clone();
    let latest_path_clone = latest_path.clone();
    tokio::task::spawn_blocking(move || {
        std::fs::copy(&output_path_clone, &latest_path_clone)?;
        Ok::<_, anyhow::Error>(())
    })
    .await??;

    if let Ok(mut write_guard) = lucksack_replay_path_cache().write() {
        if write_guard.len() >= 128 {
            write_guard.clear();
        }
        write_guard.insert(key, output_path.clone());
    }

    Ok(latest_path)
}

async fn create_team_collage_custom_layout(
    image_filenames: &[String],
    monster_ids: &[u32],
    ban_id: u32,
    leader_id: u32,
    first_pick: bool,
    cache: &mut HashMap<String, DynamicImage>,
) -> Result<RgbaImage> {
    let mut images = Vec::new();
    for filename in image_filenames {
        let img = load_image_local(filename, cache).await?;
        images.push(img);
    }

    let width = images[0].width();
    let height = images[0].height();
    let mut collage = ImageBuffer::new(width * 3, height * 2);

    let cross = if width == 100 && height == 100 {
        cross_image_100().clone()
    } else {
        image::imageops::resize(
            cross_image_100(),
            width,
            height,
            image::imageops::FilterType::Triangle,
        )
    };

    let mut grid_slots = [(0, 0); 5];

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

    if let Ok(read_guard) = global_monster_image_cache().read() {
        if let Some(img) = read_guard.get(filename) {
            let cloned = img.clone();
            cache.insert(filename.to_string(), cloned.clone());
            return Ok(cloned);
        }
    }

    let path = format!("assets/monster_images/{}", filename);
    let filename_string = filename.to_string();

    let img: DynamicImage;

    let local_result = tokio::task::spawn_blocking(move || -> Result<DynamicImage> {
        let data = std::fs::read(&path)
            .with_context(|| format!("Failed to read image file: {}", filename_string))?;
        image::load_from_memory(&data)
            .with_context(|| format!("Failed to decode image: {}", filename_string))
    })
    .await;

    match local_result {
        Ok(Ok(image)) => img = image,
        Ok(Err(e))
            if e.downcast_ref::<std::io::Error>().map(|io| io.kind())
                == Some(std::io::ErrorKind::NotFound) =>
        {
            let url = format!(
                "https://swarfarm.com/static/herders/images/monsters/{}",
                filename
            );
            let bytes = http_client()
                .get(&url)
                .send()
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
            return Err(anyhow!(
                "Blocking task failed for file: {}: {}",
                filename,
                e
            ))
        }
    }

    let img = img.resize_exact(100, 100, image::imageops::FilterType::Triangle);

    if let Ok(mut write_guard) = global_monster_image_cache().write() {
        write_guard.insert(filename.to_string(), img.clone());
    }

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

    let font = banner_font();

    let scale = PxScale::from(26.0);
    let margin = 8.0_f32;
    let width_f = width as f32;

    let max_left_width = width_f / 3.0 - margin * 2.0;
    let max_center_width = width_f / 3.0 - margin * 2.0;
    let max_right_width = width_f / 3.0 - margin * 2.0;

    let left = fit_text_to_width(font, scale, left_text, max_left_width);
    let center = fit_text_to_width(font, scale, center_text, max_center_width);
    let right = fit_text_to_width(font, scale, right_text, max_right_width);

    let center_w = text_width(font, scale, &center);
    let right_w = text_width(font, scale, &right);

    let y = 8;

    draw_bold_text_mut(
        &mut image,
        Rgba([255, 255, 255, 255]),
        margin as i32,
        y,
        scale,
        font,
        &left,
    );

    let center_x: i32 = ((width_f - center_w) / 2.0).round() as i32;
    draw_bold_text_mut(
        &mut image,
        Rgba([255, 255, 255, 255]),
        center_x,
        y,
        scale,
        font,
        &center,
    );

    let right_x: i32 = (width_f - right_w - margin).round() as i32;
    draw_bold_text_mut(
        &mut image,
        Rgba([255, 255, 255, 255]),
        right_x,
        y,
        scale,
        font,
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
    let offsets = [(0, 0), (1, 0), (0, 1), (1, 1)];
    for (dx, dy) in offsets {
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
    let s = text.to_string();

    if text_width(font, scale, &s) <= max_width {
        return s;
    }

    let mut chars: Vec<char> = s.chars().collect();
    while !chars.is_empty()
        && text_width(font, scale, &chars.iter().collect::<String>()) > max_width
    {
        chars.pop();
    }

    let mut result: String = chars.iter().collect();
    if !result.is_empty() {
        result.push('…');
    }
    result
}

pub fn parse_discord_mention_to_id(s: &str) -> Option<u64> {
    // Supporte: <@123> et <@!123>
    let s = s.trim();

    if !s.starts_with("<@") || !s.ends_with('>') {
        return None;
    }

    let inner = &s[2..s.len() - 1]; // retire <@ et >
    let inner = inner.strip_prefix('!').unwrap_or(inner);

    inner.parse::<u64>().ok()
}

/* ------------------ Lucksack API types ------------------ */

#[derive(Debug, Deserialize)]
pub struct LucksackSearchPlayer {
    pub player_id: i64,
    pub username: String,
    pub country: String,
    pub current_score: Option<i32>,
    pub current_rank: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LucksackSeasonEntry {
    pub season_number: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct LucksackPlayerSummary {
    pub user_info: LucksackUserInfo,
    pub summary: LucksackSummaryData,
}

#[derive(Debug, Deserialize)]
pub struct LucksackUserInfo {
    pub player_id: i64,
    pub server_id: i32,
    pub username: String,
    pub country: String,
    pub image: String,
}

#[derive(Debug, Deserialize)]
pub struct LucksackSummaryData {
    pub total_matches: i32,
    pub overall_win_rate: f64,
    pub peak_score: i32,
    pub current_score: i32,
    pub current_rank: i64,
    pub current_rank_bracket: i32,
    pub score_last_3_days: i32,
    pub score_last_7_days: i32,
}

/* ------------------ Lucksack API calls ------------------ */

pub async fn search_players_lucksack(username: &str) -> Result<Vec<LucksackSearchPlayer>> {
    let url = format!("https://api.lucksack.gg/players/search/{}", username);
    let res = http_client()
        .get(&url)
        .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .header("sec-fetch-site", "none")
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send request: {}", e))?;

    if !res.status().is_success() {
        return Err(anyhow!("HTTP {}", res.status()));
    }

    res.json::<Vec<LucksackSearchPlayer>>()
        .await
        .map_err(|e| anyhow!("Failed to parse search JSON: {}", e))
}

pub async fn get_lucksack_season_numbers() -> Result<Vec<i32>> {
    let url = "https://api.lucksack.gg/seasons";
    let res = http_client()
        .get(url)
        .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .header("sec-fetch-site", "none")
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send seasons request: {}", e))?;

    if !res.status().is_success() {
        return Err(anyhow!("HTTP {}", res.status()));
    }

    let seasons = res
        .json::<Vec<LucksackSeasonEntry>>()
        .await
        .map_err(|e| anyhow!("Failed to parse seasons JSON: {}", e))?;

    let mut season_numbers: Vec<i32> = seasons
        .into_iter()
        .filter_map(|s| s.season_number)
        .collect();
    season_numbers.sort_unstable_by(|a, b| b.cmp(a));
    season_numbers.dedup();

    if season_numbers.is_empty() {
        return Err(anyhow!("No valid season_number found"));
    }

    Ok(season_numbers)
}

pub async fn get_lucksack_player_summary(
    player_id: i64,
    season: i32,
) -> Result<LucksackPlayerSummary> {
    let url = format!(
        "https://api.lucksack.gg/players/{}/summary?season={}",
        player_id, season
    );
    let res = http_client()
        .get(&url)
        .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .header("sec-fetch-site", "none")
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send request: {}", e))?;

    if !res.status().is_success() {
        return Err(anyhow!("HTTP {}", res.status()));
    }

    res.json::<LucksackPlayerSummary>()
        .await
        .map_err(|e| anyhow!("Failed to parse summary JSON: {}", e))
}

/* ------------------ Lucksack rank emojis ------------------ */

pub fn get_rank_emojis_for_bracket(bracket: i32) -> String {
    let conqueror = format!("<:conqueror:{}>", CONQUEROR_EMOJI_ID.lock().unwrap());
    let punisher = format!("<:punisher:{}>", PUNISHER_EMOJI_ID.lock().unwrap());
    let guardian = format!("<:guardian:{}>", GUARDIAN_EMOJI_ID.lock().unwrap());

    match bracket {
        8 => conqueror.to_string(),
        9 => conqueror.repeat(2),
        10 => conqueror.repeat(3),
        11 => punisher.to_string(),
        12 => punisher.repeat(2),
        13 => punisher.repeat(3),
        14 => guardian.to_string(),
        15 => guardian.repeat(2),
        16 => guardian.repeat(3),
        _ => "Unranked".to_string(),
    }
}

/* ------------------ Lucksack picks types & API ------------------ */

#[derive(Debug, Deserialize)]
pub struct LucksackPickEntry {
    pub monster_id: i64,
    pub played_count: i32,
    pub win_rate: f64,
}

#[derive(Debug, Deserialize)]
pub struct LucksackBoxEntry {
    pub monster_image: String,
    pub played_count: i32,
}

pub async fn get_lucksack_player_picks(
    player_id: i64,
    season: i32,
) -> Result<Vec<LucksackPickEntry>> {
    let url = format!(
        "https://api.lucksack.gg/players/{}/picks?season={}&min_game_played=3",
        player_id, season
    );
    let res = http_client()
        .get(&url)
        .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .header("sec-fetch-site", "none")
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send picks request: {}", e))?;

    if !res.status().is_success() {
        return Err(anyhow!("HTTP {}", res.status()));
    }

    res.json::<Vec<LucksackPickEntry>>()
        .await
        .map_err(|e| anyhow!("Failed to parse picks JSON: {}", e))
}

pub async fn get_lucksack_player_ld5_box(
    player_id: i64,
    season: i32,
) -> Result<Vec<LucksackBoxEntry>> {
    let url = format!(
        "https://api.lucksack.gg/players/{}/box?season={}&element=light%2Cdark&ld5=true",
        player_id, season
    );
    let res = http_client()
        .get(&url)
        .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .header("sec-fetch-site", "none")
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send LD5 box request: {}", e))?;

    if !res.status().is_success() {
        return Err(anyhow!("HTTP {}", res.status()));
    }

    res.json::<Vec<LucksackBoxEntry>>()
        .await
        .map_err(|e| anyhow!("Failed to parse LD5 box JSON: {}", e))
}

/// com2us_id → image_filename (from monsters_elements.json)
static COMID_TO_IMAGE: OnceLock<HashMap<i32, String>> = OnceLock::new();

fn get_comid_to_image_map() -> &'static HashMap<i32, String> {
    COMID_TO_IMAGE.get_or_init(|| {
        let file =
            fs::read_to_string("monsters_elements.json").expect("monsters_elements.json not found");
        let v: Value = serde_json::from_str(&file).expect("invalid monsters_elements.json");
        let arr = v["monsters"].as_array().expect("monsters must be an array");

        let mut map = HashMap::new();
        for m in arr {
            let com2us_id = m["com2us_id"].as_i64().unwrap_or(0) as i32;
            let filename = m["image_filename"].as_str().unwrap_or("").to_string();
            if com2us_id != 0 && !filename.is_empty() {
                map.insert(com2us_id, filename);
            }
        }
        map
    })
}

pub async fn format_lucksack_top_monsters(picks: &[LucksackPickEntry]) -> String {
    let Ok(collection) = get_mob_emoji_collection().await else {
        return String::new();
    };

    let img_map = get_comid_to_image_map();
    let mut lines = vec![];

    for (idx, pick) in picks.iter().take(10).enumerate() {
        let monster_id = pick.monster_id as i32;

        let emoji_str = if let Some(image_filename) = img_map.get(&monster_id) {
            // "unit_icon_0168_1_5.png" → "0168_1_5"
            let emoji_name = image_filename
                .trim_start_matches("unit_icon_")
                .trim_end_matches(".png");

            let doc = collection
                .find_one(doc! { "name": emoji_name })
                .await
                .ok()
                .flatten();

            if let Some(d) = doc {
                let id = d.get_str("id").unwrap_or("");
                let name = d.get_str("name").unwrap_or("unit");
                format!("<:{}:{}>", name, id)
            } else {
                "❓".to_string()
            }
        } else {
            "❓".to_string()
        };

        lines.push(format!(
            "{}. {} {} picks, **{:.1}%** WR",
            idx + 1,
            emoji_str,
            pick.played_count,
            pick.win_rate
        ));
    }

    if lines.is_empty() {
        "No data".to_string()
    } else {
        lines.join("\n")
    }
}

pub async fn format_lucksack_ld_monsters_emojis(ld_box: &[LucksackBoxEntry]) -> String {
    let Ok(collection) = get_mob_emoji_collection().await else {
        return "No data".to_string();
    };

    let mut emojis = Vec::new();
    let mut seen = HashSet::new();

    for monster in ld_box {
        if monster.played_count <= 0 {
            continue;
        }

        let emoji_name = monster
            .monster_image
            .trim_start_matches("unit_icon_")
            .trim_end_matches(".png");

        if emoji_name.is_empty() || !seen.insert(emoji_name.to_string()) {
            continue;
        }

        let doc = collection
            .find_one(doc! { "name": emoji_name })
            .await
            .ok()
            .flatten();

        if let Some(d) = doc {
            let id = d.get_str("id").unwrap_or("");
            let name = d.get_str("name").unwrap_or("unit");
            if !id.is_empty() {
                emojis.push(format!("<:{}:{}>", name, id));
            }
        }
    }

    if emojis.is_empty() {
        "No data".to_string()
    } else {
        emojis.join(" ")
    }
}

/* ------------------ Lucksack embed builder ------------------ */

pub fn create_lucksack_player_embed(
    summary: &LucksackPlayerSummary,
    rank_emojis: String,
    top_monsters: String,
    ld_monsters: String,
) -> CreateEmbed {
    let s = &summary.summary;
    let info = &summary.user_info;

    let _rank_label = match s.current_rank_bracket {
        11 => "P1",
        12 => "P2",
        13 => "P3",
        14 => "G1",
        15 => "G2",
        16 => "G3",
        _ => "?",
    };

    let server_name = match info.server_id {
        1 => "Korea",
        2 => "Japan",
        3 => "China",
        4 => "Global",
        5 => "Asia",
        6 => "Europe",
        _ => "??",
    };

    let alias_suffix = PLAYER_ALIAS_MAP
        .get(&info.player_id)
        .map(|alias| format!(" (aka. {})", alias))
        .unwrap_or_default();
    let display_name = if alias_suffix.is_empty() {
        info.username.clone()
    } else {
        format!("{}{}", info.username, alias_suffix)
    };

    let score_3d = if s.score_last_3_days >= 0 {
        format!("+{}", s.score_last_3_days)
    } else {
        s.score_last_3_days.to_string()
    };

    let score_7d = if s.score_last_7_days >= 0 {
        format!("+{}", s.score_last_7_days)
    } else {
        s.score_last_7_days.to_string()
    };

    let description = format!(
        "**Elo**: {} • **Rank**: #{} • **Peak Elo**: {}",
        s.current_score, s.current_rank, s.peak_score
    );
    let lucksack_profile_url = format!("https://lucksack.gg/player/{}", info.player_id);

    let split_field_chunks = |text: &str, max_len: usize| -> Vec<String> {
        if text.is_empty() {
            return vec!["No data".to_string()];
        }

        if text.len() <= max_len {
            return vec![text.to_string()];
        }

        let mut chunks = Vec::new();
        let mut current = String::new();

        for token in text.split_whitespace() {
            let candidate_len = if current.is_empty() {
                token.len()
            } else {
                current.len() + 1 + token.len()
            };

            if candidate_len <= max_len {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(token);
            } else {
                if !current.is_empty() {
                    chunks.push(current);
                    current = String::new();
                }

                if token.len() <= max_len {
                    current.push_str(token);
                } else {
                    let mut start = 0usize;
                    while start < token.len() {
                        let mut end = (start + max_len).min(token.len());
                        while end > start && !token.is_char_boundary(end) {
                            end -= 1;
                        }
                        if end == start {
                            break;
                        }
                        chunks.push(token[start..end].to_string());
                        start = end;
                    }
                }
            }
        }

        if !current.is_empty() {
            chunks.push(current);
        }

        if chunks.is_empty() {
            vec!["No data".to_string()]
        } else {
            chunks
        }
    };

    let ld_chunks = split_field_chunks(&ld_monsters, 1020);

    let mut embed = CreateEmbed::default()
        .title(format!(
            ":flag_{}: {} (id: {}) - RTA Statistics",
            info.country.to_lowercase(),
            display_name,
            info.player_id,
        ))
        .thumbnail(info.image.clone())
        .color(serenity::Colour::from_rgb(0, 180, 255))
        .description(description)
        .field(
            "Lucksack Profile",
            format!("[Open profile]({})", lucksack_profile_url),
            false,
        )
        .field("Win Rate", format!("{:.2}%", s.overall_win_rate), true)
        .field("Total Matches", s.total_matches.to_string(), true)
        .field("Server", server_name, true)
        .field("Rank Bracket", rank_emojis, true)
        .field("Elo (3 days)", score_3d, true)
        .field("Elo (7 days)", score_7d, true);

    for (idx, chunk) in ld_chunks.into_iter().enumerate() {
        let name = if idx == 0 {
            "✨ LD Monsters (RTA only)".to_string()
        } else {
            format!("✨ LD Monsters (RTA only) ({})", idx + 1)
        };
        embed = embed.field(name, chunk, false);
    }

    embed
        .field("🏆 Top Played Monsters", top_monsters, false)
        .footer(CreateEmbedFooter::new("Data is gathered from lucksack.gg"))
}

/* ------------------ Lucksack matches types & API ------------------ */

#[derive(Debug, Deserialize)]
pub struct LucksackMatchesResponse {
    pub matches: Vec<LucksackMatch>,
}

#[derive(Debug, Deserialize)]
pub struct LucksackMatch {
    pub won: bool,
    pub had_first_pick: bool,
    pub battle_time: String,
    pub my_monsters: Vec<i64>,
    pub my_leader: i64,
    pub my_bans: i64,
    pub my_username: String,
    pub my_score: i32,
    pub opponent_monsters: Vec<i64>,
    pub opponent_leader: i64,
    pub opponent_bans: i64,
    pub opponent_username: String,
    pub opponent_score: i32,
}

pub async fn get_lucksack_player_matches(
    player_id: i64,
    season: i32,
) -> Result<Vec<LucksackMatch>> {
    let url = format!(
        "https://api.lucksack.gg/players/{}/matches?season={}&limit=6&offset=0",
        player_id, season
    );
    let res = http_client()
        .get(&url)
        .header("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .header("sec-fetch-site", "none")
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send matches request: {}", e))?;

    if !res.status().is_success() {
        return Err(anyhow!("HTTP {}", res.status()));
    }

    let resp = res
        .json::<LucksackMatchesResponse>()
        .await
        .map_err(|e| anyhow!("Failed to parse matches JSON: {}", e))?;

    Ok(resp.matches)
}
