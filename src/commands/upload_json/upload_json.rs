use crate::MONGO_URI;
use std::collections::HashMap;

use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::{get_server_name, send_log};
use crate::commands::shared::models::{LoggerDocument, Mode};
use crate::commands::upload_json::process_json::process_json;
use crate::Data;
use mongodb::{bson::doc, Client, Collection};
use poise::serenity_prelude::CreateEmbed;
use poise::{
    serenity_prelude::{self as serenity, Attachment, Error},
    CreateReply,
};
use reqwest;
use serde_json::Value;
use serenity::builder::CreateEmbedFooter;

/// 📂 Upload a JSON file to get an account score, and some data about rune sets eff% and rune speed
///
/// Usage: `/upload_json`
#[poise::command(slash_command)]
pub async fn upload_json(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    file: Attachment,
    #[description = "Select the mode (defaults to Classic)"] mode: Option<Mode>,
) -> Result<(), Error> {
    // Defer the response to avoid the 3 seconds timeout
    ctx.defer().await?;

    if file.url.is_empty() {
        let error_message = "No file provided. Please attach a JSON file.";
        let reply = ctx.send(create_embed_error(&error_message)).await?;
        schedule_message_deletion(reply, ctx).await?;
        send_log(LoggerDocument::new(
            &ctx.author().name,
            &"upload_json".to_string(),
            &get_server_name(&ctx).await?,
            false,
            chrono::Utc::now().timestamp(),
        ))
        .await?;
        return Ok(());
    }

    if !file.filename.to_lowercase().ends_with(".json") {
        let error_message = "The provided file is not a JSON file.";
        let reply = ctx.send(create_embed_error(&error_message)).await?;
        schedule_message_deletion(reply, ctx).await?;
        send_log(LoggerDocument::new(
            &ctx.author().name,
            &"upload_json".to_string(),
            &get_server_name(&ctx).await?,
            false,
            chrono::Utc::now().timestamp(),
        ))
        .await?;
        return Ok(());
    }

    let response = match reqwest::get(&file.url).await {
        Ok(resp) => resp,
        Err(e) => {
            let error_message = format!("Failed to download the file: {}", e);
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"upload_json".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            return Ok(());
        }
    };

    let content = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            let error_message = format!("Failed to read the file content: {}", e);
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"upload_json".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            return Ok(());
        }
    };

    let json: Value = match serde_json::from_str(&content) {
        Ok(parsed) => parsed,
        Err(e) => {
            let error_message = format!("Failed to parse JSON: {}", e);
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"upload_json".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            return Ok(());
        }
    };

    let mode = mode.unwrap_or(Mode::Classic); // valeur par défaut si None

    let mode_id = match mode {
        Mode::Classic => 0,
        Mode::NoSpeedDetail => 1,
        Mode::Anonymized => 2,
        Mode::NoSpeedDetailAndAnonymized => 3,
    };

    let (
        rta_score_eff,
        rta_score_spd,
        siege_score_eff,
        siege_score_spd,
        map_score_eff,
        map_score_spd,
        wizard_info_data,
        account_info_data,
    ) = process_json(json);

    let wizard_name = wizard_info_data
        .get("wizard_name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let wizard_id = wizard_info_data
        .get("wizard_id")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let json_date = wizard_info_data
        .get("wizard_last_login")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");

    let hive_id = account_info_data
        .get("channel_uid")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    // La date JSON : "2025-03-14 16:33:16" (Fuseaux horaire : Corée du Sud => UTC+9)
    // Extraction du jour, mois et année
    let date = json_date.split(' ').collect::<Vec<&str>>()[0];
    let date = date.split('-').collect::<Vec<&str>>();
    let year = date[0];
    let month = date[1];
    let day = date[2];

    let mut eff_table = String::new();
    eff_table.push_str("Eff%    100     110     120     130\n");

    let mut total_eff: HashMap<&str, i32> = HashMap::new();
    for bucket in &["100", "110", "120", "130"] {
        total_eff.insert(bucket, 0);
    }

    let row_order_eff = [
        "Other",
        "Will",
        "Swift",
        "Violent",
        "Despair",
        "Shield",
        "Nemesis",
        "Seal",
        "Destroy",
        "Intangible",
    ];

    for key in &row_order_eff {
        if let Some(category) = map_score_eff.get(&key.to_string()) {
            let display_key = match *key {
                "Other" => "Rest",
                "Intangible" => "Intang.",
                other => other,
            };
            let mut row = format!("{:<8}", display_key);
            for &bucket in &["100", "110", "120", "130"] {
                let count = category.get(bucket).copied().unwrap_or(0);
                row.push_str(&format!("{:<8}", count));
                *total_eff.get_mut(bucket).unwrap() += count as i32;
            }
            row.push('\n');
            eff_table.push_str(&row);
        }
    }
    eff_table.push('\n');
    eff_table.push_str(&format!("{:<8}", "Total"));
    for bucket in &["100", "110", "120", "130"] {
        let total = total_eff.get(bucket).unwrap();
        eff_table.push_str(&format!("{:<8}", total));
    }

    let mut spd_table = String::new();
    spd_table.push_str("Spd     26      30      34      36\n");

    let mut total_spd: HashMap<&str, i32> = HashMap::new();
    for bucket in &["26", "30", "34", "36"] {
        total_spd.insert(bucket, 0);
    }
    let row_order_spd = [
        "Other",
        "Will",
        "Swift",
        "Violent",
        "Despair",
        "Shield",
        "Nemesis",
        "Seal",
        "Destroy",
        "Intangible",
    ];

    for key in &row_order_spd {
        if let Some(category) = map_score_spd.get(&key.to_string()) {
            let display_key = match *key {
                "Other" => "Rest",
                "Intangible" => "Intang.",
                other => other,
            };
            let mut row = format!("{:<8}", display_key);
            for &bucket in &["26", "30", "34", "36"] {
                let count = category.get(bucket).copied().unwrap_or(0);
                row.push_str(&format!("{:<8}", count));
                *total_spd.get_mut(bucket).unwrap() += count as i32;
            }
            row.push('\n');
            spd_table.push_str(&row);
        }
    }
    spd_table.push('\n');
    spd_table.push_str(&format!("{:<8}", "Total"));
    for bucket in &["26", "30", "34", "36"] {
        let total = total_spd.get(bucket).unwrap();
        spd_table.push_str(&format!("{:<8}", total));
    }

    // Ajouter l'image du JSON
    let pp_base_url = "https://swex.oss-cn-hangzhou.aliyuncs.com/playerImage/";
    let pp_url = format!("{}{}.jpg", pp_base_url, hive_id);

    // Télécharger l'image
    let response = reqwest::get(pp_url)
        .await
        .map_err(|_| serenity::Error::Other("Failed to download pp image"))?;
    let image_bytes = response
        .bytes()
        .await
        .map_err(|_| serenity::Error::Other("Failed to read pp image bytes"))?;

    let mut embed = CreateEmbed::default();
    embed = embed
        .title("JSON Report")
        .description(format!(
            "**Account**: {} (ID: {})\n**JSON Date**: {}-{}-{}\n",
            if mode_id == 2 || mode_id == 3 {
                "HIDDEN"
            } else {
                wizard_name
            },
            if mode_id == 2 || mode_id == 3 {
                "HIDDEN".to_string()
            } else {
                wizard_id.to_string()
            },
            day,
            month,
            year
        ))
        .field(
            "Amount of runes per set and efficiency",
            format!("```autohotkey\n{}\n```", eff_table),
            false,
        )
        .field(
            "Efficiency Score",
            format!(
                "RTA: **{}** - Siege: **{}**\n",
                rta_score_eff, siege_score_eff
            ),
            false,
        )
        .field(
            "Amount of runes per set and speed",
            format!(
                "```autohotkey\n{}\n```",
                if mode_id == 1 || mode_id == 3 {
                    "HIDDEN"
                } else {
                    &spd_table
                }
            ),
            false,
        )
        .field(
            "Speed Score",
            format!(
                "RTA: **{}** - Siege: **{}**\n",
                rta_score_spd, siege_score_spd
            ),
            false,
        )
        .field(
            "User that uploaded the JSON",
            format!("<@{}>", ctx.author().id),
            false,
        )
        .field(
            "Leaderboard",
            "You can check the leaderboard [HERE](https://leaderboard-bot-swbox.netlify.app/)",
            false,
        )
        .color(0x00FF00)
        .footer(CreateEmbedFooter::new(
            "Join our community on discord.gg/AfANrTVaDJ to share feedback, get support, and connect with others!",
        ));

    // Ajouter le thumbnail uniquement si ce n'est pas anonymisé
    if mode_id != 2 && mode_id != 3 {
        embed = embed.thumbnail("attachment://pp.jpg");
    }

    let reply = if mode_id == 2 || mode_id == 3 {
        CreateReply {
            embeds: vec![embed],
            ..Default::default()
        }
    } else {
        let attachements = serenity::CreateAttachment::bytes(image_bytes.to_vec(), "pp.jpg");

        CreateReply {
            embeds: vec![embed],
            attachments: vec![attachements],
            ..Default::default()
        }
    };

    ctx.send(reply).await?;

    send_log(LoggerDocument::new(
        &ctx.author().name,
        &"upload_json".to_string(),
        &get_server_name(&ctx).await?,
        true,
        chrono::Utc::now().timestamp(),
    ))
    .await?;

    // Préparation des données pour MongoDB
    let mongo_uri = {
        let uri_guard = MONGO_URI.lock().unwrap();
        uri_guard.clone()
    };

    let collection = match get_mongo_collection(&mongo_uri).await {
        Ok(collection) => collection,
        Err(e) => {
            let error_message = format!("Failed to get MongoDB collection: {}", e);
            ctx.send(create_embed_error(&error_message)).await.ok();
            return Err(Error::Other(Box::leak(e.to_string().into_boxed_str())));
        }
    };

    // Utiliser la date JSON au lieu de la date courante (DD-MM-YYYY)
    let apparition = doc! {
        "date": format!("{}-{}-{}", day, month, year),
        "pseudo": wizard_name,
        "rta_eff": rta_score_eff,
        "siege_eff": siege_score_eff,
        "rta_spd": rta_score_spd,
        "siege_spd": siege_score_spd,
        "anonyme": if mode_id == 2 || mode_id == 3 { 1 } else { 0 }
    };

    let filter = doc! { "id": wizard_id.to_string() };

    match collection.find_one(filter.clone()).await {
        Ok(Some(existing_doc)) => {
            // Vérifier si la date existe déjà dans le tableau "apparitions"
            if let Some(apparitions) = existing_doc.get_array("apparitions").ok() {
                let date_exists = apparitions.iter().any(|entry| {
                    if let Some(doc) = entry.as_document() {
                        if let Ok(date) = doc.get_str("date") {
                            return date == format!("{}-{}-{}", day, month, year);
                        }
                    }
                    false
                });

                if date_exists {
                    // La date existe déjà, pas besoin de mettre à jour
                    return Ok(());
                }
            }

            // La date n'existe pas, on met à jour le document
            let update = doc! {
                "$push": { "apparitions": apparition }
            };

            match collection.update_one(filter, update).await {
                Ok(_result) => {}
                Err(e) => {
                    let error_message = format!("Failed to update MongoDB: {}", e);
                    ctx.send(create_embed_error(&error_message)).await.ok();
                    return Err(Error::Other(Box::leak(e.to_string().into_boxed_str())));
                }
            }
        }
        Ok(None) => {
            // Le document n'existe pas, on insère un nouveau document
            let new_document = doc! {
                "id": wizard_id.to_string(),
                "apparitions": vec![apparition]
            };

            match collection.insert_one(new_document).await {
                Ok(_result) => {}
                Err(e) => {
                    let error_message = format!("Failed to insert into MongoDB: {}", e);
                    ctx.send(create_embed_error(&error_message)).await.ok();
                    return Err(Error::Other(Box::leak(e.to_string().into_boxed_str())));
                }
            }
        }
        Err(e) => {
            let error_message = format!("Failed to query MongoDB: {}", e);
            ctx.send(create_embed_error(&error_message)).await.ok();
            return Err(Error::Other(Box::leak(e.to_string().into_boxed_str())));
        }
    };

    Ok(())
}

async fn get_mongo_collection(
    mongo_uri: &str,
) -> Result<Collection<mongodb::bson::Document>, mongodb::error::Error> {
    let client = Client::with_uri_str(mongo_uri).await?;

    let db = client.database("bot-swbox-db");
    Ok(db.collection("upload-json"))
}
