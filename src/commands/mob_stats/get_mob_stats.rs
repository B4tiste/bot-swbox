use poise::{
    serenity_prelude::{self as serenity, CreateEmbed, Error},
    CreateReply, Modal,
};

use crate::commands::mob_stats::modal::MobStatsInfosModal;
use crate::commands::mob_stats::utils::get_monster_rta_info;
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::utils::{get_monster_general_info, get_monster_slug, get_season};
use crate::{commands::shared::logs::send_log, Data, GUARDIAN_EMOJI_ID};

/// 📂 Displays the stats of the given monster. (Option: Season)
///
/// Usage: `/get_mob_stats`
#[poise::command(slash_command)]
pub async fn get_mob_stats(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    let modal_result = MobStatsInfosModal::execute(ctx).await;

    let (input_data, _input_status) = match &modal_result {
        Ok(Some(data)) => (format!("{:?}", data), true),
        Ok(None) => ("No input provided".to_string(), false),
        Err(_) => ("Error executing modal".to_string(), false),
    };

    let modal_data = match modal_result {
        Ok(Some(data)) => data,
        Ok(None) => {
            send_log(&ctx, input_data, false, "Modal canceled").await?;
            return Ok(());
        }
        Err(_) => {
            let error_message = "Error executing modal.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(&ctx, input_data, false, "Error in modal").await?;
            return Ok(());
        }
    };

    let monster_slug = match get_monster_slug(modal_data.name.clone()).await {
        Ok(slug) => slug,
        Err(_) => {
            let error_message = "Monster not found.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(&ctx, input_data, false, "Monster not found").await?;
            return Ok(());
        }
    };

    let monster_general_info = match get_monster_general_info(monster_slug.slug.clone()).await {
        Ok(info) => info,
        Err(_) => {
            let error_message = "Unable to retrieve general monster information.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(&ctx, input_data, false, "General information not found").await?;
            return Ok(());
        }
    };

    let season = match get_season(modal_data.season).await {
        Ok(season) => season,
        Err(_) => {
            let error_message = "Unable to retrieve season.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(&ctx, input_data, false, "Season not found").await?;
            return Ok(());
        }
    };

    let monster_rta_info_no_g3 =
        match get_monster_rta_info(monster_general_info.id.to_string(), season, false).await {
            Ok(info) => info,
            Err(_) => {
                let error_message = "Unable to retrieve RTA information (No G3).";
                let reply = ctx.send(create_embed_error(&error_message)).await?;
                schedule_message_deletion(reply, ctx).await?;
                send_log(&ctx, input_data, false, "RTA stats (No G3) not found").await?;
                return Ok(());
            }
        };

    let monster_rta_info_g3 =
        match get_monster_rta_info(monster_general_info.id.to_string(), season, true).await {
            Ok(info) => info,
            Err(_) => {
                let error_message = "Unable to retrieve RTA information (G3).";
                let reply = ctx.send(create_embed_error(&error_message)).await?;
                schedule_message_deletion(reply, ctx).await?;
                send_log(&ctx, input_data, false, "RTA stats (G3) not found").await?;
                return Ok(());
            }
        };

    let thumbnail = format!(
        "https://swarfarm.com/static/herders/images/monsters/{}",
        monster_general_info.image_filename
    );
    let guardian_emote_str = format!("<:guardian:{}>", GUARDIAN_EMOJI_ID.lock().unwrap());

    let embed = CreateEmbed::default()
        .title(format!(
            "Monster stats {} - Season {}",
            monster_slug.name, season
        ))
        .color(serenity::Colour::from_rgb(255, 0, 255))
        .thumbnail(thumbnail)
        .field("**Stats (All ranks):**", "", false)
        .field(
            "Play rate",
            format!(
                "{:.2}% ({})",
                monster_rta_info_no_g3.play_rate, monster_rta_info_no_g3.played
            ),
            true,
        )
        .field(
            "Win rate",
            format!(
                "{:.2}% ({})",
                monster_rta_info_no_g3.win_rate, monster_rta_info_no_g3.winner
            ),
            true,
        )
        .field(
            "Ban rate",
            format!(
                "{:.2}% ({})",
                monster_rta_info_no_g3.ban_rate, monster_rta_info_no_g3.banned
            ),
            true,
        )
        .field(
            "Lead rate",
            format!(
                "{:.2}% ({})",
                monster_rta_info_no_g3.lead_rate, monster_rta_info_no_g3.leader
            ),
            true,
        )
        .field("", "", false)
        .field(
            format!("**Stats {}**", guardian_emote_str.repeat(3)),
            "",
            false,
        )
        .field(
            "Play rate",
            format!(
                "{:.2}% ({})",
                monster_rta_info_g3.play_rate, monster_rta_info_g3.played
            ),
            true,
        )
        .field(
            "Win rate",
            format!(
                "{:.2}% ({})",
                monster_rta_info_g3.win_rate, monster_rta_info_g3.winner
            ),
            true,
        )
        .field(
            "Ban rate",
            format!(
                "{:.2}% ({})",
                monster_rta_info_g3.ban_rate, monster_rta_info_g3.banned
            ),
            true,
        )
        .field(
            "Lead rate",
            format!(
                "{:.2}% ({})",
                monster_rta_info_g3.lead_rate, monster_rta_info_g3.leader
            ),
            true,
        );

    let reply = CreateReply {
        embeds: vec![embed.clone()],
        ..Default::default()
    };
    ctx.send(reply).await?;

    send_log(
        &ctx,
        input_data,
        true,
        format!("Stats successfully sent for monster {}", monster_slug.name),
    )
    .await?;

    Ok(())
}
