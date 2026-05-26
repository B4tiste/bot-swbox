use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serenity::builder::EditInteractionResponse;
use serenity::{CreateInteractionResponse, CreateInteractionResponseMessage, Error};

use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::get_server_name;
use crate::commands::shared::logs::send_log;
use crate::commands::shared::models::LoggerDocument;
use crate::{Data, LUCKSACK_MONSTER_MAP};

use crate::commands::how_to_build::utils::{
    build_how_to_build_embed, create_lucksack_rank_buttons, fetch_lucksack_build,
    get_latest_lucksack_season,
};

const LUCKSACK_IMG_BASE_URL: &str = "https://static.lucksack.gg/images/monsters/";

/// Autocomplete basé sur LUCKSACK_MONSTER_MAP (label)
pub async fn autocomplete_lucksack_monster<'a>(
    _ctx: poise::ApplicationContext<'a, Data, Error>,
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    let lower = partial.to_ascii_lowercase();

    let mut prefix_matches: Vec<String> = LUCKSACK_MONSTER_MAP
        .keys()
        .filter(|name| name.to_ascii_lowercase().starts_with(&lower))
        .cloned()
        .collect();

    let mut contains_matches: Vec<String> = LUCKSACK_MONSTER_MAP
        .keys()
        .filter(|name| {
            let name_l = name.to_ascii_lowercase();
            !name_l.starts_with(&lower) && name_l.contains(&lower)
        })
        .cloned()
        .collect();

    prefix_matches.sort_by_key(|s| s.len());
    contains_matches.sort_by(|a, b| {
        let la = a.to_ascii_lowercase().find(&lower).unwrap_or(usize::MAX);
        let lb = b.to_ascii_lowercase().find(&lower).unwrap_or(usize::MAX);
        la.cmp(&lb).then(a.len().cmp(&b.len()))
    });

    prefix_matches
        .into_iter()
        .chain(contains_matches.into_iter())
        .take(10)
}

/// 📂 Shows RTA runes and artifacts data for a given monster
#[poise::command(slash_command)]
pub async fn how_to_build(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[autocomplete = "autocomplete_lucksack_monster"]
    #[description = "Name of the monster"]
    monster_name: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let season = match get_latest_lucksack_season().await {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("❌ Failed to fetch current season: {}", e);
            let reply = ctx.send(create_embed_error(&msg)).await?;
            schedule_message_deletion(reply, ctx).await?;

            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"how_to_build".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;

            return Ok(());
        }
    };

    let server_name = get_server_name(&ctx).await?;

    let (monster_id, collab_id, image, collab_image) = match LUCKSACK_MONSTER_MAP.get(&monster_name)
    {
        Some((id, collab, image, collab_image)) => {
            (*id, *collab, image.clone(), collab_image.clone())
        }
        None => {
            let msg = format!(
                "❌ Cannot find '{}', please use the autocomplete feature for a perfect match.",
                monster_name
            );
            let reply = ctx.send(create_embed_error(&msg)).await?;
            schedule_message_deletion(reply, ctx).await?;

            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"how_to_build".to_string(),
                &server_name,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;

            return Ok(());
        }
    };

    // Thumbnail URL basée sur le champ `image` du monsters_catalog.json
    let mut image_url: Option<String> = Some(format!("{}{}", LUCKSACK_IMG_BASE_URL, image));

    // Lucksack ranks:
    // G3 = 16
    // G1-G3 = 102
    // P2-P3 = 103
    // P1 = 11
    let mut current_rank: i32 = 102;

    // IMPORTANT: si collab_id existe, on l'utilise en priorité
    let mut effective_monster_id: i32 = collab_id.unwrap_or(monster_id);

    // si on utilise l'id collab, on bascule aussi l'image si collab_image existe
    if collab_id.is_some() {
        if let Some(ci) = collab_image.clone() {
            image_url = Some(format!("{}{}", LUCKSACK_IMG_BASE_URL, ci));
        }
    }

    // fetch initial (collab prioritaire), fallback sur monster_id si collab échoue
    let build = match fetch_lucksack_build(effective_monster_id, season, current_rank).await {
        Ok(data) => data,
        Err(e1) => {
            // fallback seulement si on était en collab
            if collab_id.is_some() && effective_monster_id != monster_id {
                match fetch_lucksack_build(monster_id, season, current_rank).await {
                    Ok(data) => {
                        effective_monster_id = monster_id;
                        image_url = Some(format!("{}{}", LUCKSACK_IMG_BASE_URL, image));
                        data
                    }
                    Err(e2) => {
                        let msg = format!("❌ Error fetching data 1: {} (fallback: {})", e1, e2);
                        let reply = ctx.send(create_embed_error(&msg)).await?;
                        schedule_message_deletion(reply, ctx).await?;

                        send_log(LoggerDocument::new(
                            &ctx.author().name,
                            &"how_to_build".to_string(),
                            &server_name,
                            false,
                            chrono::Utc::now().timestamp(),
                        ))
                        .await?;

                        return Ok(());
                    }
                }
            } else {
                let msg = format!("❌ Error fetching data 2: {}", e1);
                let reply = ctx.send(create_embed_error(&msg)).await?;
                schedule_message_deletion(reply, ctx).await?;

                send_log(LoggerDocument::new(
                    &ctx.author().name,
                    &"how_to_build".to_string(),
                    &server_name,
                    false,
                    chrono::Utc::now().timestamp(),
                ))
                .await?;

                return Ok(());
            }
        }
    };

    let embed = build_how_to_build_embed(
        &monster_name,
        season,
        current_rank,
        &build,
        image_url.clone(),
    );

    let reply = ctx
        .send(CreateReply {
            embeds: vec![embed],
            components: Some(vec![create_lucksack_rank_buttons(current_rank, false)]),
            ..Default::default()
        })
        .await?;

    let message_id = reply.message().await?.id;
    let channel_id = ctx.channel_id();
    let user_id = ctx.author().id;

    while let Some(interaction) =
        serenity::ComponentInteractionCollector::new(&ctx.serenity_context.shard)
            .channel_id(channel_id)
            .message_id(message_id)
            .filter(move |i| i.user.id == user_id)
            .timeout(std::time::Duration::from_secs(600))
            .await
    {
        let selected_rank = match interaction.data.custom_id.as_str() {
            "rank_g3" => 16,
            "rank_g1g2g3" => 102,
            "rank_p2p3" => 103,
            "rank_p1" => 11,
            _ => continue,
        };

        if selected_rank == current_rank {
            continue;
        }
        current_rank = selected_rank;

        interaction
            .create_response(
                &ctx.serenity_context,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("🔄 Loading...")
                        .components(vec![create_lucksack_rank_buttons(current_rank, true)]),
                ),
            )
            .await?;

        let build = match fetch_lucksack_build(effective_monster_id, season, current_rank).await {
            Ok(data) => data,
            Err(e) => {
                // ✅ fallback si on était sur collab_id
                if collab_id.is_some() && effective_monster_id != monster_id {
                    match fetch_lucksack_build(monster_id, season, current_rank).await {
                        Ok(data) => {
                            effective_monster_id = monster_id;
                            image_url = Some(format!("{}{}", LUCKSACK_IMG_BASE_URL, image));
                            data
                        }
                        Err(_) => {
                            interaction
                                .edit_response(
                                    &ctx.serenity_context.http,
                                    EditInteractionResponse::new()
                                        .content(format!("❌ Error fetching data 3: {}", e))
                                        .components(vec![create_lucksack_rank_buttons(
                                            current_rank,
                                            false,
                                        )])
                                        .embeds(vec![]),
                                )
                                .await?;
                            continue;
                        }
                    }
                } else {
                    interaction
                        .edit_response(
                            &ctx.serenity_context.http,
                            EditInteractionResponse::new()
                                .content(format!("❌ Error fetching data 4: {}", e))
                                .components(vec![create_lucksack_rank_buttons(current_rank, false)])
                                .embeds(vec![]),
                        )
                        .await?;
                    continue;
                }
            }
        };

        let embed = build_how_to_build_embed(
            &monster_name,
            season,
            current_rank,
            &build,
            image_url.clone(),
        );

        interaction
            .edit_response(
                &ctx.serenity_context.http,
                EditInteractionResponse::new()
                    .content("")
                    .embeds(vec![embed])
                    .components(vec![create_lucksack_rank_buttons(current_rank, false)]),
            )
            .await?;
    }

    send_log(LoggerDocument::new(
        &ctx.author().name,
        &"how_to_build".to_string(),
        &server_name,
        true,
        chrono::Utc::now().timestamp(),
    ))
    .await?;

    Ok(())
}
