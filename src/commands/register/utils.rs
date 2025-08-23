use futures::stream::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::Client as MongoClient;
use poise::serenity_prelude::{ChannelId, Context as SerenityContext, CreateMessage};

// pub async fn fetch_fresh_coupons() -> Result<serde_json::Value, anyhow::Error> {
//     let client = Client::builder().cookie_store(true).build()?;
//     let home_url = "https://swq.jp/l/fr-FR/";
//     let home_resp = client.get(home_url).send().await?;
//     let home_html = home_resp.text().await?;
//     let re = Regex::new(r#""token"\s*:\s*"([a-zA-Z0-9_\-]+)""#).unwrap();
//     let csrf_token = re
//         .captures(&home_html)
//         .and_then(|caps| caps.get(1))
//         .map(|m| m.as_str())
//         .ok_or_else(|| anyhow::anyhow!("_csrf_token non trouvé dans le JS FW"))?;
//     let mut headers = HeaderMap::new();
//     headers.insert(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36".parse().unwrap());
//     headers.insert("accept", "*/*".parse().unwrap());
//     headers.insert(
//         "accept-language",
//         "fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7".parse().unwrap(),
//     );
//     headers.insert("priority", "u=1, i".parse().unwrap());
//     headers.insert("referer", home_url.parse().unwrap());
//     headers.insert(
//         "sec-ch-ua",
//         r#""Not)A;Brand";v="8", "Chromium";v="138", "Google Chrome";v="138""#
//             .parse()
//             .unwrap(),
//     );
//     headers.insert("sec-ch-ua-mobile", "?0".parse().unwrap());
//     headers.insert("sec-ch-ua-platform", r#""Windows""#.parse().unwrap());
//     headers.insert("sec-fetch-dest", "empty".parse().unwrap());
//     headers.insert("sec-fetch-mode", "cors".parse().unwrap());
//     headers.insert("sec-fetch-site", "same-origin".parse().unwrap());
//     headers.insert("x-requested-with", "XMLHttpRequest".parse().unwrap());

//     let coupons_url = format!(
//         "https://swq.jp/_special/rest/Sw/Coupon?_csrf_token={}&_ctx%5Bb%5D=master&_ctx%5Bc%5D=JPY&_ctx%5Bl%5D=fr-FR&_ctx%5Bt%5D=Europe%2FBerlin%3B%2B0200&results_per_page=25",
//         csrf_token
//     );

//     let coupons_resp = client.get(&coupons_url).headers(headers).send().await?;
//     if !coupons_resp.status().is_success() {
//         let status = coupons_resp.status();
//         let body = coupons_resp.text().await?;
//         return Err(anyhow::anyhow!("Coupons HTTP status {}: {}", status, body));
//     }
//     let coupons_json = coupons_resp.json::<serde_json::Value>().await?;
//     Ok(coupons_json)
// }

pub async fn fetch_fresh_coupons() -> Result<serde_json::Value, anyhow::Error> {
    use reqwest::header::{HeaderMap, HeaderValue};

    let url = "https://sw-coupons.netlify.app/.netlify/functions/get-coupons";

    let client = reqwest::Client::builder().build()?;

    let mut headers = HeaderMap::new();
    headers.insert("accept", HeaderValue::from_static("*/*"));
    headers.insert(
        "accept-language",
        HeaderValue::from_static("fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7"),
    );
    headers.insert("priority", HeaderValue::from_static("u=1, i"));
    headers.insert(
        "referer",
        HeaderValue::from_static("https://sw-coupons.netlify.app/"),
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
    headers.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
    headers.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
    headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
    headers.insert(
        "user-agent",
        HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36"),
    );

    let resp = client.get(url).headers(headers).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        eprintln!("[ERROR] Coupons request failed: {status} | Body: {body}");
        return Err(anyhow::anyhow!("Coupons HTTP status {}: {}", status, body));
    }

    let json = resp.json::<serde_json::Value>().await?;

    Ok(json)
}

pub async fn update_coupon_list(mongo_uri: &str) -> anyhow::Result<()> {

    let coupons_json = fetch_fresh_coupons().await?;
    let mongo = MongoClient::with_uri_str(mongo_uri).await?;
    let db = mongo.database("bot-swbox-db");
    let coupons_col = db.collection::<Document>("coupons");

    coupons_col.delete_many(doc! {}).await?;

    fn reward_label(typ: &str, amount: i64) -> String {
        let nice = match typ {
            "crystals" => "Crystals",
            "energy" => "Energy",
            "mystical_scroll" => "Mystical Scroll",
            "ancient_coins" => "Ancient Coins",
            "mana" => "Mana",
            "summoning_stones" => "Summoning Stones",
            "guild_points" => "Guild Points",
            "arena_wings" => "Wings",
            _ => typ,
        };
        if amount == 1 {
            format!("{amount} {nice}")
        } else {
            if nice.ends_with('s') {
                format!("{amount} {nice}")
            } else {
                format!("{amount} {nice}s")
            }
        }
    }

    let source = coupons_json["coupons"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let docs: Vec<Document> = source
        .into_iter()
        .filter(|c| c["status"].as_str() == Some("valid"))
        .map(|c| {
            let rewards: Vec<String> = c["rewards"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|r| {
                    let typ = r["type"].as_str().unwrap_or("");
                    let amount = r["amount"].as_i64().unwrap_or(0);
                    if typ.is_empty() || amount <= 0 {
                        return None;
                    }
                    Some(reward_label(typ, amount))
                })
                .collect();

            let code = c["code"].as_str().unwrap_or_default();

            doc! {
                "label": code,
                "status": "verified",
                "resources": rewards,
                "source_status": c["status"].as_str().unwrap_or(""),
                "lastUpdated": c["lastUpdated"].as_str().unwrap_or(""),
            }
        })
        .collect();

    if !docs.is_empty() {
        coupons_col.insert_many(docs).await?;
    } else {
        eprintln!("[WARN] No valid coupons to insert");
    }

    Ok(())
}

pub async fn apply_missing_coupons_to_user(mongo_uri: &str, hive_id: &str) -> anyhow::Result<()> {
    let mongo = MongoClient::with_uri_str(mongo_uri).await?;
    let db = mongo.database("bot-swbox-db");
    let users_col = db.collection::<Document>("registered_users");
    let coupons_col = db.collection::<Document>("coupons");

    // 1. Get user
    let Some(user_doc) = users_col.find_one(doc! { "hive_id": hive_id }).await? else {
        return Ok(()); // User not found
    };

    let hive_id = user_doc.get_str("hive_id")?;
    let server = user_doc.get_str("server")?;
    let mut applied: Vec<String> = user_doc
        .get_array("applied_coupons")
        .ok()
        .and_then(|arr| {
            Some(
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
            )
        })
        .unwrap_or_else(Vec::new);

    // 2. Get all coupons
    let mut all_coupons = coupons_col.find(doc! { "status": "verified" }).await?;
    let mut updated = false;
    while let Some(coupon_doc) = all_coupons.try_next().await? {
        let label = coupon_doc.get_str("label")?;
        if applied.contains(&label.to_string()) {
            continue;
        }
        // Apply coupon

        let params = [
            ("country", "FR"),
            ("lang", "fr"),
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
        headers.insert(reqwest::header::COOKIE, "gdpr_section=true; _ga=GA1.1.1229236292.1730271769; _ga_FWV2C4HMXW=GS1.1.1730271768.1.1.1730271786.0.0.0; language=fr".parse().unwrap());
        let coupon_url = "https://event.withhive.com/ci/smon/evt_coupon/useCoupon";

        let res = client
            .post(coupon_url)
            .headers(headers.clone())
            .form(&params)
            .send()
            .await;

        match res {
            Ok(resp) => {
                // Optionally, you can print the response JSON:
                let _ = resp.text().await.unwrap_or_else(|_| "N/A".to_string());
            }
            Err(e) => {
                println!(
                    "[ERROR] Echec application coupon {label} pour {hive_id}: {:?}",
                    e
                );
            }
        }

        // Simulate a random delay to avoid rate limiting between 5 and 10s
        let delay_ms = rand::random::<u64>() % 5000 + 5000;

        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;

        // Add as applied (even if failed: to avoid spam)
        applied.push(label.to_string());
        updated = true;
    }
    // 3. Update applied_coupons
    if updated {
        users_col
            .update_one(
                doc! { "hive_id": hive_id },
                doc! { "$set": { "applied_coupons": &applied } },
            )
            .await?;
    }
    Ok(())
}

pub async fn apply_coupons_to_all_users(mongo_uri: &str) -> anyhow::Result<()> {
    let mongo = MongoClient::with_uri_str(mongo_uri).await?;
    let db = mongo.database("bot-swbox-db");
    let users_col = db.collection::<Document>("registered_users");
    let mut cursor = users_col.find(doc! {}).await?;
    while let Some(user_doc) = cursor.try_next().await? {
        let hive_id = user_doc.get_str("hive_id")?;
        apply_missing_coupons_to_user(mongo_uri, hive_id).await?;
        // Simulate a random delay to avoid rate limiting beetween 5 and 10s
        let delay_ms = rand::random::<u64>() % 5000 + 5000;

        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
    }

    Ok(())
}

pub async fn notify_new_coupons(
    serenity_ctx: &SerenityContext,
    mongo_uri: &str,
) -> anyhow::Result<()> {
    // Connexion Mongo
    let mongo = MongoClient::with_uri_str(mongo_uri).await?;
    let db = mongo.database("bot-swbox-db");
    let sent_coupons_col = db.collection::<Document>("sent_coupons");
    let coupons_col = db.collection::<Document>("coupons");

    // 1. Derniers coupons envoyés (labels)
    let last_doc = sent_coupons_col
        .find_one(doc! { "_id": "sent_coupons" })
        .await?;
    let last_labels: Vec<String> = last_doc
        .and_then(|doc| doc.get_array("labels").ok().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    // 2. Coupons actuellement vérifiés
    let mut cursor = coupons_col.find(doc! { "status": "verified" }).await?;
    let mut current_labels = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        if let Some(label) = doc.get_str("label").ok() {
            current_labels.push(label.to_string());
        }
    }

    // 3. Nouveaux coupons ?
    let just_new: Vec<String> = current_labels
        .iter()
        .filter(|label| !last_labels.contains(label))
        .cloned()
        .collect();

    // 4. Envoi si nouveaux coupons
    if !just_new.is_empty() {
        let cache = &serenity_ctx.cache;
        let guild_ids = cache.guilds();
        let mut sample_channels = Vec::new();
        for guild_id in guild_ids.iter() {
            if let Some(guild) = cache.guild(guild_id) {
                for channel in guild.channels.values() {
                    if channel.name == "coupons-swbox" {
                        sample_channels.push(ChannelId::from(channel.id));
                    }
                }
            }
        }

        // if just_new.len >1 => messafe with "s" else "message with no s"
        let message = if just_new.len() > 1 {
            format!(
                "**New codes available !**\n{}\n-# Direct link to apply them : https://event.withhive.com/ci/smon/evt_coupon",
                just_new
                    .iter()
                    .map(|c| format!("- `{}` → <http://withhive.me/313/{}>", c, c))
                    .collect::<Vec<_>>()
                .join("\n")
            )
        } else {
            format!(
                "**New code available !**\n`{}` → <http://withhive.me/313/{}> \n-# Direct link to apply it : https://event.withhive.com/ci/smon/evt_coupon",
                just_new[0],
                just_new[0]
            )
        };

        for channel_id in &sample_channels {
            // Ignore errors (pas de panic si le bot n'a pas les droits dans certains serveurs)
            let _ = channel_id
                .send_message(&serenity_ctx.http, CreateMessage::new().content(&message))
                .await;
        }

        // 5. Mets à jour la liste des coupons envoyés
        sent_coupons_col
            .update_one(
                doc! { "_id": "sent_coupons" },
                doc! { "$set": { "labels": &current_labels } },
            )
            .await?;
    }

    Ok(())
}
