use crate::commands::best_pve_teams::models::Dungeon;
use crate::commands::best_pve_teams::utils::{
    build_monster_name_map, create_pve_teams_embed, get_dungeon_stats,
};
use crate::commands::player_stats::utils::get_mob_emoji_collection;
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::{get_server_name, send_log};
use crate::commands::shared::models::LoggerDocument;
use crate::Data;
use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serenity::Error;

/// 📂 Displays the current best PvE teams to use
///
/// Usage: `/best_pve_teams`
#[poise::command(slash_command)]
pub async fn best_pve_teams(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Select the dungeon"] dungeon: Dungeon,
) -> Result<(), Error> {
    // Évite le timeout de 3 s
    ctx.defer().await?;

    // Récupération de la collection d'emojis
    let collection = match get_mob_emoji_collection().await {
        Ok(c) => c,
        Err(_) => {
            let err_msg =
                "Impossible de récupérer les emojis des monstres (DB error). Réessaie plus tard.";
            let reply = ctx.send(create_embed_error(err_msg)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"best_pve_teams".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            return Ok(());
        }
    };

    let dungeon_name = dungeon.clone().label();
    let dungeon_slug = dungeon.clone().slug();
    let dungeon_id = dungeon.id();

    // Récupération des données du donjon sélectionné
    let dungeon_data = match get_dungeon_stats(dungeon_id).await {
        Ok(data) => data,
        Err(e) => {
            let err_msg = format!("Impossible de récupérer les données : {}", e);
            let reply = ctx.send(create_embed_error(&err_msg)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"best_pve_teams".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            return Ok(());
        }
    };

    // Trier le tableau par rank_score décroissant
    let top_n: usize = 5;
    let mut sorted_data = dungeon_data.clone();
    sorted_data.sort_by(|a, b| {
        b.rank
            .partial_cmp(&a.rank)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let len = sorted_data.len();
    let top = &mut sorted_data[..top_n.min(len)];

    for team in top.iter_mut() {
        team.average_time_ms = team.mean_time_ms.round() as u32;
        team.success_rate_pct = team.success_rate * 100.0;
    }

    let monster_name_map = build_monster_name_map();

    let embed = create_pve_teams_embed(
        dungeon_name,
        dungeon_slug,
        top,
        &collection,
        &monster_name_map,
    )
    .await;

    let reply = CreateReply {
        embeds: vec![embed],
        ..Default::default()
    };

    ctx.send(reply).await?;

    // 📝 Logging
    send_log(LoggerDocument::new(
        &ctx.author().name,
        &"best_pve_teams".to_string(),
        &get_server_name(&ctx).await?,
        true,
        chrono::Utc::now().timestamp(),
    ))
    .await?;

    Ok(())
}
