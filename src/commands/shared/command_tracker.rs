use anyhow::{Context, Result};
use mongodb::{
    bson::{doc, Document},
    Client, Collection,
};

use crate::MONGO_URI;

const DAILY_COMMAND_LIMIT: i64 = 3; // Adjust this limit as needed

pub async fn get_users_collection() -> Result<Collection<Document>> {
    let mongo_uri = { MONGO_URI.lock().unwrap().clone() };
    let client = Client::with_uri_str(&mongo_uri).await?;
    Ok(client
        .database("bot-swbox-db")
        .collection::<Document>("users"))
}

/// Check if user has reached daily limit and track command usage
/// Returns Ok(true) if command should proceed, Ok(false) if limit reached
pub async fn track_and_check_command_limit(
    discord_id: &str,
    command_name: &str,
) -> Result<bool> {
    let users = get_users_collection().await?;
    let now = chrono::Utc::now().timestamp();

    // Get start of today in UTC
    let today_start = chrono::Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();

    let filter = doc! { "discord_id": discord_id };

    // Find existing user
    let existing_user = users.find_one(filter.clone()).await?;

    if let Some(user_doc) = existing_user {
        // Count today's commands
        let commands = user_doc
            .get_array("commands")
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|cmd| cmd.as_document())
            .filter(|cmd| {
                cmd.get_i64("timestamp")
                    .map(|ts| ts >= today_start)
                    .unwrap_or(false)
            })
            .count() as i64;

        // Check if user has active membership
        let is_member = user_doc
            .get_str("membership_status")
            .map(|status| status == "active")
            .unwrap_or(false);

        // Members have unlimited commands
        if !is_member && commands >= DAILY_COMMAND_LIMIT {
            return Ok(false);
        }

        // Add command to tracking
        let update = doc! {
            "$push": {
                "commands": {
                    "name": command_name,
                    "timestamp": now,
                }
            },
            "$set": {
                "updated_at": now,
            }
        };

        users.update_one(filter, update).await?;
    } else {
        // Create new user with first command
        let new_user = doc! {
            "discord_id": discord_id,
            "created_at": now,
            "updated_at": now,
            "membership_status": "none",
            "commands": [{
                "name": command_name,
                "timestamp": now,
            }],
        };

        users.insert_one(new_user).await?;
    }

    Ok(true)
}

/// Get the number of commands used today and the daily limit
pub async fn get_command_usage_today(discord_id: &str) -> Result<(i64, i64, bool)> {
    let users = get_users_collection().await?;

    let today_start = chrono::Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();

    let filter = doc! { "discord_id": discord_id };
    let user_doc = users.find_one(filter).await?;

    if let Some(doc) = user_doc {
        let commands_today = doc
            .get_array("commands")
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|cmd| cmd.as_document())
            .filter(|cmd| {
                cmd.get_i64("timestamp")
                    .map(|ts| ts >= today_start)
                    .unwrap_or(false)
            })
            .count() as i64;

        let is_member = doc
            .get_str("membership_status")
            .map(|status| status == "active")
            .unwrap_or(false);

        Ok((commands_today, DAILY_COMMAND_LIMIT, is_member))
    } else {
        Ok((0, DAILY_COMMAND_LIMIT, false))
    }
}
