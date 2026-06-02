use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use anyhow::{anyhow, Context, Result};
use chrono::NaiveDateTime;
use image::GenericImage;
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use poise::serenity_prelude as serenity;
use reqwest::Client;
use serde::Deserialize;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use crate::commands::replays::models::Root;
use crate::commands::shared::clients::http_client;

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

static GLOBAL_MONSTER_IMAGE_CACHE: OnceLock<RwLock<HashMap<String, DynamicImage>>> =
    OnceLock::new();
static CROSS_IMAGE_100: OnceLock<RgbaImage> = OnceLock::new();
static BANNER_FONT: OnceLock<FontArc> = OnceLock::new();
fn global_monster_image_cache() -> &'static RwLock<HashMap<String, DynamicImage>> {
    GLOBAL_MONSTER_IMAGE_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn cross_image_100() -> &'static RgbaImage {
    CROSS_IMAGE_100.get_or_init(|| {
        const CROSS_BYTES: &[u8] = include_bytes!("../player_stats/cross.png");
        image::load_from_memory(CROSS_BYTES)
            .expect("Erreur lors du chargement de cross.png")
            .resize_exact(100, 100, image::imageops::FilterType::Triangle)
            .to_rgba8()
    })
}

fn banner_font() -> &'static FontArc {
    BANNER_FONT.get_or_init(|| {
        const FONT_BYTES: &[u8] = include_bytes!("../player_stats/NotoSansCJK-Regular.otf");
        FontArc::try_from_vec(FONT_BYTES.to_vec()).expect("Police invalide")
    })
}

pub async fn get_replays_data(ids: &Vec<i32>, level: i32) -> Result<Vec<Replay>> {
    let url = "https://m.swranking.com/api/player/replayallist";
    let client = Client::new();

    let body = serde_json::json!({
        "pageNum":1,
        "pageSize":16,
        "level": level,
        "monsterIds": ids,
    });

    let res = client
        .post(url)
        .json(&body)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let status = res.status();
    let json: Root = res.json().await?;

    if !status.is_success() {
        return Err(anyhow!("Error status {}: {:?}", status, json.data.list));
    }

    // If playerOne does have the monster in the ids, swap playerOne with playerTwo
    let mut replays = json.data.list;
    for replay in &mut replays {
        if replay
            .player_one
            .monster_info_list
            .iter()
            .any(|m| ids.contains(&(m.monster_id as i32)))
        {
            // Player one has the monster, no need to swap
            continue;
        } else if replay
            .player_two
            .monster_info_list
            .iter()
            .any(|m| ids.contains(&(m.monster_id as i32)))
        {
            // Player two has the monster, swap players
            std::mem::swap(&mut replay.player_one, &mut replay.player_two);
            // if replay.status is 1, set it to 2 else if replay.status is 2, set it to 1
            if replay.status == 1 {
                replay.status = 2;
            } else if replay.status == 2 {
                replay.status = 1;
            }
        }
    }
    // Return the list of replays
    Ok(replays)
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

pub fn create_replays_embed(
    monster_names: &[String],
    level: i32,
    player_names: &[String],
) -> CreateEmbed {
    let level_str = match level {
        1 => "G1-G2",
        3 => "G3",
        4 => "P1-P3",
        _ => "Unknown",
    };

    let description = if monster_names.len() == 1 {
        format!(
            "Recent replays for **{}** - **Level**: {}",
            monster_names[0], level_str
        )
    } else {
        let monsters_list = monster_names
            .iter()
            .map(|name| format!("• **{}**", name))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "Recent replays for:\n{}\n\n**Level**: {}",
            monsters_list, level_str
        )
    };

    // Construire la chaîne des joueurs avec format en liste :
    /*
    - `PLAYER1`
    - `PLAYER2`
    - `PLAYER3`
    */
    let players_field = if player_names.is_empty() {
        "None".to_string()
    } else {
        player_names
            .iter()
            .map(|name| format!("• `{}`", name))
            .collect::<Vec<_>>()
            .join("\n")
    };

    CreateEmbed::default()
        .title("🎬 Replays")
        .description(description)
        .color(serenity::Colour::from_rgb(0, 123, 255)) // Bleu
        .image("attachment://replay.png")
        .field("Players", players_field, false) // ← insertion du champ
        .field(
            "ℹ️ Tip",
            "Use the buttons below to view stats for different RTA ranks (P1-P3, G1-G2, G3).",
            false,
        )
        .footer(CreateEmbedFooter::new(
            "Data is gathered from m.swranking.com",
        ))
}

pub fn create_loading_replays_embed(monster_names: &[String], level: i32) -> CreateEmbed {
    let level_str = match level {
        1 => "G1-G2",
        3 => "G3",
        4 => "P1-P3",
        _ => "Unknown",
    };

    let description = if monster_names.len() == 1 {
        format!(
            "Loading replays for **{}** - **Level**: {}",
            monster_names[0], level_str
        )
    } else {
        let monsters_list = monster_names
            .iter()
            .map(|name| format!("• **{}**", name))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "Loading replays for:\n{}\n\n**Level**: {}",
            monsters_list, level_str
        )
    };

    CreateEmbed::default()
        .title("🎬 Replays")
        .description(description)
        .color(serenity::Colour::from_rgb(255, 165, 0)) // Orange pour le chargement
        .field(
            "Status",
            "<a:loading:1358029412716515418> Loading new replay data...",
            false,
        )
        .footer(CreateEmbedFooter::new(
            "Please wait while we fetch the replay data...",
        ))
}

pub fn create_replay_level_buttons(
    guardian_id: u64,
    punisher_id: u64,
    selected_level: i32,
    disabled: bool,
) -> serenity::CreateActionRow {
    let style_for = |level| {
        if level == selected_level {
            serenity::ButtonStyle::Primary
        } else {
            serenity::ButtonStyle::Secondary
        }
    };

    serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new("level_p1p3")
            .label("P1-P3")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: punisher_id.into(),
                name: Some("punisher".to_string()),
            })
            .style(style_for(4)),
        serenity::CreateButton::new("level_g1g2")
            .label("G1-G2")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: guardian_id.into(),
                name: Some("guardian".to_string()),
            })
            .style(style_for(1)),
        serenity::CreateButton::new("level_g3")
            .label("G3")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: guardian_id.into(),
                name: Some("guardian".to_string()),
            })
            .style(style_for(3)),
    ])
}
