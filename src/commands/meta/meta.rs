use crate::commands::meta::utils::{
    build_tier_line, create_loading_meta_embed, create_meta_embed, create_meta_level_buttons,
};
use crate::commands::player_stats::utils::get_mob_emoji_collection;
use crate::commands::rta_core::utils::get_tierlist_data;
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::{get_server_name, send_log};
use crate::commands::shared::models::LoggerDocument;
use crate::{Data, API_TOKEN, GUARDIAN_EMOJI_ID, PUNISHER_EMOJI_ID};

use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serenity::{
    builder::EditInteractionResponse, CreateInteractionResponse, CreateInteractionResponseMessage,
    Error,
};

use std::time::Duration;

/// 📂 Displays the current meta as a TierList
///
/// Usage: `/meta`
#[poise::command(slash_command)]
pub async fn get_meta(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    // Évite le timeout de 3 s
    ctx.defer().await?;

    let user_id = ctx.author().id;

    // 🔐 Récupération du token API
    let token = {
        let guard = API_TOKEN.lock().unwrap();
        guard.clone().ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing API token",
            ))
        })?
    };

    // Niveau d'API par défaut (1 = G1-G2)
    let mut current_level = 1;

    // 😃 Récupération de la collection d'emojis
    let collection = match get_mob_emoji_collection().await {
        Ok(c) => c,
        Err(_) => {
            let err_msg =
                "Impossible de récupérer les emojis des monstres (DB error). Réessaie plus tard.";
            let reply = ctx.send(create_embed_error(err_msg)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"get_meta".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            return Ok(());
        }
    };

    // 📥 Récupération de la tierlist initiale
    let tierlist_data = match get_tierlist_data(current_level, &token).await {
        Ok(data) => data,
        Err(e) => {
            let err_msg = format!("Impossible de récupérer les données : {}", e);
            let reply = ctx.send(create_embed_error(&err_msg)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"get_meta".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            return Ok(());
        }
    };

    // 🔁 Construire les lignes d'emojis pour chaque tier
    let sss_line = build_tier_line(&tierlist_data.sss_monster, &collection).await;
    let ss_line = build_tier_line(&tierlist_data.ss_monster, &collection).await;
    let s_line = build_tier_line(&tierlist_data.s_monster, &collection).await;
    let a_line = build_tier_line(&tierlist_data.a_monster, &collection).await;
    let b_line = build_tier_line(&tierlist_data.b_monster, &collection).await;
    let date = tierlist_data.date.clone().unwrap_or_default();

    // Emojis custom pour les boutons (mêmes que /get_replays)
    let guardian_id: u64 = GUARDIAN_EMOJI_ID.lock().unwrap().parse().unwrap();
    let punisher_id: u64 = PUNISHER_EMOJI_ID.lock().unwrap().parse().unwrap();

    // 🧩 Construction de l'embed via utils
    let embed = create_meta_embed(
        tierlist_data.level.into(),
        &sss_line,
        &ss_line,
        &s_line,
        &a_line,
        &b_line,
        &date,
    );

    let reply = ctx
        .send(CreateReply {
            embeds: vec![embed],
            components: Some(vec![create_meta_level_buttons(
                guardian_id,
                punisher_id,
                current_level,
                false,
            )]),
            ..Default::default()
        })
        .await?;

    let message_id = reply.message().await?.id;
    let channel_id = ctx.channel_id();

    // Boucle de gestion des interactions avec les boutons
    while let Some(interaction) =
        serenity::ComponentInteractionCollector::new(&ctx.serenity_context.shard)
            .channel_id(channel_id)
            .message_id(message_id)
            .filter(move |i| i.user.id == user_id)
            .timeout(Duration::from_secs(600))
            .await
    {
        let selected_level = match interaction.data.custom_id.as_str() {
            "meta_level_c1p3" => 0,
            "meta_level_g1g2" => 1,
            "meta_level_g3" => 3,
            _ => continue,
        };

        // Si l'utilisateur clique sur le niveau déjà affiché, on ignore
        if selected_level == current_level {
            continue;
        }

        current_level = selected_level;

        // Afficher un embed de chargement
        let loading_embed = create_loading_meta_embed(current_level);

        interaction
            .create_response(
                &ctx.serenity_context,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(loading_embed)
                        .components(vec![create_meta_level_buttons(
                            guardian_id,
                            punisher_id,
                            current_level,
                            true, // Boutons désactivés pendant le chargement
                        )]),
                ),
            )
            .await?;

        // Récupérer les nouvelles données de tierlist
        let new_tierlist_data = match get_tierlist_data(current_level, &token).await {
            Ok(data) => data,
            Err(e) => {
                interaction
                    .edit_response(
                        &ctx.serenity_context.http,
                        EditInteractionResponse::new()
                            .content(format!("❌ Impossible de récupérer les données : {}", e))
                            .components(vec![])
                            .embeds(vec![]),
                    )
                    .await?;
                continue;
            }
        };

        // Recalcul des lignes d'emojis
        let new_sss_line = build_tier_line(&new_tierlist_data.sss_monster, &collection).await;
        let new_ss_line = build_tier_line(&new_tierlist_data.ss_monster, &collection).await;
        let new_s_line = build_tier_line(&new_tierlist_data.s_monster, &collection).await;
        let new_a_line = build_tier_line(&new_tierlist_data.a_monster, &collection).await;
        let new_b_line = build_tier_line(&new_tierlist_data.b_monster, &collection).await;
        let new_date = new_tierlist_data.date.clone().unwrap_or_default();

        let final_embed = create_meta_embed(
            new_tierlist_data.level.into(),
            &new_sss_line,
            &new_ss_line,
            &new_s_line,
            &new_a_line,
            &new_b_line,
            &new_date,
        );

        interaction
            .edit_response(
                &ctx.serenity_context.http,
                EditInteractionResponse::new()
                    .embeds(vec![final_embed])
                    .components(vec![create_meta_level_buttons(
                        guardian_id,
                        punisher_id,
                        current_level,
                        false, // Boutons réactivés
                    )]),
            )
            .await?;
    }

    // 📝 Logging
    send_log(LoggerDocument::new(
        &ctx.author().name,
        &"get_meta".to_string(),
        &get_server_name(&ctx).await?,
        true,
        chrono::Utc::now().timestamp(),
    ))
    .await?;

    Ok(())
}
