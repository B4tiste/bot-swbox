use crate::commands::ranks::utils::{get_prediction_info, get_rank_info, ENABLE_PREDICTION};
use crate::commands::shared::logs::send_log;
use crate::{
    commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion},
    Data,
};
use poise::{
    serenity_prelude::{self as serenity, Error},
    CreateReply,
};
use serenity::builder::CreateEmbedFooter;

/// 📂 Displays the current scores for ranks (C1 -> G3) with prediction inline (from swrta.top)
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
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(
                &ctx,
                "Command: /ranks".to_string(),
                false,
                error_message.to_string(),
            )
            .await?;
            return Ok(());
        }
    };

    // 2) Prediction thresholds (HTML scraping) — optional
    let prediction = if ENABLE_PREDICTION {
        get_prediction_info().await.ok()
    } else {
        None
    };

    let thumbnail = "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true";

    // ----- Build the single section (live + optional prediction inline) -----

    // Helper to turn a rank vector into grouped text, adding `(pred)` when available
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

        let groups = ["Conqueror", "Punisher", "Guardian"];
        let mut description = String::new();

        for (i, group) in groups.iter().enumerate() {
            description.push_str(&format!("{group}:\n"));
            for j in 0..3 {
                let index = i * 3 + j;
                let (rank_key, live) = &live_pairs[index];

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
    full_description.push_str(&build_grouped_description(&scores, &prediction));

    // Optional small source note if predictions were attempted
    if ENABLE_PREDICTION {
        if prediction.is_some() {
            full_description.push_str("*Prediction source: <https://swrta.top/predict>*\n");
        } else {
            full_description.push_str("_Prediction currently unavailable (failed to fetch from swrta.top)._");
        }
    }

    // Single embed; no image/attachments
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
            "Join our community on discord.gg/AfANrTVaDJ to share feedback, get support, and connect with others!",
        ));

    let reply = CreateReply {
        embeds: vec![embed],
        ..Default::default()
    };

    ctx.send(reply).await?;

    send_log(
        &ctx,
        "Command: /ranks".to_string(),
        true,
        "Embed sent".to_string(),
    )
    .await?;

    Ok(())
}
