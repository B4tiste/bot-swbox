use std::vec;
use serde::Deserialize;

use poise::{serenity_prelude::{self as serenity}, CreateReply};
use crate::commands::{mob_stats::lib::{get_latest_season, get_monster_general_info, get_monster_rta_info, get_monster_slug}, ranks::lib::{Context, Error}};

#[derive(Deserialize, Clone)]
pub struct SlugData {
    pub name: String,
    pub slug: String,
}

#[derive(Deserialize)]
pub struct MonsterGeneralInfoData {
    pub id: i32,
    pub image_filename: String,
}

#[derive(Deserialize)]
pub struct MonsterRtaInfoData {
    pub played: i32,
    pub winner: i32,
    pub banned: i32,
    pub leader: i32,
    pub play_rate: f32,
    pub win_rate: f32,
    pub ban_rate: f32,
    pub lead_rate: f32
}

/// 📂 Affiche les stats du monstre donné.
///
/// Displays the stats of a given mob.
///
/// Usage: `/mob_stats <mob_name>`
#[poise::command(slash_command, prefix_command)]
pub async fn get_mob_stats(ctx: Context<'_>,#[description="Nom du monstre"] mob_name: String) -> Result<(), Error>{

    let monster_slug = get_monster_slug(mob_name).await?;
    let monster_general_info = get_monster_general_info(monster_slug.slug).await?;
    let latest_season = get_latest_season().await?;
    let monster_rta_info_no_g3 = get_monster_rta_info(monster_general_info.id.to_string(), latest_season, false).await?;
    let monster_rta_info_g3 = get_monster_rta_info(monster_general_info.id.to_string(), latest_season, true).await?;

    let thumbnail = format!("https://swarfarm.com/static/herders/images/monsters/{}", monster_general_info.image_filename);

    // Création de l'embed avec les données "no G3" et "G3"
    let embed = serenity::CreateEmbed::default()
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

        let reply = CreateReply{
            embeds: vec![embed],
            ..Default::default()
        };

        ctx.send(reply).await?;

    Ok(())
}