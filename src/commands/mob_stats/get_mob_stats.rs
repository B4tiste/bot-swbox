use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serenity::builder::EditInteractionResponse;
use serenity::{CreateInteractionResponse, CreateInteractionResponseMessage, Error};

use crate::commands::mob_stats::utils::{
    build_loading_monster_stats_embed,
    build_monster_stats_embed,
    create_level_buttons,
    format_good_teams,
    format_good_matchups,
    format_bad_matchups,
    get_monster_matchups_swrt,
    get_monster_stats_swrt,
    get_swrt_settings,
};
use crate::commands::player_stats::utils::{get_emoji_from_filename, get_mob_emoji_collection};
use crate::commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion};
use crate::commands::shared::logs::send_log;
use crate::{Data, API_TOKEN, CONQUEROR_EMOJI_ID, GUARDIAN_EMOJI_ID, PUNISHER_EMOJI_ID};

// Import de la map globale
use crate::MONSTER_MAP;

/// Autocomplete basé sur MONSTER_MAP
async fn autocomplete_monster<'a>(
    _ctx: poise::ApplicationContext<'a, Data, Error>,
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    let lower = partial.to_ascii_lowercase();
    MONSTER_MAP
        .keys()
        .filter(move |name| name.to_ascii_lowercase().contains(&lower))
        .take(10)
        .cloned()
}

/// 📂 Affiche les stats du monstre
#[poise::command(slash_command)]
pub async fn get_mob_stats(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[autocomplete = "autocomplete_monster"]
    #[description = "Name of the monster"]
    monster_name: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let input_data = format!("Monster name: {:?}", monster_name);
    let user_id = ctx.author().id;

    // 1️⃣ Récupérer l'ID depuis la map
    let com2us_id = match MONSTER_MAP.get(&monster_name) {
        Some(&id) => id as i32,
        None => {
            let msg = format!("❌ Cannot find '{}', please use the autocomplete feature for a perfect match.", monster_name);
            let reply = ctx.send(create_embed_error(&msg)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(&ctx, input_data, false, &msg).await?;
            return Ok(());
        }
    };

    let token = {
        let guard = API_TOKEN.lock().unwrap();
        guard.clone().ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing API token",
            ))
        })?
    };

    let season = match get_swrt_settings(&token).await {
        Ok(data) => data,
        Err(e) => {
            let reply = ctx.send(create_embed_error(&e)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(&ctx, input_data, false, &e).await?;
            return Ok(());
        }
    };

    let mut current_level = 1;

    let stats = get_monster_stats_swrt(com2us_id, season, &token, current_level)
        .await
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let initial_embed = build_monster_stats_embed(&stats, season, current_level)
        .await
        .field(
            "📈 Best Teammates",
            "<a:loading:1358029412716515418> Loading...",
            false,
        )
        .field(
            "📈 Best Matchups",
            "<a:loading:1358029412716515418> Loading...",
            true,
        )
        .field(
            "📉 Worst Matchups",
            "<a:loading:1358029412716515418> Loading...",
            true,
        )
        .field(
            "ℹ️ Tip",
            "Use the buttons below to view stats for different arena ranks (C1-C3, G1-G2, G3, P1-P3).",
            false,
        );

    let conqueror_id: u64 = CONQUEROR_EMOJI_ID.lock().unwrap().parse().unwrap();
    let guardian_id: u64 = GUARDIAN_EMOJI_ID.lock().unwrap().parse().unwrap();
    let punisher_id: u64 = PUNISHER_EMOJI_ID.lock().unwrap().parse().unwrap();

    let reply = ctx
        .send(CreateReply {
            embeds: vec![initial_embed],
            components: Some(vec![create_level_buttons(
                conqueror_id,
                guardian_id,
                punisher_id,
                current_level,
                true,
            )]),
            ..Default::default()
        })
        .await?;

    let message_id = reply.message().await?.id;
    let channel_id = ctx.channel_id();

    let (high_teams, high_matchups, low_matchups) =
        get_monster_matchups_swrt(com2us_id, season, current_level, &token)
            .await
            .unwrap_or((vec![], vec![], vec![]));

    let collection = get_mob_emoji_collection()
        .await
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let monster_emoji = get_emoji_from_filename(&collection, &stats.image_filename)
        .await
        .unwrap_or("❓".to_string());

    let good_teams = format_good_teams(&monster_emoji, &high_teams);
    let good_matchups = format_good_matchups(&monster_emoji, &high_matchups);
    let bad_matchups = format_bad_matchups(&monster_emoji, &low_matchups);

    let updated_embed = build_monster_stats_embed(&stats, season, current_level)
        .await
        .field("📈 Best Teammates", good_teams, false)
        .field("📈 Best Matchups", good_matchups, true)
        .field("📉 Worst Matchups", bad_matchups, true)
        .field(
            "ℹ️ Tip",
            "Use the buttons below to view stats for different RTA ranks (C1-C3, P1-P3, G1-G2, G3).",
            false,
        );

    reply
        .edit(
            poise::Context::Application(ctx),
            CreateReply {
                embeds: vec![updated_embed],
                components: Some(vec![create_level_buttons(
                    conqueror_id,
                    guardian_id,
                    punisher_id,
                    current_level,
                    false,
                )]),
                ..Default::default()
            },
        )
        .await?;

    while let Some(interaction) =
        serenity::ComponentInteractionCollector::new(&ctx.serenity_context.shard)
            .channel_id(channel_id)
            .message_id(message_id)
            .filter(move |i| i.user.id == user_id)
            .timeout(std::time::Duration::from_secs(600))
            .await
    {
        let selected_level = match interaction.data.custom_id.as_str() {
            "level_c1c3" => 0,
            "level_g1g2" => 1,
            "level_g3" => 3,
            "level_p1p3" => 4,
            _ => continue,
        };

        if selected_level == current_level {
            continue;
        }

        current_level = selected_level;

        let loading_embed = build_loading_monster_stats_embed(
            &stats.monster_name,
            &stats.image_filename,
            season,
            current_level,
        )
        .await;

        interaction
            .create_response(
                &ctx.serenity_context,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(loading_embed)
                        .components(vec![create_level_buttons(
                            conqueror_id,
                            guardian_id,
                            punisher_id,
                            current_level,
                            true,
                        )]),
                ),
            )
            .await?;

        let new_stats =
            match get_monster_stats_swrt(com2us_id, season, &token, current_level)
                .await
            {
                Ok(data) => data,
                Err(e) => {
                    interaction
                        .edit_response(
                            &ctx.serenity_context.http,
                            EditInteractionResponse::new()
                                .content(format!("❌ Error fetching data: {}", e))
                                .components(vec![])
                                .embeds(vec![]),
                        )
                        .await?;
                    continue;
                }
            };

        let (high_teams, high_matchups, low_matchups) =
            get_monster_matchups_swrt(com2us_id, season, current_level, &token)
                .await
                .unwrap_or((vec![], vec![], vec![]));

        let monster_emoji = get_emoji_from_filename(&collection, &new_stats.image_filename)
            .await
            .unwrap_or("❓".to_string());

        let good_teams = format_good_teams(&monster_emoji, &high_teams);
        let good_matchups = format_good_matchups(&monster_emoji, &high_matchups);
        let bad_matchups = format_bad_matchups(&monster_emoji, &low_matchups);

        let final_embed = build_monster_stats_embed(&new_stats, season, current_level)
            .await
            .field("📈 Dream Teams", good_teams, false)
            .field("📈 Best Matchups", good_matchups, true)
            .field("📉 Worst Matchups", bad_matchups, true)
            .field(
                "ℹ️ Tip",
                "Use the buttons below to view stats for different RTA ranks (C1-C3, P1-P3, G1-G2, G3).",
                false,
            );

        interaction
            .edit_response(
                &ctx.serenity_context.http,
                EditInteractionResponse::new()
                    .embeds(vec![final_embed])
                    .components(vec![create_level_buttons(
                        conqueror_id,
                        guardian_id,
                        punisher_id,
                        current_level,
                        false,
                    )]),
            )
            .await?;
    }

    send_log(
        &ctx,
        input_data,
        true,
        format!("SWRT stats sent for {}", monster_name),
    )
    .await?;

    Ok(())
}
