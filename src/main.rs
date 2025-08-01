mod commands;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use poise::serenity_prelude::{ClientBuilder, GatewayIntents};
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

// Pour la map des monstres
use once_cell::sync::Lazy;
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
use crate::commands::register::register::register;
use crate::commands::support::support::support;

use futures::stream::StreamExt;
use mongodb::{bson::doc, Client as MongoClient};
use regex::Regex;
use reqwest::{
    header::{HeaderMap, USER_AGENT},
    Client,
};

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

pub async fn fetch_fresh_coupons() -> Result<serde_json::Value, anyhow::Error> {
    // 1. Création du client avec cookie store
    let client = Client::builder().cookie_store(true).build()?;

    // 2. Charger la page d’accueil pour avoir cookies ET le _csrf_token dans le HTML
    let home_url = "https://swq.jp/l/fr-FR/";
    let home_resp = client.get(home_url).send().await?;
    let home_html = home_resp.text().await?;

    // 3. Parser le _csrf_token (dans un input hidden)
    let re = Regex::new(r#""token"\s*:\s*"([a-zA-Z0-9_\-]+)""#).unwrap();
    let csrf_token = re
        .captures(&home_html)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| anyhow::anyhow!("_csrf_token non trouvé dans le JS FW"))?;

    // 4. Headers identiques à la vraie requête
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36".parse().unwrap());
    headers.insert("accept", "*/*".parse().unwrap());
    headers.insert(
        "accept-language",
        "fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7".parse().unwrap(),
    );
    headers.insert("priority", "u=1, i".parse().unwrap());
    headers.insert("referer", home_url.parse().unwrap());
    headers.insert(
        "sec-ch-ua",
        r#""Not)A;Brand";v="8", "Chromium";v="138", "Google Chrome";v="138""#
            .parse()
            .unwrap(),
    );
    headers.insert("sec-ch-ua-mobile", "?0".parse().unwrap());
    headers.insert("sec-ch-ua-platform", r#""Windows""#.parse().unwrap());
    headers.insert("sec-fetch-dest", "empty".parse().unwrap());
    headers.insert("sec-fetch-mode", "cors".parse().unwrap());
    headers.insert("sec-fetch-site", "same-origin".parse().unwrap());
    headers.insert("x-requested-with", "XMLHttpRequest".parse().unwrap());

    // 5. Construire l’URL des coupons avec le vrai token
    let coupons_url = format!(
        "https://swq.jp/_special/rest/Sw/Coupon?_csrf_token={}&_ctx%5Bb%5D=master&_ctx%5Bc%5D=JPY&_ctx%5Bl%5D=fr-FR&_ctx%5Bt%5D=Europe%2FBerlin%3B%2B0200&results_per_page=25",
        csrf_token
    );

    // 6. Faire la requête coupons avec headers+cookies+csrf_token
    let coupons_resp = client.get(&coupons_url).headers(headers).send().await?;

    // 7. Vérification du status
    if !coupons_resp.status().is_success() {
        let status = coupons_resp.status();
        let body = coupons_resp.text().await?;
        return Err(anyhow::anyhow!("Coupons HTTP status {}: {}", status, body));
    }
    let coupons_json = coupons_resp.json::<serde_json::Value>().await?;
    Ok(coupons_json)
}

// ----------------------------------------------------------------------

pub async fn parse_and_apply_coupons(mongo_uri: String) {
    loop {
        // 1. Télécharger la liste des coupons via la nouvelle fonction (anti-403)
        let coupons_json = match fetch_fresh_coupons().await {
            Ok(j) => j,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(3600)).await;
                continue;
            }
        };

        // 2. Récupérer la collection coupons & registered_users
        let mongo = match MongoClient::with_uri_str(&mongo_uri).await {
            Ok(c) => c,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(3600)).await;
                continue;
            }
        };
        let db = mongo.database("bot-swbox-db");
        let coupons_col = db.collection("coupons");
        let users_col = db.collection::<mongodb::bson::Document>("registered_users");

        // 3. Liste des labels de coupons verified du JSON API
        let api_verified_labels: Vec<String> = coupons_json
            .get("data")
            .and_then(|arr| arr.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|coupon| {
                let status = coupon.get("Status")?.as_str()?;
                let label = coupon.get("Label")?.as_str()?;
                if status == "verified" { Some(label.to_string()) } else { None }
            })
            .collect();

        // 4. Supprimer tous les coupons dans la base qui ne sont plus verified (ou supprimés de l’API)
        let filter = doc! {
            "Label": { "$nin": &api_verified_labels }
        };
        if let Err(e) = coupons_col.delete_many(filter).await {
            eprintln!("Erreur lors de la suppression des coupons obsolètes: {:?}", e);
        }

        // 5. Traiter/insérer tous les nouveaux coupons verified
        for coupon in coupons_json
            .get("data")
            .and_then(|arr| arr.as_array())
            .unwrap_or(&vec![])
            .iter()
        {
            let label = coupon.get("Label").and_then(|v| v.as_str()).unwrap_or("");
            let status = coupon.get("Status").and_then(|v| v.as_str()).unwrap_or("");

            let resources: Vec<String> = coupon
                .get("Resources")
                .and_then(|v| v.as_array())
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|res| {
                    let quantity = res.get("Quantity")?.as_str()?;
                    let label = res.get("Sw_Resource")?.get("Label")?.as_str()?;
                    Some(format!("{} {}", quantity, label))
                })
                .collect();

            if status != "verified" {
                continue;
            }

            // Vérifie si déjà en base (insertion unique)
            if coupons_col
                .find_one(doc! { "Label": label })
                .await
                .unwrap_or(None)
                .is_some()
            {
                continue; // déjà appliqué
            }

            // Ajoute en base pour ne plus le refaire
            if let Err(_) = coupons_col
                .insert_one(
                    doc! { "Label": label, "Status": status, "Resources": resources.clone() }
                )
                .await
            {
                continue;
            }

            // Applique à tous les users
            let mut cursor = users_col.find(doc! {}).await.unwrap();
            while let Some(user) = cursor.next().await {
                if let Ok(user_doc) = user {
                    let hive_id = user_doc.get_str("hive_id").unwrap_or("");
                    let server = user_doc.get_str("server").unwrap_or("europe");
                    let _ = user_doc.get_str("user_id").unwrap_or("");

                    // Applique à tous les users
                    let params = [
                        ("country", "FR"), // ou "EN" selon besoin
                        ("lang", "fr"),    // ou "en"
                        ("server", server),
                        ("hiveid", hive_id),
                        ("coupon", label),
                    ];

                    let client = reqwest::Client::new();
                    let mut headers = reqwest::header::HeaderMap::new();

                    headers.insert(
                        "Accept",
                        "application/json, text/javascript, */*; q=0.01"
                            .parse()
                            .unwrap(),
                    );
                    headers.insert(
                        "Accept-Language",
                        "fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7".parse().unwrap(),
                    );
                    headers.insert("Connection", "keep-alive".parse().unwrap());
                    headers.insert(
                        "Content-Type",
                        "application/x-www-form-urlencoded; charset=UTF-8"
                            .parse()
                            .unwrap(),
                    );
                    headers.insert("Origin", "https://event.withhive.com".parse().unwrap());
                    headers.insert(
                        "Referer",
                        "https://event.withhive.com/ci/smon/evt_coupon"
                            .parse()
                            .unwrap(),
                    );
                    headers.insert("Sec-Fetch-Dest", "empty".parse().unwrap());
                    headers.insert("Sec-Fetch-Mode", "cors".parse().unwrap());
                    headers.insert("Sec-Fetch-Site", "same-origin".parse().unwrap());
                    headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36".parse().unwrap());
                    headers.insert("X-Requested-With", "XMLHttpRequest".parse().unwrap());
                    headers.insert(
                        "sec-ch-ua",
                        r#""Not)A;Brand";v="8", "Chromium";v="138", "Google Chrome";v="138""#
                            .parse()
                            .unwrap(),
                    );
                    headers.insert("sec-ch-ua-mobile", "?0".parse().unwrap());
                    headers.insert("sec-ch-ua-platform", r#""Windows""#.parse().unwrap());

                    headers.insert(
                        reqwest::header::COOKIE,
                        "gdpr_section=true; _ga=GA1.1.1229236292.1730271769; _ga_FWV2C4HMXW=GS1.1.1730271768.1.1.1730271786.0.0.0; language=fr".parse().unwrap()
                    );

                    let coupon_url = "https://event.withhive.com/ci/smon/evt_coupon/useCoupon";

                    let res = client
                        .post(coupon_url)
                        .headers(headers.clone())
                        .form(&params)
                        .send()
                        .await;

                    let _ = match res {
                        Ok(resp) => {
                            if let Ok(json) = resp.json::<serde_json::Value>().await {
                                matches!(
                                    json.get("retCode"),
                                    Some(serde_json::Value::Number(n)) if n.as_i64() == Some(100)
                                )
                            } else {
                                false
                            }
                        }
                        Err(_) => false,
                    };
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(3600)).await; // 1 heure
    }
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
            Err(anyhow::anyhow!("Login failed: {:?}", json.get("enMessage")))
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

    // Lancer la tâche pour appliquer les coupons
    let mongo_uri = MONGO_URI.lock().unwrap().clone();
    tokio::spawn(async move {
        parse_and_apply_coupons(mongo_uri).await;
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
                register(),
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
