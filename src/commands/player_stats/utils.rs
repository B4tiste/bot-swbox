use crate::commands::ranks::utils::get_rank_info;
use crate::commands::shared::player_alias::PLAYER_ALIAS_MAP;
use crate::MONGO_URI;
use ab_glyph::{FontArc, PxScale};
use anyhow::{anyhow, Context, Result};
use image::GenericImage;
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use mongodb::{bson::doc, Client, Collection};
use poise::serenity_prelude as serenity;
use rand::seq::IndexedRandom;
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

    #[serde(rename = "playerScore")]
    player_score: u32,
}

#[derive(Debug, Deserialize)]
struct ReplayMonster {
    #[serde(rename = "imageFilename")]
    image_filename: String,

    #[serde(rename = "monsterId")]
    monster_id: u32,
}

#[derive(Debug, Deserialize)]
struct EstimationResponse {
    data: EstimationData,
}

#[derive(Debug, Deserialize)]
struct EstimationData {
    one_win_rate: f32,
    two_win_rate: f32,
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
pub fn create_player_embed(
    details: &PlayerDetail,
    ld_emojis: Vec<String>,
    top_monsters: Vec<String>,
    rank_emojis: String,
    has_image: i32,
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

    // Créer une liste de gif qui seront choisis aléatoirement pour l'image de fond
    let gifs = vec![
        "https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExczN3N3YxcjAzc3g5bWpqY2VleXA2MHN0bm9rcDVvaG00MGZrbHoweSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/2WjpfxAI5MvC9Nl8U7/giphy.gif",
        "https://media3.giphy.com/media/v1.Y2lkPTc5MGI3NjExeXRmY2locjR2cnJ5d2JvdWF5djN5cTRlajdna3JxeTA4d2RsdzVxciZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/rGDZbxkkjo0hfLe4EA/giphy.gif",
        "https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExbTRsODVtNThvbTl2bW50NnhzYjB5MWN3aHF5dW40NTIwMmpoaGk0ayZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/WiIuC6fAOoXD2/giphy.gif",
        "https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExZHFreWtobWUwdmx4MGlpYXZvZjVubDd4ejBuOTcweTh1d3IyaGtzeiZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/KDZdynDNJUrrp7EjTM/giphy.gif"
    ];

    let random_gif = gifs.choose(&mut rand::rng()).unwrap_or(&gifs[0]);

    CreateEmbed::default()
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
        .field("Matches Played", details.season_count.unwrap_or(0).to_string(), true)
        .field("✨ LD Monsters (RTA only)", ld_display, false)
        .field("🔥 Most Used Units Winrate", top_display, false)
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

pub async fn create_replay_image(recent_replays: Vec<Replay>, token: &str, rows: i32, cols: i32) -> Result<PathBuf> {
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

        let (wr_one, wr_two) = get_replay_estimation(
            token,
            &monster_ids_one,
            &monster_ids_two,
            battle.player_one.leader_monster_id,
            battle.player_two.leader_monster_id,
            battle.player_one.ban_monster_id,
            battle.player_two.ban_monster_id,
            if is_p1_first_pick { 1 } else { 2 },
        )
        .await
        .unwrap_or((0.0, 0.0));

        let p1_banner = create_name_banner(
            format!(
                "{} - {} - {:.0}%",
                battle.player_one.player_score,
                battle.player_one.player_name,
                wr_one * 100.0
            )
            .as_str(),
            img1.width(),
            Rgba([0, 0, 0, 0]),
        );

        let p2_banner = create_name_banner(
            format!(
                "{:.0}% - {} - {}",
                wr_two * 100.0,
                battle.player_two.player_name,
                battle.player_two.player_score
            )
            .as_str(),
            img2.width(),
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
        let y = if first_pick && i == 0 {
            ((height * 2) as f32 / 2.0 - (height as f32 / 2.0)).round() as u32
        } else if !first_pick && i == 4 {
            ((height * 2) as f32 / 2.0 - (height as f32 / 2.0)).round() as u32
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

    cache.insert(filename.to_string(), img.clone());
    Ok(img)
}

fn create_name_banner(text: &str, width: u32, color: Rgba<u8>) -> RgbaImage {
    let height = 40;
    let mut image = ImageBuffer::from_pixel(width, height, color); // Couleur de fond (semi-transparente)

    const FONT_BYTES: &[u8] = include_bytes!("NotoSansCJK-Regular.otf");
    let font = FontArc::try_from_vec(FONT_BYTES.to_vec()).expect("Police invalide");

    let scale = PxScale::from(26.0);
    let text_width = (text.len() as u32 * 8).min(width);
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

/// Appelle l'API de prédiction et retourne les estimations de winrate (one, two)
pub async fn get_replay_estimation(
    token: &str,
    player_one_units: &[u32],
    player_two_units: &[u32],
    leader_one: u32,
    leader_two: u32,
    ban_one: u32,
    ban_two: u32,
    first_pick: u8, // 1 or 2
) -> Result<(f32, f32)> {
    let url = "https://m.swranking.com/api/ai/calcGameResult";
    let client = reqwest::Client::new();

    let json_body = serde_json::json!({
        "player_one_units": player_one_units.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","),
        "player_two_units": player_two_units.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(","),
        "leader_one_unit": leader_one,
        "leader_two_unit": leader_two,
        "ban_one_unit": ban_one,
        "ban_two_unit": ban_two,
        "first_pick": first_pick,
    });

    let res = client
        .post(url)
        .json(&json_body)
        .header("Authentication", token)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(anyhow!("API returned error status: {}", res.status()));
    }

    let resp: EstimationResponse = res.json().await?;
    Ok((resp.data.one_win_rate, resp.data.two_win_rate))
}
