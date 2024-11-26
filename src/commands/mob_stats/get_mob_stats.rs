use poise::serenity_prelude::CreateEmbed;

use poise::{serenity_prelude::{self as serenity}, CreateReply};
use crate::commands::{embed_error_handling::{create_embed_error, schedule_message_deletion}, mob_stats::lib::{get_latest_season, get_monster_general_info, get_monster_rta_info, get_monster_slug}, ranks::lib::{Context, Error}};



/// 📂 Affiche les stats du monstre donné.
///
/// Displays the stats of a given mob.
///
/// Usage: `/mob_stats <mob_name>`
#[poise::command(slash_command, prefix_command)]
pub async fn get_mob_stats(ctx: Context<'_>, #[description = "Nom du monstre"] mob_name: String) -> Result<(), Error> {
    // Récupération des informations du monstre avec gestion des erreurs
    let monster_slug = match get_monster_slug(mob_name).await {
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

    let latest_season = match get_latest_season().await {
        Ok(season) => season,
        Err(_) => {
            let error_message = "Impossible de récupérer la dernière saison.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let monster_rta_info_no_g3 = match get_monster_rta_info(monster_general_info.id.to_string(), latest_season, false).await {
        Ok(info) => info,
        Err(_) => {
            let error_message = "Impossible de récupérer les informations RTA (No G3).";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let monster_rta_info_g3 = match get_monster_rta_info(monster_general_info.id.to_string(), latest_season, true).await {
        Ok(info) => info,
        Err(_) => {
            let error_message = "Impossible de récupérer les informations RTA (G3).";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    // Construction de l'embed
    let thumbnail = format!("https://swarfarm.com/static/herders/images/monsters/{}", monster_general_info.image_filename);

    let embed = CreateEmbed::default()
        .title(format!("Stats du monstre {}", monster_slug.name))
        .color(serenity::Colour::from_rgb(255, 0, 255))
        .thumbnail(thumbnail)
        .field("**Stats (No G3) :**", "", false)
        .field("Play rate", format!("{:.2}% ({})", monster_rta_info_no_g3.play_rate, monster_rta_info_no_g3.played), true)
        .field("Win rate", format!("{:.2}% ({})", monster_rta_info_no_g3.win_rate, monster_rta_info_no_g3.winner), true)
        .field("Ban rate", format!("{:.2}% ({})", monster_rta_info_no_g3.ban_rate, monster_rta_info_no_g3.banned), true)
        .field("Lead rate", format!("{:.2}% ({})", monster_rta_info_no_g3.lead_rate, monster_rta_info_no_g3.leader), true)
        .field("", "", false)
        .field("**Stats (G3) :**", "", false)
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