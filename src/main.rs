mod commands;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use poise::serenity_prelude::{ClientBuilder, Context as SerenityContext, GatewayIntents};
use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, CONNECTION, CONTENT_TYPE, ORIGIN, REFERER,
    USER_AGENT,
};
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

// Pour la map des monstres
use once_cell::sync::{Lazy, OnceCell};
use serde::Deserialize;
use std::{collections::HashMap, fs};

// use crate::commands::duo_stats::get_duo_stats::get_duo_stats;
use crate::commands::help::help::help;
use crate::commands::leaderboard::get_leaderboard::get_rta_leaderboard;
use crate::commands::mob_stats::get_mob_stats::get_mob_stats;
use crate::commands::player_names::track_player_names::track_player_names;
use crate::commands::player_stats::get_player_stats::get_player_stats;
use crate::commands::ranks::get_ranks::get_ranks;
use crate::commands::replays::get_replays::get_replays;
use crate::commands::rta_core::get_rta_core::get_rta_core;
use crate::commands::suggestion::send_suggestion::send_suggestion;
use crate::commands::upload_json::upload_json::upload_json;
// use crate::commands::how_to_build::how_to_build::how_to_build;
// use crate::commands::register::register::register;
use crate::commands::register::utils::{
    apply_coupons_to_all_users, notify_new_coupons, update_coupon_list,
};
use crate::commands::support::support::support;

lazy_static! {
    static ref LOG_CHANNEL_ID: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
    static ref GUARDIAN_EMOJI_ID: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref PUNISHER_EMOJI_ID: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref CONQUEROR_EMOJI_ID: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref MONGO_URI: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    // Variable globale pour stocker le token de l'API
    static ref API_TOKEN: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
}

// Contexte pour Serenity
static SERENITY_CTX: OnceCell<SerenityContext> = OnceCell::new();

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
    let file: MonstersFile =
        serde_json::from_str(&data).expect("Impossible de parser monsters_elements.json");

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
    // 1) Hash MD5 du mot de passe (comme ta version)
    let md5_password = format!("{:x}", md5::compute(password));

    // 2) Client avec cookie store pour gérer JSESSIONID
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .context("Failed to build reqwest client with cookie store")?;

    // 3) Pré-vol: GET la page pour récupérer JSESSIONID
    //    (le cookie est stocké automatiquement dans le cookie store)
    client
        .get("https://m.swranking.com/")
        .header(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36"))
        .send()
        .await
        .context("Preflight GET failed")?
        .error_for_status()
        .context("Preflight GET returned non-success")?;

    // 4) Headers identiques/suffisants pour mimer la requête curl
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
    headers.insert(
        ACCEPT_LANGUAGE,
        HeaderValue::from_static("fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7"),
    );
    headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    headers.insert(ORIGIN, HeaderValue::from_static("https://m.swranking.com"));
    headers.insert(
        REFERER,
        HeaderValue::from_static("https://m.swranking.com/"),
    );
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36"));

    // Headers "client-hint" + Authentication:null que le serveur semble attendre
    headers.insert(
        "Authentication",
        HeaderValue::from_static("null"), // oui, littéral "null"
    );
    headers.insert(
        "sec-ch-ua",
        HeaderValue::from_static(
            r#""Not;A=Brand";v="99", "Google Chrome";v="139", "Chromium";v="139""#,
        ),
    );
    headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
    headers.insert(
        "sec-ch-ua-platform",
        HeaderValue::from_static(r#""Windows""#),
    );
    headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));
    headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
    headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));

    // 5) Corps de la requête (form-urlencoded)
    let params = [("username", username), ("password", md5_password)];

    // 6) POST /api/login — les cookies (dont JSESSIONID) seront renvoyés automatiquement
    let resp = client
        .post("https://m.swranking.com/api/login")
        .headers(headers)
        .form(&params)
        .send()
        .await
        .context("POST /api/login failed")?
        .error_for_status()
        .context("POST /api/login returned non-success")?;

    // 7) Parse JSON + extraction du token (comme ta version)
    let json: serde_json::Value = resp.json().await.context("Invalid JSON body")?;
    if json.get("enMessage").and_then(|v| v.as_str()) == Some("Success") {
        let token = json
            .get("data")
            .and_then(|d| d.get("token"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow::anyhow!("Token non trouvé dans la réponse"))?;
        Ok(token.to_owned())
    } else {
        Err(anyhow::anyhow!("Login failed: {:?}", json.get("enMessage")))
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

    // Lancer une tâche périodique pour rafraîchir le token de l'API avec retry (max 5)
    tokio::spawn(async move {
        loop {
            let mut retry_count = 0;
            loop {
                match login(username.clone(), password.clone()).await {
                    Ok(token) => {
                        *API_TOKEN.lock().unwrap() = Some(token);
                        println!("Token de l'API rafraîchi avec succès");
                        break;
                    }
                    Err(e) => {
                        retry_count += 1;
                        eprintln!("Erreur lors du login (tentative {}/5): {:?}", retry_count, e);
                        if retry_count >= 5 {
                            eprintln!("Échec du login après 5 tentatives, attente avant nouvel essai...");
                            break;
                        }
                        sleep(Duration::from_secs(5)).await;
                    }
                }
            }
            // Rafraîchir le token toutes les heures
            sleep(Duration::from_secs(3600)).await;
        }
    });

    // Lancer une tâche périodique pour mettre à jour la liste des coupons et les appliquer aux utilisateurs
    let mongo_uri = MONGO_URI.lock().unwrap().clone();
    tokio::spawn(async move {
        // Wait for the serenity context to be set
        while SERENITY_CTX.get().is_none() {
            sleep(Duration::from_secs(1)).await;
        }
        loop {
            // Mettre à jour la liste des coupons
            if let Err(e) = update_coupon_list(&mongo_uri).await {
                eprintln!("Failed to update coupons: {e:?}");
            }

            // Notifier les nouveaux coupons
            if let Some(ctx) = SERENITY_CTX.get() {
                if let Err(e) = notify_new_coupons(ctx, &mongo_uri).await {
                    eprintln!("Failed to notify new coupons: {e:?}");
                }
            } else {
                eprintln!("Serenity context not ready, skip notify_new_coupons");
            }

            // Appliquer les coupons à tous les utilisateurs
            if let Err(e) = apply_coupons_to_all_users(&mongo_uri).await {
                eprintln!("Failed to apply coupons: {e:?}");
            }
            sleep(Duration::from_secs(1800)).await; // Toutes les 30 minutes
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
    println!(
        "monsters_elements.json downloaded and saved to {}",
        monsters_file_path
    );

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
                support(),
                // register(),
            ],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            let _ = SERENITY_CTX.set(ctx.clone());
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
