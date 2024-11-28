mod commands;

use anyhow::Context as _;
use poise::serenity_prelude::{ClientBuilder, GatewayIntents};
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;
use lazy_static::lazy_static;
use std::sync::Arc;
use std::sync::Mutex;

// Personnal code add
use crate::commands::ranks::get_ranks::get_ranks;
use crate::commands::mob_stats::get_mob_stats::get_mob_stats;
use crate::commands::help::help::help;
use crate::commands::duo_stats::get_duo_stats::get_duo_stats;

use poise::serenity_prelude as serenity;
use serenity::model::id::ChannelId;

lazy_static! {
    static ref GUARDIAN_EMOJI_ID: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref PUNISHER_EMOJI_ID: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref CONQUEROR_EMOJI_ID: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
}

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secret_store: SecretStore) -> ShuttleSerenity {
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    let guardian_emoji_id = secret_store
        .get("GUARDIAN_EMOJI_ID")
        .context("'GUARDIAN_EMOJI_ID' was not found")?;

    let punisher_emoji_id = secret_store
        .get("PUNISHER_EMOJI_ID")
        .context("'PUNISHER_EMOJI_ID' was not found")?;

    let conqueror_emoji_id = secret_store
        .get("CONQUEROR_EMOJI_ID")
        .context("'CONQUEROR_EMOJI_ID' was not found")?;

    *GUARDIAN_EMOJI_ID.lock().unwrap() = guardian_emoji_id;
    *PUNISHER_EMOJI_ID.lock().unwrap() = punisher_emoji_id;
    *CONQUEROR_EMOJI_ID.lock().unwrap() = conqueror_emoji_id;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![get_ranks(), get_mob_stats(), help(), get_duo_stats()],
            pre_command: |ctx| {
                Box::pin(async move {
                    let channel_id = ChannelId::new(1311708133621633044);
                    let user_name = &ctx.author().name;
                    let command_name = &ctx.command().name;
                    let message = format!("User **{}** executed command `{}`", user_name, command_name);
                    if let Err(why) = channel_id.say(&ctx.serenity_context().http, message).await {
                        println!("Error sending message: {:?}", why);
                    }
                })
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(())
            })
        })
        .build();

    let client = ClientBuilder::new(discord_token, GatewayIntents::non_privileged())
        .framework(framework)
        .await
        .map_err(shuttle_runtime::CustomError::new)?;

    Ok(client.into())
}
