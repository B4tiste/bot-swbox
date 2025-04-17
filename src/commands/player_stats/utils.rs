use crate::commands::ranks::utils::get_rank_info;
use crate::commands::shared::player_alias::PLAYER_ALIAS_MAP;
use crate::MONGO_URI;
use ab_glyph::{FontArc, PxScale};
use anyhow::{anyhow, Result};
use image::GenericImage;
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use mongodb::{bson::doc, Client, Collection};
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};
use std::collections::HashMap;
use std::path::PathBuf;

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
    player_one: ReplayPlayer,

    #[serde(rename = "playerTwo")]
    player_two: ReplayPlayer,

    #[serde(rename = "firstPick")]
    first_pick: u32,

    #[serde(rename = "status")]
    status: u32,
}

#[derive(Debug, Deserialize)]
struct ReplayPlayer {
    #[serde(rename = "monsterInfoList")]
    monster_info_list: Vec<ReplayMonster>,

    #[serde(rename = "banMonsterId")]
    ban_monster_id: u32,

    #[serde(rename = "leaderMonsterId")]
    leader_monster_id: u32,

    #[serde(rename = "playerId")]
    player_id: u32,

    #[serde(rename = "playerName")]
    player_name: String,
}

#[derive(Debug, Deserialize)]
struct ReplayMonster {
    #[serde(rename = "imageFilename")]
    image_filename: String,

    #[serde(rename = "monsterId")]
    monster_id: u32,
}

// #[derive(Debug, Deserialize)]
// pub struct ReplayMonster {
//     #[serde(rename = "imageFilename")]
//     pub image_filename: String,
//     #[serde(rename = "monsterId")]
//     pub monster_id: i32,
// }

// #[derive(Debug, Deserialize)]
// pub struct ReplayPlayer {
//     #[serde(rename = "swrtPlayerId")]
//     pub swrt_player_id: i64,
//     #[serde(rename = "monsterInfoList")]
//     pub monster_info_list: Vec<ReplayMonster>,
//     #[serde(rename = "playerCountry")]
//     pub player_country: String,
//     #[serde(rename = "playerName")]
//     pub name: String,
//     #[serde(rename = "leaderMonsterId")]
//     pub leader_monster_id: i32,
//     #[serde(rename = "banMonsterId")]
//     pub ban_monster_id: Option<i32>,
//     #[serde(rename = "playerId")]
//     pub player_id: i64,
// }

// #[derive(Debug, Deserialize)]
// pub struct Replay {
//     #[serde(rename = "playerOne")]
//     pub player_one: ReplayPlayer,
//     #[serde(rename = "playerTwo")]
//     pub player_two: ReplayPlayer,
//     #[serde(rename = "status")]
//     pub replay_type: i32,
//     #[serde(rename = "firstPick")]
//     pub first_pick: i64,
// }

// #[derive(Debug, Deserialize)]
// struct ReplayListData {
//     list: Vec<Replay>,
// }

// #[derive(Debug, Deserialize)]
// struct ReplayListWrapper {
//     page: ReplayListData,
// }

// #[derive(Debug, Deserialize)]
// struct ReplayListResponse {
//     data: Option<ReplayListWrapper>,
// }

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

pub async fn get_emoji_from_filename_with_stars(
    collection: &Collection<mongodb::bson::Document>,
    filename: &str,
) -> Option<String> {
    let name_no_ext = filename.replace(".png", "").replace("unit_icon_", "");

    let emoji_doc = collection
        .find_one(doc! { "name": &name_no_ext })
        .await
        .ok()??;

    let natural_stars = emoji_doc.get_i32("natural_stars").unwrap_or(0);

    if natural_stars < 5 {
        return None;
    }

    let id = emoji_doc.get_str("id").ok()?;
    Some(format!("<:{}:{}>", name_no_ext, id))
}

pub async fn format_player_ld_monsters_emojis(details: &PlayerDetail) -> Vec<String> {
    let mut emojis = vec![];

    let mut files = vec![];
    if let Some(ld) = &details.monster_ld_imgs {
        files.extend(ld.clone());
    }

    files.sort();
    files.dedup();

    if let Ok(collection) = get_mob_emoji_collection().await {
        for file in files {
            // ici on utilise la fonction avec filtre sur les étoiles
            if let Some(emoji) = get_emoji_from_filename_with_stars(&collection, &file).await {
                emojis.push(emoji);
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

    if let Some(monsters) = &details.player_monsters {
        for (index, m) in monsters.iter().enumerate() {
            if let Some(emoji) = get_emoji_from_filename(&collection, &m.monster_img).await {
                let pick_display = if m.pick_total >= 1000 {
                    let k = m.pick_total / 1000;
                    let remainder = (m.pick_total % 1000) / 100;
                    // if remainder == 0 {
                    //     format!("{}k", k)
                    // } else {
                    //     format!("{}k.{:02}", k, remainder)
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

    output
}

/// Creates an embed for player info without the replay list
pub fn create_player_embed_without_replays(
    details: &PlayerDetail,
    ld_emojis: Vec<String>,
    top_monsters: Vec<String>,
    rank_emojis: String,
) -> CreateEmbed {
    let format_emojis = |mut list: Vec<String>| {
        let mut result = list.join(" ");
        while result.len() > 1020 && !list.is_empty() {
            list.pop();
            result = list.join(" ");
        }
        if result.len() >= 1020 {
            result.push_str(" …");
        }
        if result.is_empty() {
            "None".to_string()
        } else {
            result
        }
    };

    let ld_display = format_emojis(ld_emojis.clone());
    let top_display = format_emojis(top_monsters);

    CreateEmbed::default()
        .title(format!(
            ":flag_{}: {}{} RTA Statistics (Regular Season only)",
            details.player_country.to_lowercase(),
            details.name,
            PLAYER_ALIAS_MAP
                .get(&details.swrt_player_id)
                .map(|alias| format!(" ({})", alias))
                .unwrap_or_default()
        ))
        .thumbnail(details.head_img.clone().unwrap_or_default())
        .color(serenity::Colour::from_rgb(0, 180, 255))
        .description("⚠️ Stats are not 100% accurate ➡️ The very last battle is not included in the elo/rank, and people around 1300 elo will have weird stats (missing games, weird winrates) ⚠️")
        .field("WinRate", format!("{:.1} %", details.win_rate.unwrap_or(0.0) * 100.0), true)
        .field("Elo", details.player_score.unwrap_or(0).to_string(), true)
        .field("Rank", details.player_rank.unwrap_or(0).to_string(), true)
        .field("🏆 Estimation", rank_emojis, true)
        .field("Matches Played", details.season_count.unwrap_or(0).to_string(), true)
        .field("✨ LD Monsters (RTA only)", ld_display, false)
        .field("🔥 Most Used Units Winrate", top_display, false)
        .footer(CreateEmbedFooter::new(
            "Please use /send_suggestion to report any issue.",
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
        "pageSize": 4,
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

pub async fn create_replay_image(recent_replays: Vec<Replay>) -> Result<PathBuf> {
    let base_url = "https://swarfarm.com/static/herders/images/monsters/";

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
            .map(|m| format!("{}{}", base_url, m.image_filename))
            .collect();

        let urls_player_two: Vec<String> = battle
            .player_two
            .monster_info_list
            .iter()
            .map(|m| format!("{}{}", base_url, m.image_filename))
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
        ).await?;

        let img2 = create_team_collage_custom_layout(
            &urls_player_two,
            &monster_ids_two,
            battle.player_two.ban_monster_id,
            battle.player_two.leader_monster_id,
            !is_p1_first_pick,
            &mut image_cache,
        ).await?;

        let image_width = img1.width() / 3;
        let spacing = image_width / 2;
        let combined_width = img1.width() + img2.width() + spacing;
        let height = img1.height().max(img2.height());

        let mut final_image = ImageBuffer::new(combined_width, height);
        final_image.copy_from(&img1, 0, 0).unwrap();
        final_image
            .copy_from(&img2, img1.width() + spacing, 0)
            .unwrap();

        let p1_banner = create_name_banner(
            &battle.player_one.player_name,
            img1.width(),
            "NotoSansCJK-Regular.otf",
            Rgba([0, 0, 0, 0]),
        );
        let p2_banner = create_name_banner(
            &battle.player_two.player_name,
            img2.width(),
            "NotoSansCJK-Regular.otf",
            Rgba([0, 0, 0, 0]),
        );

        let banner_height = p1_banner.height().max(p2_banner.height());
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

        inner.copy_from(&p1_banner, 0, 0).unwrap();
        inner
            .copy_from(&p2_banner, img1.width() + spacing, 0)
            .unwrap();
        inner.copy_from(&final_image, 0, banner_height).unwrap();

        // Intégrer zone intérieure centrée dans la bordure
        section
            .copy_from(&inner, border_thickness, border_thickness)
            .unwrap();

        max_width = max_width.max(section_total_width);
        sections.push(section);
    }

    // Créer l'image finale verticale avec un padding entre chaque section
    let padding_between_sections = 10;
    let total_height: u32 = sections
        .iter()
        .map(|img| img.height() + padding_between_sections)
        .sum::<u32>()
        - padding_between_sections;

    let mut output = ImageBuffer::new(max_width, total_height);

    let mut y_offset = 0;
    for section in sections {
        output.copy_from(&section, 0, y_offset).unwrap();
        y_offset += section.height() + padding_between_sections;
    }

    // Enregistrer l'image finale
    let output_path = PathBuf::from(format!("/tmp/replay_{}.png", "test"));

    let output_path_clone = output_path.clone();
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all("/tmp")?;
        output.save(&output_path_clone)?;
        Ok::<_, anyhow::Error>(output_path_clone)
    })
    .await??;

    Ok(output_path)
}

async fn create_team_collage_custom_layout(
    image_urls: &[String],
    monster_ids: &[u32],
    ban_id: u32,
    leader_id: u32,
    first_pick: bool,
    cache: &mut HashMap<String, DynamicImage>,
) -> Result<RgbaImage> {
    let mut images = Vec::new();
    for url in image_urls {
        let img = download_image_cached(url, cache).await?; // <-- await ici
        images.push(img);
    }

    let width = images[0].width();
    let height = images[0].height();

    // Taille de l’image finale : 4 colonnes pour accueillir le monstre décalé
    let mut collage = ImageBuffer::new(width * 3, height * 2);

    // Charger croix
    let cross = image::open("cross.png")
        .expect("Erreur lors du chargement de cross.png")
        .resize_exact(width, height, image::imageops::FilterType::Lanczos3)
        .to_rgba8();

    // Séparer les monstres
    let mut grid_slots = vec![(0, 0); 5]; // (x,y) pour chaque monstre

    if first_pick {
        // First pick layout
        // M2 M4
        // M3 M5
        // M1 → à gauche, centré verticalement
        grid_slots[1] = (1, 0); // M2
        grid_slots[2] = (1, 1); // M3
        grid_slots[3] = (2, 0); // M4
        grid_slots[4] = (2, 1); // M5
        grid_slots[0] = (0, 1); // M1 (sera décalé manuellement verticalement ensuite)
    } else {
        // Non first pick layout
        // M1 M3
        // M2 M4
        // M5 → à droite, centré verticalement
        grid_slots[0] = (0, 0); // M1
        grid_slots[1] = (0, 1); // M2
        grid_slots[2] = (1, 0); // M3
        grid_slots[3] = (1, 1); // M4
        grid_slots[4] = (2, 1); // M5 (sera décalé verticalement ensuite)
    }

    for (i, (img, &monster_id)) in images.iter().zip(monster_ids).enumerate() {
        let mut rgba = img.to_rgba8();

        // ⭐ Leader encadré vert
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

        // ❌ Banni avec croix
        if monster_id == ban_id {
            image::imageops::overlay(&mut rgba, &cross, 0, 0);
        }

        // 📌 Placement selon grille
        let (grid_x, grid_y) = grid_slots[i];
        let x = grid_x as u32 * width;

        let y = if first_pick && i == 0 {
            // M1 décalé verticalement (first pick)
            ((height * 2) as f32 / 2.0 - (height as f32 / 2.0)).round() as u32
        } else if !first_pick && i == 4 {
            // M5 décalé verticalement (last pick)
            ((height * 2) as f32 / 2.0 - (height as f32 / 2.0)).round() as u32
        } else {
            grid_y as u32 * height
        };

        collage.copy_from(&rgba, x, y).unwrap();
    }

    // retourner l'image finale
    Ok(collage)
}

async fn download_image_cached(
    url: &str,
    cache: &mut HashMap<String, DynamicImage>,
) -> Result<DynamicImage> {
    if let Some(img) = cache.get(url) {
        return Ok(img.clone());
    }

    let bytes = reqwest::get(url).await?.bytes().await?;
    let img = image::load_from_memory(&bytes)?;
    cache.insert(url.to_string(), img.clone());
    Ok(img)
}

fn create_name_banner(text: &str, width: u32, font_path: &str, color: Rgba<u8>) -> RgbaImage {
    let height = 40;
    let mut image = ImageBuffer::from_pixel(width, height, color); // Couleur de fond (semi-transparente)

    let font_data = std::fs::read(font_path).expect("Erreur lecture police");
    let font = FontArc::try_from_vec(font_data).expect("Police invalide");

    let scale = PxScale::from(26.0);
    let text_width = (text.len() as u32 * 14).min(width);
    let x = ((width - text_width).max(0)) as i32 / 2;
    let y = 8;

    draw_bold_text_mut(
        &mut image,
        Rgba([255, 255, 255, 255]),
        x,
        y,
        scale,
        &font,
        text,
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
