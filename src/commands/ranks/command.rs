use crate::commands::ranks::utils::{get_prediction_info, get_rank_info, ENABLE_PREDICTION};
use crate::commands::shared::logs::{get_server_name, send_log};
use crate::commands::shared::models::LoggerDocument;
use crate::{
    commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion},
    Data,
};
use chrono::TimeZone;
use poise::{
    serenity_prelude::{self as serenity, Error},
    CreateReply,
};
use serenity::builder::CreateEmbedFooter;

/// 📂 Displays the current scores for ranks (P2 -> G3) with prediction inline (from swrta.top)
///
/// Usage: `/get_ranks`
#[poise::command(slash_command)]
pub async fn get_ranks(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    // Defer the response to avoid the 3 seconds timeout
    ctx.defer().await?;

    // 1) Live/current thresholds (API)
    let scores = match get_rank_info().await {
        Ok(scores) => scores,
        Err(_) => {
            let error_message = "Unable to retrieve ELO information.";
            let reply = ctx.send(create_embed_error(error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(LoggerDocument::new(
                &ctx.author().name,
                "get_ranks",
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            return Ok(());
        }
    };

    // 2) Prediction thresholds (HTML scraping) - optional
    let prediction = if ENABLE_PREDICTION {
        get_prediction_info().await.ok()
    } else {
        None
    };

    let thumbnail = "https://raw.githubusercontent.com/B4tiste/landing-page-bot/refs/heads/main/src/assets/images/old_bot_logo.gif";

    // Helper: build grouped description for P2,P3 and G1,G2,G3 (with optional prediction)
    fn build_grouped_description(
        live_pairs: &[(String, i32)],
        pred_pairs: &Option<Vec<(String, i32)>>,
    ) -> String {
        use std::collections::HashMap;

        // Map prediction by the exact same emote string key
        let pred_map: HashMap<&str, i32> = pred_pairs
            .as_ref()
            .map(|v| v.iter().map(|(k, v)| (k.as_str(), *v)).collect())
            .unwrap_or_default();

        // live_pairs order (from utils): P2, P3, G1, G2, G3
        let groups: [(&str, std::ops::Range<usize>); 2] = [
            ("Punisher", 0..2), // P2,P3
            ("Guardian", 2..5), // G1,G2,G3
        ];

        let mut description = String::new();

        for (group, range) in groups {
            description.push_str(&format!("{group} :\n"));
            for idx in range {
                let (rank_key, live) = &live_pairs[idx];
                if let Some(pred) = pred_map.get(rank_key.as_str()) {
                    description.push_str(&format!("{rank_key} : {live} (→ **{pred}**)\n"));
                } else {
                    description.push_str(&format!("{rank_key} : **{live}**\n"));
                }
            }
            description.push('\n');
        }

        description
    }

    // Single section (Current + prediction inline)
    let mut full_description = String::new();
    if ENABLE_PREDICTION {
        full_description.push_str("Format → [ELO : live threshold (→ predicted cutoff)]\n\n");
    } else {
        full_description.push_str("Format → [ELO : live threshold]\n\n");
    }

    // Exact season end date/time provided by maintainer: 26/09/2026 07:00:00 (UTC)
    let date = chrono::Utc
        .with_ymd_and_hms(2026, 9, 26, 7, 0, 0)
        .single()
        .expect("Invalid hardcoded season end date");

    // Calculate the remaining time
    let now = chrono::Utc::now();
    let remaining = date.signed_duration_since(now);

    // Format as "X days and YY hours, MM minutes"
    let total_seconds = remaining.num_seconds().max(0);
    let months = total_seconds / 2592000;
    let days = (total_seconds % 2592000) / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;

    let formatted_time = if months > 0 {
        format!("{}mo {}d", months, days)
    } else if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    };

    full_description.push_str(&format!(
        "Season end date : <t:{}:F>\nRemaining time : `{}`\n\n",
        date.timestamp(),
        formatted_time
    ));

    full_description.push_str(&build_grouped_description(&scores, &prediction));

    full_description.push_str("Check out `/services` or [MyShop](https://discord.gg/myshop) if you need help reaching your desired rank.\n");

    // Optional small source note if predictions were attempted
    if ENABLE_PREDICTION {
        if prediction.is_some() {
            full_description.push_str("\n*Prediction source: <https://swrta.top/predict>*\n");
        } else {
            full_description
                .push_str("\n_Prediction currently unavailable (failed to fetch from swrta.top)._");
        }
    }

    full_description.push_str("\n⚠️ SWbox is not responsible for any data inaccuracy ⚠️");

    let embed = serenity::CreateEmbed::default()
        .title(if ENABLE_PREDICTION {
            "Rank thresholds (Live + Prediction)"
        } else {
            "Rank thresholds (Live)"
        })
        .color(serenity::Colour::from_rgb(0, 0, 255))
        .thumbnail(thumbnail)
        .description(full_description)
        .footer(CreateEmbedFooter::new(
            "Data is gathered from m.swranking.com",
        ));

    let reply = CreateReply {
        embeds: vec![embed],
        ..Default::default()
    };

    ctx.send(reply).await?;

    send_log(LoggerDocument::new(
        &ctx.author().name,
        "get_ranks",
        &get_server_name(&ctx).await?,
        true,
        chrono::Utc::now().timestamp(),
    ))
    .await?;

    Ok(())
}
