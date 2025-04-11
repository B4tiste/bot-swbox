use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serenity::{
    CreateInteractionResponse, CreateInteractionResponseMessage,
    Error,
};

use crate::commands::mob_stats::utils::{
    build_monster_stats_embed, create_level_buttons, get_monster_stats_swrt, get_swrt_settings,
};
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::utils::{get_monster_general_info, get_monster_slug};
use crate::{
    commands::shared::logs::send_log, Data, API_TOKEN, CONQUEROR_EMOJI_ID, GUARDIAN_EMOJI_ID,
    PUNISHER_EMOJI_ID,
};

/// 📊 Displays the monster stats from SWRT
#[poise::command(slash_command)]
pub async fn get_mob_stats(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Name of the monster"] monster_name: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let input_data = format!("Monster name: {:?}", monster_name);
    let user_id = ctx.author().id;

    let token = {
        let guard = API_TOKEN.lock().unwrap();
        guard.clone().ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing API token",
            ))
        })?
    };

    let monster_slug = match get_monster_slug(monster_name.clone()).await {
        Ok(slug) => slug,
        Err(_) => {
            let msg = "Monster not found.";
            let reply = ctx.send(create_embed_error(msg)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(&ctx, input_data, false, msg).await?;
            return Ok(());
        }
    };

    let monster_info = match get_monster_general_info(monster_slug.slug.clone()).await {
        Ok(info) => info,
        Err(_) => {
            let msg = "Unable to retrieve monster info.";
            let reply = ctx.send(create_embed_error(msg)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(&ctx, input_data, false, msg).await?;
            return Ok(());
        }
    };

    let (season, version) = match get_swrt_settings(&token).await {
        Ok(data) => data,
        Err(e) => {
            let reply = ctx.send(create_embed_error(&e)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(&ctx, input_data, false, &e).await?;
            return Ok(());
        }
    };

    let mut current_level = 1;

    let stats = get_monster_stats_swrt(monster_info.id, season, &version, &token, current_level)
        .await
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    let embed = build_monster_stats_embed(&stats, season, current_level).await;

    let conqueror_id: u64 = CONQUEROR_EMOJI_ID.lock().unwrap().parse().unwrap();
    let guardian_id: u64 = GUARDIAN_EMOJI_ID.lock().unwrap().parse().unwrap();
    let punisher_id: u64 = PUNISHER_EMOJI_ID.lock().unwrap().parse().unwrap();

    let reply = ctx
        .send(CreateReply {
            embeds: vec![embed],
            components: Some(vec![create_level_buttons(
                conqueror_id,
                guardian_id,
                punisher_id,
                current_level,
            )]),
            ..Default::default()
        })
        .await?;

    let message_id = reply.message().await?.id;
    let channel_id = ctx.channel_id();

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

        // Ne fais rien si c’est le même niveau qu’actuellement
        if selected_level == current_level {
            // Répond simplement "déjà sélectionné" silencieusement ou ignore
            continue;
        }

        current_level = selected_level;

        let new_stats =
            match get_monster_stats_swrt(monster_info.id, season, &version, &token, current_level)
                .await
            {
                Ok(data) => data,
                Err(e) => {
                    interaction
                        .create_response(
                            &ctx.serenity_context,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .content(format!("❌ Error: {}", e))
                                    .ephemeral(true),
                            ),
                        )
                        .await?;
                    continue;
                }
            };

        let new_embed = build_monster_stats_embed(&new_stats, season, current_level).await;

        interaction
            .create_response(
                &ctx.serenity_context,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(new_embed)
                        .components(vec![create_level_buttons(
                            conqueror_id,
                            guardian_id,
                            punisher_id,
                            current_level,
                        )]),
                ),
            )
            .await?;
    }

    send_log(
        &ctx,
        input_data,
        true,
        format!("SWRT stats sent for {}", monster_slug.name),
    )
    .await?;

    Ok(())
}
