use anyhow::{Context, Result};
use mongodb::{
    bson::{doc, oid::ObjectId, Document},
    options::FindOneOptions,
    Client, Collection,
};

use crate::MONGO_URI;

pub async fn get_memberships_collection() -> Result<Collection<Document>> {
    let mongo_uri = { MONGO_URI.lock().unwrap().clone() };
    let client = Client::with_uri_str(&mongo_uri).await?;
    Ok(client
        .database("bot-swbox-db")
        .collection::<Document>("memberships"))
}

pub async fn get_users_collection() -> Result<Collection<Document>> {
    let mongo_uri = { MONGO_URI.lock().unwrap().clone() };
    let client = Client::with_uri_str(&mongo_uri).await?;
    Ok(client
        .database("bot-swbox-db")
        .collection::<Document>("users"))
}

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Find the latest unclaimed subscription-payment event for this email
pub async fn find_latest_unclaimed_membership_event(
    collection: &Collection<Document>,
    email: &str,
) -> Result<Option<Document>> {
    let filter = doc! {
        "email": email,
        "claimed": false,
        "is_subscription_payment": true,
        // "type": "Subscription" // if you store it; if not, remove this line
    };

    let opts = FindOneOptions::builder()
        .sort(doc! { "created_at": -1 })
        .build();

    let doc = collection
        .find_one(filter)
        .with_options(opts)
        .await
        .context("Failed to query memberships")?;

    Ok(doc)
}

/// Atomically claim a membership doc by _id (race-safe)
pub async fn claim_membership_by_id(
    collection: &Collection<Document>,
    id: ObjectId,
    discord_id: &str,
    now_ts: i64,
) -> Result<bool> {
    let filter = doc! { "_id": id, "claimed": false };

    let update = doc! {
        "$set": {
            "claimed": true,
            "claimed_by_discord_id": discord_id,
            "claimed_at": now_ts,
        }
    };

    let res = collection
        .update_one(filter, update)
        .await
        .context("Failed to update membership claim")?;

    Ok(res.modified_count == 1)
}
