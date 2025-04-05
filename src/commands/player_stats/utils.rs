use crate::MONGO_URI;
use anyhow::{anyhow, Result};
use mongodb::{bson::doc, Client, Collection};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Player {
    #[serde(rename = "playerName")]
    pub name: String,
    #[serde(rename = "swrtPlayerId")]
    pub swrt_player_id: i64,
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

pub async fn format_player_emojis_only(details: &PlayerDetail) -> Vec<String> {
    let mut emojis = vec![];

    let mut files = vec![];
    if let Some(ld) = &details.monster_ld_imgs {
        files.extend(ld.clone());
    }

    files.sort();
    files.dedup();

    if let Ok(collection) = get_mob_emoji_collection().await {
        for file in files {
            if let Some(emoji) = get_emoji_from_filename(&collection, &file).await {
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
        let season_count = details.season_count.unwrap_or(1) as f32;

        for (index, m) in monsters.iter().enumerate() {
            if let Some(emoji) = get_emoji_from_filename(&collection, &m.monster_img).await {
                let pick_total = m.pick_total as f32;
                let pick_rate = (pick_total / season_count) * 100.0;

                let entry = format!(
                    "{}. {} Played {}/{} ({:.2} %), WinRate : {:.2} % \n",
                    index + 1,
                    emoji,
                    m.pick_total,
                    season_count as i32,
                    pick_rate,
                    m.win_rate
                );
                output.push(entry);
            }
        }
    }

    output
}

