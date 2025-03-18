use crate::commands::json::process_json::process_json;
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::send_log;
use crate::Data;
use poise::{
    serenity_prelude::{Attachment, Error},
    CreateReply,
};
use reqwest;
use serde_json::Value;

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

    let (score, map_score_eff, map_score_spd, wizard_info_data) = process_json(json);

    let wizard_name = wizard_info_data
        .get("wizard_name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let wizard_id = wizard_info_data
        .get("wizard_id")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let mut message = String::new();
    message.push_str("-------------[JSON]-------------\n");
    message.push_str(&format!("Account: {} (ID: {})\n\n", wizard_name, wizard_id));
    message.push_str(&format!("Score: {}\n\n", score));

    message.push_str("Eff     100     110     120\n");

    use std::collections::HashMap;
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
            message.push_str(&row);
        }
    }
    message.push_str(&format!("{:<8}", "Total"));
    for bucket in &["100", "110", "120"] {
        let total = total_eff.get(bucket).unwrap();
        message.push_str(&format!("{:<8}", total));
    }
    message.push_str("\n--------------------------------\n\n");

    message.push_str("Spd     23      26      29      32\n");
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
            message.push_str(&row);
        }
    }
    message.push_str(&format!("{:<8}", "Total"));
    for bucket in &["23", "26", "29", "32"] {
        let total = total_spd.get(bucket).unwrap();
        message.push_str(&format!("{:<8}", total));
    }
    message.push_str("\n--------------------------------\n");

    ctx.send(CreateReply {
        content: Some(format!("```autohotkey\n{}```", message)),
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

    Ok(())
}
