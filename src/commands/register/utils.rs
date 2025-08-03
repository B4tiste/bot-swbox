use futures::stream::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::Client as MongoClient;
use regex::Regex;
use reqwest::{
    header::{HeaderMap, USER_AGENT},
    Client,
};

pub async fn fetch_fresh_coupons() -> Result<serde_json::Value, anyhow::Error> {
    let client = Client::builder().cookie_store(true).build()?;
    let home_url = "https://swq.jp/l/fr-FR/";
    let home_resp = client.get(home_url).send().await?;
    let home_html = home_resp.text().await?;
    let re = Regex::new(r#""token"\s*:\s*"([a-zA-Z0-9_\-]+)""#).unwrap();
    let csrf_token = re
        .captures(&home_html)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| anyhow::anyhow!("_csrf_token non trouvé dans le JS FW"))?;
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

    let coupons_url = format!(
        "https://swq.jp/_special/rest/Sw/Coupon?_csrf_token={}&_ctx%5Bb%5D=master&_ctx%5Bc%5D=JPY&_ctx%5Bl%5D=fr-FR&_ctx%5Bt%5D=Europe%2FBerlin%3B%2B0200&results_per_page=25",
        csrf_token
    );

    let coupons_resp = client.get(&coupons_url).headers(headers).send().await?;
    if !coupons_resp.status().is_success() {
        let status = coupons_resp.status();
        let body = coupons_resp.text().await?;
        return Err(anyhow::anyhow!("Coupons HTTP status {}: {}", status, body));
    }
    let coupons_json = coupons_resp.json::<serde_json::Value>().await?;
    Ok(coupons_json)
}

pub async fn update_coupon_list(mongo_uri: &str) -> anyhow::Result<()> {
    let coupons_json = fetch_fresh_coupons().await?;
    let mongo = MongoClient::with_uri_str(mongo_uri).await?;
    let db = mongo.database("bot-swbox-db");
    let coupons_col = db.collection::<Document>("coupons");
    coupons_col.delete_many(doc! {}).await?;

    let verified_coupons: Vec<_> = coupons_json["data"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|coupon| coupon["Status"].as_str() == Some("verified"))
        .map(|coupon| {
            let resources: Vec<String> = coupon["Resources"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|res| {
                    let quantity = res["Quantity"].as_str().unwrap_or("");
                    let label = res["Sw_Resource"]["Label"].as_str().unwrap_or("");
                    Some(format!("{} {}", quantity, label))
                })
                .collect();
            doc! {
                "label": coupon["Label"].as_str().unwrap_or(""),
                "status": "verified",
                "resources": resources,
            }
        })
        .collect();
    if !verified_coupons.is_empty() {
        coupons_col.insert_many(verified_coupons).await?;
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
