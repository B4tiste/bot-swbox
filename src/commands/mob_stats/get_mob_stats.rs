use poise::{
    serenity_prelude::{self as serenity, CreateEmbed, Error},
    Modal,
    CreateReply,
};

use crate::GUARDIAN_EMOJI_ID;
use crate::commands::shared::utils::{get_season, get_monster_general_info, get_monster_slug};
use crate::commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion};
use crate::commands::mob_stats::lib::get_monster_rta_info;
use crate::commands::mob_stats::modal::ChampionsInfosModal;

/// 📂 Affiche les stats du monstre donné.
///
/// Displays the stats of a given mob.
///
/// Usage: `/mob_stats <mob_name>`
#[poise::command(slash_command)]
pub async fn get_mob_stats(ctx: poise::ApplicationContext<'_, (), Error>) -> Result<(), Error> {
    let modal_data: ChampionsInfosModal = match ChampionsInfosModal::execute(ctx).await {
        Ok(Some(data)) => data,
        Ok(None) => return Ok(()),
        Err(_) => {
            let error_message = "Erreur lors de l'exécution du modal.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    //Récupération des informations du monstre avec gestion des erreurs
    let monster_slug = match get_monster_slug(modal_data.name).await {
        Ok(slug) => slug,
        Err(_) => {
            let error_message = "Monstre introuvable.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let monster_general_info = match get_monster_general_info(monster_slug.slug.clone()).await {
        Ok(info) => info,
        Err(_) => {
            let error_message = "Impossible de récupérer les informations générales du monstre.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let season = match get_season(modal_data.season).await {
        Ok(season) => season,
        Err(_) => {
            let error_message = "Impossible de récupérer la saison.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let monster_rta_info_no_g3 = match get_monster_rta_info(monster_general_info.id.to_string(), season, false).await {
        Ok(info) => info,
        Err(_) => {
            let error_message = "Impossible de récupérer les informations RTA (No G3).";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let monster_rta_info_g3 = match get_monster_rta_info(monster_general_info.id.to_string(), season, true).await {
        Ok(info) => info,
        Err(_) => {
            let error_message = "Impossible de récupérer les informations RTA (G3).";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let thumbnail = format!("https://swarfarm.com/static/herders/images/monsters/{}", monster_general_info.image_filename);
    let guardian_emote_str = format!("<:guardian:{}>", GUARDIAN_EMOJI_ID.lock().unwrap());

    let embed = CreateEmbed::default()
        .title(format!("Stats du monstre {} - Saison {}", monster_slug.name, season))
        .color(serenity::Colour::from_rgb(255, 0, 255))
        .thumbnail(thumbnail)
        .field("**Stats (All ranks) :**", "", false)
        .field("Play rate", format!("{:.2}% ({})", monster_rta_info_no_g3.play_rate, monster_rta_info_no_g3.played), true)
        .field("Win rate", format!("{:.2}% ({})", monster_rta_info_no_g3.win_rate, monster_rta_info_no_g3.winner), true)
        .field("Ban rate", format!("{:.2}% ({})", monster_rta_info_no_g3.ban_rate, monster_rta_info_no_g3.banned), true)
        .field("Lead rate", format!("{:.2}% ({})", monster_rta_info_no_g3.lead_rate, monster_rta_info_no_g3.leader), true)
        .field("", "", false)
        .field(format!("**Stats {}**", guardian_emote_str.repeat(3)), "", false)
        .field("Play rate", format!("{:.2}% ({})", monster_rta_info_g3.play_rate, monster_rta_info_g3.played), true)
        .field("Win rate", format!("{:.2}% ({})", monster_rta_info_g3.win_rate, monster_rta_info_g3.winner), true)
        .field("Ban rate", format!("{:.2}% ({})", monster_rta_info_g3.ban_rate, monster_rta_info_g3.banned), true)
        .field("Lead rate", format!("{:.2}% ({})", monster_rta_info_g3.lead_rate, monster_rta_info_g3.leader), true);

    let reply = CreateReply {
        embeds: vec![embed],
        ..Default::default()
    };

    ctx.send(reply).await?;

    Ok(())
}