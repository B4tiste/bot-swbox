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

    // For each of the top 3 teams, build the DungeonTeamData.average_time_ms by iterating over the runs and calculating the average duration of successful runs.
    // Also, create build the DungeonTeamData.succeess_rate based on the number of successful runs over total runs.
    for team in top.iter_mut() {
        let successful_runs: Vec<&crate::commands::best_pve_teams::models::RunData> =
            team.runs.iter().filter(|run| run.success).collect();
        let average_time_ms = if !successful_runs.is_empty() {
            successful_runs
                .iter()
                .map(|run| run.duration_ms)
                .sum::<u32>()
                / successful_runs.len() as u32
        } else {
            0
        };
        let success_rate = if !team.runs.is_empty() {
            successful_runs.len() as f64 / team.runs.len() as f64 * 100.0
        } else {
            0.0
        };
        team.average_time_ms = average_time_ms;
        team.success_rate = success_rate;
        // println!(
        //     "Team: {:?}, Average Time (ms): {}, Success Rate: {:.2}%",
        //     team.members, average_time_ms, success_rate
        // );
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
