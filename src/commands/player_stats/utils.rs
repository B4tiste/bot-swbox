use crate::MONGO_URI;
use anyhow::{anyhow, Result};
use mongodb::{bson::doc, Client as mongoClient, Collection};
use reqwest::Client;
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
    enMessage: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchData {
    list: Vec<Player>,
}

#[derive(Debug, Deserialize)]
pub struct PlayerDetail {
    #[serde(rename = "playerName")]
    pub name: String,
    pub playerScore: Option<i32>,
    pub playerRank: Option<i32>,
    pub winRate: Option<f32>,
    pub headImg: Option<String>,
    pub playerMonsters: Option<Vec<PlayerMonster>>,
    pub monsterSimpleImgs: Option<Vec<String>>,
    pub monsterLDImgs: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct PlayerMonster {
    pub monsterId: i32,
    pub monsterImg: String,
    pub winRate: f32,
}

#[derive(Debug, Deserialize)]
struct DetailResponse {
    data: Option<PlayerDetailWrapper>,
    enMessage: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlayerDetailWrapper {
    player: PlayerDetail,
    playerMonsters: Option<Vec<PlayerMonster>>,
    monsterSimpleImgs: Option<Vec<String>>,
    monsterLDImgs: Option<Vec<String>>,
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
            "Erreur status {}: {:?}",
            status,
            resp_json.enMessage
        ));
    }

    resp_json
        .data
        .map(|d| PlayerDetail {
            name: d.player.name,
            playerScore: d.player.playerScore,
            playerRank: d.player.playerRank,
            winRate: d.player.winRate,
            headImg: d.player.headImg,
            playerMonsters: d.playerMonsters, // ✅ depuis data.*
            monsterSimpleImgs: d.monsterSimpleImgs, // ✅ depuis data.*
            monsterLDImgs: d.monsterLDImgs,   // ✅ depuis data.*
        })
        .ok_or_else(|| anyhow!("Détail du joueur introuvable"))
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
            "Erreur status {}: {:?}",
            status,
            resp_json.enMessage
        ));
    }

    Ok(resp_json.data.map(|d| d.list).unwrap_or_default())
}

/// Récupère la collection Mongo une seule fois
pub async fn get_mob_emoji_collection() -> Result<Collection<mongodb::bson::Document>> {
    let mongo_uri = {
        let uri_guard = MONGO_URI.lock().unwrap();
        uri_guard.clone()
    };

    let client = mongoClient::with_uri_str(&mongo_uri).await?;
    Ok(client
        .database("bot-swbox-db")
        .collection::<mongodb::bson::Document>("mob-emoji"))
}

/// Récupère un emoji Discord depuis MongoDB à partir du nom de fichier du monstre
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

/// Formate les informations du joueur + une sélection d'emojis (max 10)
pub async fn format_player_info_with_emojis(details: &PlayerDetail) -> String {
    // println!("Détails du joueur: {:?}", details);

    let mut emojis: Vec<String> = vec![];

    // Récupère tous les fichiers image potentiels
    let mut files = vec![];
    if let Some(ld) = &details.monsterLDImgs {
        files.extend(ld.clone());
    }
    // if let Some(simple) = &details.monsterSimpleImgs {
    //     files.extend(simple.clone());
    // }
    // if let Some(top) = &details.playerMonsters {
    //     files.extend(top.iter().map(|m| m.monsterImg.clone()));
    // }

    // Supprime les doublons
    files.sort();
    files.dedup();

    // println!("Fichiers: {:?}", files);

    // ✅ Ouvre une seule connexion MongoDB
    let collection = match get_mob_emoji_collection().await {
        Ok(c) => c,
        Err(_) => {
            return format!(
                "Nom: {}\nWinRate: {:.2}%\nScore: {}\nRank: {}\nImage: {}\nMonstres: ❌ Erreur Mongo",
                details.name,
                details.winRate.unwrap_or(0.0) * 100.0,
                details.playerScore.unwrap_or(0),
                details.playerRank.unwrap_or(0),
                details.headImg.clone().unwrap_or_else(|| "N/A".to_string()),
            );
        }
    };

    // 🔁 Utilise la même collection dans la boucle
    for file in files.iter() {
        if let Some(emoji) = get_emoji_from_filename(&collection, file).await {
            emojis.push(emoji);
        }
    }

    format!(
        "Nom: {}\nWinRate: {:.2}%\nScore: {}\nRank: {}\nImage: {}\nMonstres: {}",
        details.name,
        details.winRate.unwrap_or(0.0) * 100.0,
        details.playerScore.unwrap_or(0),
        details.playerRank.unwrap_or(0),
        details.headImg.clone().unwrap_or_else(|| "N/A".to_string()),
        if emojis.is_empty() {
            "Aucun".to_string()
        } else {
            emojis.join(" ")
        }
    )
}

pub async fn format_player_emojis_only(details: &PlayerDetail) -> Vec<String> {
    let mut emojis = vec![];

    let mut files = vec![];
    if let Some(ld) = &details.monsterLDImgs {
        files.extend(ld.clone());
    }
    // if let Some(simple) = &details.monsterSimpleImgs {
    //     files.extend(simple.clone());
    // }
    // if let Some(top) = &details.playerMonsters {
    //     files.extend(top.iter().map(|m| m.monsterImg.clone()));
    // }

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

    // Récupère la collection Mongo une seule fois
    let collection = match get_mob_emoji_collection().await {
        Ok(c) => c,
        Err(_) => return output,
    };

    if let Some(monsters) = &details.playerMonsters {
        for m in monsters {
            if let Some(emoji) = get_emoji_from_filename(&collection, &m.monsterImg).await {
                let entry = format!("{} `{:.2}%`\n", emoji, m.winRate * 100.0);
                output.push(entry);
            }
        }
    }

    output
}
