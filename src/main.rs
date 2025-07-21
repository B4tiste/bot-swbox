mod commands;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use poise::serenity_prelude::{ClientBuilder, GatewayIntents};
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

// Pour la map des monstres
use std::{collections::HashMap, fs};
use once_cell::sync::Lazy;
use serde::Deserialize;

// use crate::commands::duo_stats::get_duo_stats::get_duo_stats;
use crate::commands::help::help::help;
use crate::commands::mob_stats::get_mob_stats::get_mob_stats;
use crate::commands::player_names::track_player_names::track_player_names;
use crate::commands::ranks::get_ranks::get_ranks;
use crate::commands::suggestion::send_suggestion::send_suggestion;
use crate::commands::upload_json::upload_json::upload_json;
use crate::commands::player_stats::get_player_stats::get_player_stats;
use crate::commands::leaderboard::get_leaderboard::get_rta_leaderboard;
use crate::commands::rta_core::get_rta_core::get_rta_core;
use crate::commands::replays::get_replays::get_replays;
// use crate::commands::how_to_build::how_to_build::how_to_build;

lazy_static! {
    static ref LOG_CHANNEL_ID: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
    static ref GUARDIAN_EMOJI_ID: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref PUNISHER_EMOJI_ID: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref CONQUEROR_EMOJI_ID: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref MONGO_URI: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    // Variable globale pour stocker le token de l'API
    static ref API_TOKEN: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
}

pub struct Data;

/// Structure pour parser chaque entrée de monsters_elements.json
#[derive(Deserialize, Clone)]
struct MonsterEntry {
    pub com2us_id: u32,
    pub name: String,
    pub awaken_level: u8,
}

/// Wrapper si le JSON a une racine { "monsters": [...] }
#[derive(Deserialize)]
struct MonstersFile {
    pub monsters: Vec<MonsterEntry>,
}

/// Map statique: name -> com2us_id, pour awaken_level > 0
static MONSTER_MAP: Lazy<HashMap<String, u32>> = Lazy::new(|| {
    let data = fs::read_to_string("monsters_elements.json")
        .expect("Impossible de lire monsters_elements.json");
    let file: MonstersFile = serde_json::from_str(&data)
        .expect("Impossible de parser monsters_elements.json");

    let mut tmp: HashMap<String, MonsterEntry> = HashMap::new();
    for entry in file.monsters.into_iter() {
        if entry.awaken_level > 0 {
            tmp.entry(entry.name.clone())
                .and_modify(|e| {
                    if entry.awaken_level > e.awaken_level {
                        *e = entry.clone();
                    }
                })
                .or_insert(entry);
        }
    }
    tmp.into_iter()
        .map(|(name, entry)| (name, entry.com2us_id))
        .collect()
});

/// Fonction asynchrone qui se connecte au service web et retourne le token
async fn login(username: String, password: String) -> Result<String> {
    // Calculer le hash MD5 du mot de passe
    let md5_password = format!("{:x}", md5::compute(password));

    let login_url = "https://m.swranking.com/api/login";

    let client = reqwest::Client::new();

    // Construire les headers pour la requête
    let mut headers = reqwest::header::HeaderMap::new();
    // Correction ici : "*/*" au lieu de "*/"
    headers.insert("Accept", "*/*".parse()?);
    headers.insert(
        "Accept-Language",
        "fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7".parse()?,
    );
    headers.insert("Connection", "keep-alive".parse()?);
    headers.insert("Content-Type", "application/x-www-form-urlencoded".parse()?);
    headers.insert("Origin", "https://m.swranking.com".parse()?);
    headers.insert("Referer", "https://m.swranking.com/".parse()?);
    headers.insert("Sec-Fetch-Dest", "empty".parse()?);
    headers.insert("Sec-Fetch-Mode", "cors".parse()?);
    headers.insert("Sec-Fetch-Site", "same-origin".parse()?);
    headers.insert(
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36"
            .parse()?,
    );

    // Construire le corps de la requête
    let params = [("username", username), ("password", md5_password)];

    let response = client
        .post(login_url)
        .headers(headers)
        .form(&params)
        .send()
        .await?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await?;
        if json.get("enMessage").and_then(|v| v.as_str()) == Some("Success") {
            let token = json
                .get("data")
                .and_then(|data| data.get("token"))
                .and_then(|t| t.as_str())
                .ok_or_else(|| anyhow::anyhow!("Token non trouvé dans la réponse"))?;
            Ok(token.to_string())
        } else {
            Err(anyhow::anyhow!(
                "Login failed: {:?}",
                json.get("enMessage")
            ))
        }
    } else {
        let status = response.status();
        let text = response.text().await?;
        Err(anyhow::anyhow!(
            "Request failed with status code {}: {}",
            status,
            text
        ))
    }
}

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secret_store: SecretStore) -> ShuttleSerenity {
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    let guardian_emoji_id = secret_store
        .get("GUARDIAN_EMOJI_ID")
        .context("'GUARDIAN_EMOJI_ID' was not found")?;

    let punisher_emoji_id = secret_store
        .get("PUNISHER_EMOJI_ID")
        .context("'PUNISHER_EMOJI_ID' was not found")?;

    let conqueror_emoji_id = secret_store
        .get("CONQUEROR_EMOJI_ID")
        .context("'CONQUEROR_EMOJI_ID' was not found")?;

    let log_channel_id = secret_store
        .get("LOG_CHANNEL_ID")
        .context("'LOG_CHANNEL_ID' was not found")?
        .parse::<u64>()
        .context("'LOG_CHANNEL_ID' is not a valid number")?;

    let mongo_uri = secret_store
        .get("MONGO_URI")
        .context("'MONGO_URI' was not found")?;

    *GUARDIAN_EMOJI_ID.lock().unwrap() = guardian_emoji_id;
    *PUNISHER_EMOJI_ID.lock().unwrap() = punisher_emoji_id;
    *CONQUEROR_EMOJI_ID.lock().unwrap() = conqueror_emoji_id;
    *LOG_CHANNEL_ID.lock().unwrap() = log_channel_id;
    *MONGO_URI.lock().unwrap() = mongo_uri;

    // Récupérer username et password depuis secret_store
    let username = secret_store
        .get("USERNAME")
        .context("'USERNAME' was not found")?;
    let password = secret_store
        .get("PASSWORD")
        .context("'PASSWORD' was not found")?;

    // Lancer une tâche périodique pour rafraîchir le token de l'API
    tokio::spawn(async move {
        loop {
            match login(username.clone(), password.clone()).await {
                Ok(token) => {
                    *API_TOKEN.lock().unwrap() = Some(token);
                    println!("Token de l'API rafraîchi avec succès");
                }
                Err(e) => {
                    eprintln!("Erreur lors du login: {:?}", e);
                }
            }
            // Rafraîchir le token toutes les 30 minutes
            sleep(Duration::from_secs(1800)).await;
        }
    });

    // Télécharger le fichier "https://raw.githubusercontent.com/B4tiste/BP-data/refs/heads/main/data/monsters_elements.json"
    // et le stocker dans un fichier local
    let monsters_url = "https://raw.githubusercontent.com/B4tiste/BP-data/refs/heads/main/data/monsters_elements.json";
    let monsters_response = reqwest::get(monsters_url)
        .await
        .context("Failed to download monsters_elements.json")?;
    let monsters_content = monsters_response
        .text()
        .await
        .context("Failed to read monsters_elements.json content")?;
    let monsters_file_path = "monsters_elements.json";
    tokio::fs::write(monsters_file_path, &monsters_content)
        .await
        .context("Failed to write monsters_elements.json to file")?;
    println!("monsters_elements.json downloaded and saved to {}", monsters_file_path);

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                get_ranks(),
                get_mob_stats(),
                help(),
                // get_duo_stats(),
                send_suggestion(),
                track_player_names(),
                upload_json(),
                get_player_stats(),
                get_rta_leaderboard(),
                get_rta_core(),
                get_replays(),
                // how_to_build(),
            ],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data)
            })
        })
        .build();

    let client = ClientBuilder::new(discord_token, GatewayIntents::non_privileged())
        .framework(framework)
        .await
        .map_err(shuttle_runtime::CustomError::new)?;

    Ok(client.into())
}
