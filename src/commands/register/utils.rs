use futures::stream::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::Client as MongoClient;

pub async fn update_coupon_list(mongo_uri: &str) -> anyhow::Result<()> {
    let coupons_json = crate::fetch_fresh_coupons().await?;
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
        let _ = client
            .post(coupon_url)
            .headers(headers.clone())
            .form(&params)
            .send()
            .await;
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
    }
    Ok(())
}
