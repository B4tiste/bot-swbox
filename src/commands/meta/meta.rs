use crate::commands::player_stats::utils::get_mob_emoji_collection;
use crate::commands::rta_core::utils::{get_tierlist_data, get_emoji_from_id};
use crate::commands::rta_core::models::MonsterStat;
use crate::commands::shared::logs::{get_server_name, send_log};
use crate::commands::shared::models::LoggerDocument;
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::{Data, API_TOKEN};

use poise::{
    serenity_prelude::{self as serenity, Error},
    CreateReply,
};
use serenity::builder::CreateEmbedFooter;

/// 📂 Displays the current meta as a TierList
///
/// Usage: `/meta`
#[poise::command(slash_command)]
pub async fn get_meta(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    // Évite le timeout de 3 s
    ctx.defer().await?;

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

    // Niveau d'API par défaut (1 = comme tu avais)
    let api_level = 1;

    // 📥 Récupération de la tierlist
    let tierlist_data = match get_tierlist_data(api_level, &token).await {
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

    // 🔁 Construire les lignes d'emojis pour chaque tier
    // (on fait comme dans get_rta_core, boucle + get_emoji_from_id)

    // Helper local pour éviter de répéter trop de code (sync, pas async)
    async fn build_tier_line(
        monsters: &[MonsterStat],
        collection: &mongodb::Collection<mongodb::bson::Document>,
    ) -> String {
        let mut line = String::new();

        for m in monsters {
            let emoji = get_emoji_from_id(collection, m.monster_id)
                .await
                .unwrap_or_default();

            if !emoji.is_empty() {
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(&emoji);
            }
        }

        if line.is_empty() {
            "–".to_string()
        } else {
            line
        }
    }

    let sss_line = build_tier_line(&tierlist_data.sss_monster, &collection).await;
    let ss_line = build_tier_line(&tierlist_data.ss_monster, &collection).await;
    // let s_line = build_tier_line(&tierlist_data.s_monster, &collection).await;
    // let a_line = build_tier_line(&tierlist_data.a_monster, &collection).await;
    // let b_line = build_tier_line(&tierlist_data.b_monster, &collection).await;
    // let c_line = build_tier_line(&tierlist_data.c_monster, &collection).await;

    let thumbnail = "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true";

    // 🧩 Construction de l'embed à la façon de /get_ranks
    let embed = serenity::CreateEmbed::default()
        .title("📊 RTA Meta Tier List")
        .color(serenity::Colour::from_rgb(0, 0, 255))
        .thumbnail(thumbnail)
        .description(format!(
            "Affichage de la méta actuelle (niveau d'API / tierlist : **{}**).",
            tierlist_data.level
        ))
        .field("SSS", sss_line, false)
        .field("SS", ss_line, false)
        // .field("S", s_line, false)
        // .field("A", a_line, false)
        // .field("B", b_line, false)
        // .field("C", c_line, false)
        .footer(CreateEmbedFooter::new(format!(
            "Commandé par {} • Données SWRT",
            ctx.author().name
        )));

    let reply = CreateReply {
        embeds: vec![embed],
        ..Default::default()
    };

    ctx.send(reply).await?;

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
