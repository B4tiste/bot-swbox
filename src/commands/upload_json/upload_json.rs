use std::collections::HashMap;
use crate::MONGO_URI;

use crate::commands::upload_json::process_json::process_json;
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::send_log;
use crate::Data;
use poise::serenity_prelude::CreateEmbed;
use poise::{
    serenity_prelude::{Attachment, Error},
    CreateReply,
};
use reqwest;
use serde_json::Value;
use mongodb::{bson::doc, Client, Collection};
use chrono::Utc;

/// 📂 Upload a JSON file to get an account score, and some data about rune sets eff% and rune speed
///
/// Usage: `/upload_json`
#[poise::command(slash_command)]
pub async fn upload_json(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    file: Attachment,
) -> Result<(), Error> {
    if file.url.is_empty() {
        let error_message = "No file provided. Please attach a JSON file.";
        let reply = ctx.send(create_embed_error(&error_message)).await?;
        schedule_message_deletion(reply, ctx).await?;
        send_log(
            &ctx,
            "Command: /upload_json".to_string(),
            false,
            error_message.to_string(),
        )
        .await?;
        return Ok(());
    }

    if !file.filename.to_lowercase().ends_with(".json") {
        let error_message = "The provided file is not a JSON file.";
        let reply = ctx.send(create_embed_error(&error_message)).await?;
        schedule_message_deletion(reply, ctx).await?;
        send_log(
            &ctx,
            "Command: /upload_json".to_string(),
            false,
            error_message.to_string(),
        )
        .await?;
        return Ok(());
    }

    let response = match reqwest::get(&file.url).await {
        Ok(resp) => resp,
        Err(e) => {
            let error_message = format!("Failed to download the file: {}", e);
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(
                &ctx,
                "Command: /upload_json".to_string(),
                false,
                error_message,
            )
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
            send_log(
                &ctx,
                "Command: /upload_json".to_string(),
                false,
                error_message,
            )
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
            send_log(
                &ctx,
                "Command: /upload_json".to_string(),
                false,
                error_message,
            )
            .await?;
            return Ok(());
        }
    };

    let (score_eff, score_spd, map_score_eff, map_score_spd, wizard_info_data) = process_json(json);

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

    // The json date : "2025-03-14 16:33:16" (Fuseaux horaire : Corée du Sud => UTC+9)
    // Get the day, month and year
    let date = json_date.split(' ').collect::<Vec<&str>>()[0];
    let date = date.split('-').collect::<Vec<&str>>();
    let year = date[0];
    let month = date[1];
    let day = date[2];

    let mut eff_table = String::new();
    eff_table.push_str("Eff%    100     110     120\n");

    let mut total_eff: HashMap<&str, i32> = HashMap::new();
    for bucket in &["100", "110", "120"] {
        total_eff.insert(bucket, 0);
    }

    let row_order_eff = ["Other", "Will", "Swift", "Violent", "Despair", "Intangible"];

    for key in &row_order_eff {
        if let Some(category) = map_score_eff.get(&key.to_string()) {
            let display_key = match *key {
                "Other" => "Rest",
                "Intangible" => "Intang.",
                other => other,
            };
            let mut row = format!("{:<8}", display_key);
            for &bucket in &["100", "110", "120"] {
                let count = category.get(bucket).copied().unwrap_or(0);
                row.push_str(&format!("{:<8}", count));
                *total_eff.get_mut(bucket).unwrap() += count as i32;
            }
            row.push('\n');
            eff_table.push_str(&row);
        }
    }
    eff_table.push_str(&format!("{:<8}", "Total"));
    for bucket in &["100", "110", "120"] {
        let total = total_eff.get(bucket).unwrap();
        eff_table.push_str(&format!("{:<8}", total));
    }

    let mut spd_table = String::new();
    spd_table.push_str("Spd     23      26      29      32\n");

    let mut total_spd: HashMap<&str, i32> = HashMap::new();
    for bucket in &["23", "26", "29", "32"] {
        total_spd.insert(bucket, 0);
    }
    let row_order_spd = ["Other", "Will", "Swift", "Violent", "Despair", "Intangible"];

    for key in &row_order_spd {
        if let Some(category) = map_score_spd.get(&key.to_string()) {
            let display_key = match *key {
                "Other" => "Rest",
                "Intangible" => "Intang.",
                other => other,
            };
            let mut row = format!("{:<8}", display_key);
            for &bucket in &["23", "26", "29", "32"] {
                let count = category.get(bucket).copied().unwrap_or(0);
                row.push_str(&format!("{:<8}", count));
                *total_spd.get_mut(bucket).unwrap() += count as i32;
            }
            row.push('\n');
            spd_table.push_str(&row);
        }
    }
    spd_table.push_str(&format!("{:<8}", "Total"));
    for bucket in &["23", "26", "29", "32"] {
        let total = total_spd.get(bucket).unwrap();
        spd_table.push_str(&format!("{:<8}", total));
    }

    let embed = CreateEmbed::default()
        .title("JSON Report")
        .description(format!(
            "**Account**: {} (ID: {})\n**JSON Date**: {}-{}-{}\n",
            wizard_name, wizard_id, day, month, year
        ))
        .field(
            "Amount of runes per set and efficiency",
            format!(
                "```autohotkey\n{}\n\nTotal Efficiency Score: {}\n```",
                eff_table, score_eff
            ),
            false,
        )
        .field(
            "Amount of runes per set and speed",
            format!(
                "```autohotkey\n{}\n\nTotal Speed Score: {}\n```",
                spd_table, score_spd
            ),
            false,
        )
        .field(
            "Combined Score",
            format!("Efficiency + Speed = **{}**", score_eff + score_spd),
            false,
        )
        .field(
            "User that uploaded the JSON",
            format!("<@{}>", ctx.author().id),
            false,
        )
        .color(0x00FF00);
    ctx.send(CreateReply {
        embeds: vec![embed],
        ..Default::default()
    })
    .await?;
    send_log(
        &ctx,
        "Command: /upload_json".to_string(),
        true,
        format!("JSON processed successfully"),
    )
    .await?;

    // Prepare MongoDB data
    let mongo_uri = {
        let uri_guard = MONGO_URI.lock().unwrap();
        uri_guard.clone()
    };
    println!("MongoDB URI acquired: {}", mongo_uri);

    let collection = match get_mongo_collection(&mongo_uri).await {
        Ok(collection) => {
            println!("Successfully connected to MongoDB collection.");
            collection
        },
        Err(e) => {
            let error_message = format!("Failed to get MongoDB collection: {}", e);
            println!("Error connecting to MongoDB collection: {}", e);
            ctx.send(create_embed_error(&error_message)).await.ok();
            return Err(Error::Other(Box::leak(e.to_string().into_boxed_str())));
        }
    };

    // New MongoDB operation with document existence check
    let filter = doc! { "id": wizard_id.to_string() };
    let current_date = Utc::now().format("%d-%m-%Y").to_string();
    let apparition = doc! {
        "date": current_date,
        "pseudo": wizard_name,
        "score_eff": score_eff,
        "score_spd": score_spd
    };

    println!("MongoDB filter: {:?}", filter);

    match collection.find_one(filter.clone()).await {
        Ok(Some(_)) => {
            // Document exists, update it
            let update = doc! {
                "$push": { "apparitions": apparition }
            };
            println!("Updating existing MongoDB document with update: {:?}", update);

            match collection.update_one(filter, update).await {
                Ok(result) => {
                    println!("Successfully updated MongoDB document: {:?}", result);
                },
                Err(e) => {
                    let error_message = format!("Failed to update MongoDB: {}", e);
                    println!("Error updating MongoDB document: {}", e);
                    ctx.send(create_embed_error(&error_message)).await.ok();
                    return Err(Error::Other(Box::leak(e.to_string().into_boxed_str())));
                }
            }
        },
        Ok(None) => {
            // Document does not exist, insert a new one
            let new_document = doc! {
                "id": wizard_id.to_string(),
                "apparitions": vec![apparition]
            };
            println!("Inserting new MongoDB document: {:?}", new_document);

            match collection.insert_one(new_document).await {
                Ok(result) => {
                    println!("Successfully inserted new MongoDB document: {:?}", result);
                },
                Err(e) => {
                    let error_message = format!("Failed to insert into MongoDB: {}", e);
                    println!("Error inserting MongoDB document: {}", e);
                    ctx.send(create_embed_error(&error_message)).await.ok();
                    return Err(Error::Other(Box::leak(e.to_string().into_boxed_str())));
                }
            }
        },
        Err(e) => {
            let error_message = format!("Failed to query MongoDB: {}", e);
            println!("Error querying MongoDB: {}", e);
            ctx.send(create_embed_error(&error_message)).await.ok();
            return Err(Error::Other(Box::leak(e.to_string().into_boxed_str())));
        }
    };

    Ok(())
}

async fn get_mongo_collection(mongo_uri: &str) -> Result<Collection<mongodb::bson::Document>, mongodb::error::Error> {
    let client = Client::with_uri_str(mongo_uri).await?;
    let db = client.database("bot-swbox-db");
    Ok(db.collection("upload-json"))
}
