use poise::{
    serenity_prelude::{self as serenity, CreateEmbed, Error},
    CreateReply, Modal,
};
use serenity::builder::CreateEmbedFooter;

use crate::commands::duo_stats::modal::DuoStatsInfosModal;
use crate::commands::duo_stats::utils::{create_collage_from_urls, get_monsters_duo_stats};
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::send_log;
use crate::{
    commands::shared::utils::{get_monster_general_info, get_monster_slug, get_season},
    Data,
};

/// 📂 Displays the winrate of confrontation or cooperation of two given monsters
///
/// Displays the stats of a given mob.
///
/// Usage: `/get_duo_stats`
#[poise::command(slash_command)]
pub async fn get_duo_stats(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    let modal_data: DuoStatsInfosModal = match DuoStatsInfosModal::execute(ctx).await {
        Ok(Some(data)) => data,
        Ok(None) => return Ok(()),
        Err(_) => {
            let error_message = "Error executing the modal.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            send_log(&ctx, "No data received".to_string(), false, error_message).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    if modal_data.name1.to_lowercase() == modal_data.name2.to_lowercase() {
        let error_message = "The two monsters cannot be the same.";
        let reply = ctx.send(create_embed_error(&error_message)).await?;
        send_log(
            &ctx,
            format!("Input: {:?}", modal_data),
            false,
            error_message,
        )
        .await?;
        schedule_message_deletion(reply, ctx).await?;
        return Ok(());
    }

    let mob_name_1 = modal_data.name1.as_str();
    let mob_name_2 = modal_data.name2.as_str();

    let monster_1_slug = match get_monster_slug(mob_name_1.to_string()).await {
        Ok(slug) => slug,
        Err(_) => {
            let error_message = format!("Monster 1 '**{}**' not found.", mob_name_1);
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            send_log(
                &ctx,
                format!("Input: {:?}", modal_data),
                false,
                error_message,
            )
            .await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let monster_2_slug = match get_monster_slug(mob_name_2.to_string()).await {
        Ok(slug) => slug,
        Err(_) => {
            let error_message = format!("Monster 2 '**{}**' not found.", mob_name_2);
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            send_log(
                &ctx,
                format!("Input: {:?}", modal_data),
                false,
                error_message,
            )
            .await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let monster_1_general_info = match get_monster_general_info(monster_1_slug.slug.clone()).await {
        Ok(info) => info,
        Err(_) => {
            let error_message = "Unable to retrieve general information of the monster.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            send_log(
                &ctx,
                format!("Input: {:?}", modal_data),
                false,
                error_message,
            )
            .await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let monster_2_general_info = match get_monster_general_info(monster_2_slug.slug.clone()).await {
        Ok(info) => info,
        Err(_) => {
            let error_message = "Unable to retrieve general information of the monster.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            send_log(
                &ctx,
                format!("Input: {:?}", modal_data),
                false,
                error_message,
            )
            .await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let season = match get_season(None).await {
        Ok(season) => season,
        Err(_) => {
            let error_message = "Unable to retrieve the season.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            send_log(
                &ctx,
                format!("Input: {:?}", modal_data),
                false,
                error_message,
            )
            .await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let monster_duo_stats = match get_monsters_duo_stats(
        monster_1_general_info.clone(),
        monster_2_slug.clone(),
        monster_2_general_info.clone(),
        season.clone(),
    )
    .await
    {
        Ok(info) => info,
        Err(_) => {
            let error_message = "Unable to retrieve the monsters' statistics.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            send_log(
                &ctx,
                format!("Input: {:?}", modal_data),
                false,
                error_message,
            )
            .await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let bdd_channel_id = serenity::ChannelId::new(1312040599263907840);

    // Create a collage with the images of the two monsters
    let image_urls: Vec<String> = vec![
        format!(
            "https://swarfarm.com/static/herders/images/monsters/{}",
            monster_1_general_info.image_filename
        ),
        format!(
            "https://swarfarm.com/static/herders/images/monsters/{}",
            monster_duo_stats.b_monster_image_filename
        ),
    ];
    let image_urls: Vec<&str> = image_urls.iter().map(|s| s.as_str()).collect();

    if let Err(_) = create_collage_from_urls(image_urls, "collage.png").await {
        let error_message = "Unable to create the collage.";
        let reply = ctx.send(create_embed_error(&error_message)).await?;
        schedule_message_deletion(reply, ctx).await?;
        return Ok(());
    }

    // Send the collage in the BDD channel
    let attachment = serenity::CreateAttachment::path("collage.png").await?;
    let reply_handle = bdd_channel_id
        .send_message(
            &ctx.http(),
            serenity::CreateMessage::new().add_file(attachment),
        )
        .await?;
    // Retrieve the URL of the attachment in the sent message
    let attachment_url = reply_handle.attachments[0].url.clone();

    // Calculate winrates in f32
    let with_rate_str = monster_duo_stats.win_together_rate.trim_matches('"');
    let with_winrate: f32 = with_rate_str.parse::<f32>().unwrap();

    let against_rate_str = monster_duo_stats.win_against_rate.trim_matches('"');
    let against_winrate: f32 = against_rate_str.parse::<f32>().unwrap();

    let embed = CreateEmbed::default()
        .title(format!("{} & {}", monster_1_slug.name, monster_2_slug.name))
        .thumbnail(attachment_url)
        .field(
            format!("WR {} with {}", monster_1_slug.name, monster_2_slug.name),
            format!("{}%", with_winrate),
            false,
        )
        .field(
            format!("WR {} against {}", monster_1_slug.name, monster_2_slug.name),
            format!("{}%", against_winrate),
            false,
        )
        .field(
            format!(
                "WR 🔄 {} against {}",
                monster_2_slug.name, monster_1_slug.name
            ),
            format!("{}%", 100.0 - against_winrate),
            false,
        )
        .color(serenity::Colour::from_rgb(10, 50, 128))
        .footer(CreateEmbedFooter::new(
            "Data is gathered from m.swranking.com",
        ));

    let reply = CreateReply {
        embeds: vec![embed],
        ..Default::default()
    };

    ctx.send(reply).await?;
    send_log(
        &ctx,
        format!("Input: {:?}", modal_data),
        true,
        format!("Embed sent"),
    )
    .await?;

    // Delete the collage
    let _ = std::fs::remove_file("collage.png");

    Ok(())
}
