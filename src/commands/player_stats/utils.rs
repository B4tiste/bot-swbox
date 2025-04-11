use crate::MONGO_URI;
use anyhow::{anyhow, Result};
use mongodb::{bson::doc, Client, Collection};
use serde::Deserialize;
use poise::serenity_prelude as serenity;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};

use crate::commands::shared::player_alias::PLAYER_ALIAS_MAP;

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
    let name_no_ext = filename.replace(".png", "");

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
    let name_no_ext = filename.replace(".png", "");

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

/// Creates an embed from player info + emojis
pub fn create_player_embed(
    details: &crate::commands::player_stats::utils::PlayerDetail,
    ld_emojis: Vec<String>,
    top_monsters: Vec<String>,
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

    let embed = CreateEmbed::default();
    embed
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
        .description(
            "⚠️ Stats are not 100% accurate ➡️ The very last battle is not included in the elo/rank, and people around 1300 elo will have weird stats (missing games, weird winrates) ⚠️",
        )
        .field(
            "WinRate",
            format!("{:.1} %", details.win_rate.unwrap_or(0.0) * 100.0),
            true,
        )
        .field(
            "Elo",
            details.player_score.unwrap_or(0).to_string(),
            true,
        )
        .field(
            "Rank",
            details.player_rank.unwrap_or(0).to_string(),
            true,
        )
        .field(
            "Matches Played",
            details.season_count.unwrap_or(0).to_string(),
            true,
        )
        .field(
            "✨ LD Monsters (RTA only)",
            ld_display,
            false,
        )
        .field("🔥 Most Used Units Winrate", top_display, false)
        .footer(CreateEmbedFooter::new(
            "Please use /send_suggestion to report any issue.",
        ))
        .clone()
}
