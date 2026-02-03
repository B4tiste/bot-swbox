use anyhow::Result;
use mongodb::{
    bson::{doc, Document},
    Client, Collection,
};

use crate::MONGO_URI;

pub async fn get_user_links_collection() -> Result<Collection<Document>> {
    let mongo_uri = {
        let uri_guard = MONGO_URI.lock().unwrap();
        uri_guard.clone()
    };

    let client = Client::with_uri_str(&mongo_uri).await?;
    Ok(client
        .database("bot-swbox-db")
        .collection::<Document>("user-links"))
}

pub async fn upsert_user_link(
    discord_user_id: u64,
    swrt_player_id: i64,
    player_name: &str,
    player_server: i32,
    player_country: &str,
    updated_at: i64,
) -> Result<()> {
    let col = get_user_links_collection().await?;

    let filter = doc! { "discord_user_id": discord_user_id as i64 };

    let update_doc = doc! {
        "discord_user_id": discord_user_id as i64,
        "swrt_player_id": swrt_player_id,
        "player_name": player_name,
        "player_server": player_server,
        "player_country": player_country,
        "updated_at": updated_at,
    };

    match col.find_one(filter.clone()).await? {
        Some(_) => {
            let update = doc! { "$set": update_doc };
            col.update_one(filter, update).await?;
        }
        None => {
            col.insert_one(update_doc).await?;
        }
    }

    Ok(())
}

pub async fn get_user_link(discord_user_id: u64) -> Result<Option<Document>> {
    let col = get_user_links_collection().await?;
    let doc_opt = col
        .find_one(doc! { "discord_user_id": discord_user_id as i64 })
        .await?;
    Ok(doc_opt)
}

pub async fn delete_user_link(discord_user_id: u64) -> Result<u64> {
    let col = get_user_links_collection().await?;
    let res = col
        .delete_one(doc! { "discord_user_id": discord_user_id as i64 })
        .await?;
    Ok(res.deleted_count)
}
