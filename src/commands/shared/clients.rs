use anyhow::{anyhow, Context, Result};
use mongodb::Client as MongoClient;
use once_cell::sync::{Lazy, OnceCell};
use reqwest::Client as HttpClient;

static HTTP_CLIENT: Lazy<HttpClient> = Lazy::new(HttpClient::new);
static MONGO_CLIENT: OnceCell<MongoClient> = OnceCell::new();

pub fn http_client() -> &'static HttpClient {
    &HTTP_CLIENT
}

pub async fn init_mongo_client(mongo_uri: &str) -> Result<()> {
    let client = MongoClient::with_uri_str(mongo_uri)
        .await
        .context("Failed to initialize MongoDB client")?;

    MONGO_CLIENT
        .set(client)
        .map_err(|_| anyhow!("MongoDB client already initialized"))
}

pub fn mongo_client() -> Result<&'static MongoClient> {
    MONGO_CLIENT
        .get()
        .ok_or_else(|| anyhow!("MongoDB client is not initialized"))
}
