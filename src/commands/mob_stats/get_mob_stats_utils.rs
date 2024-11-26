use std::vec;
use serde::Deserialize;

use log::info;
use poise::{serenity_prelude::{self as serenity}, CreateReply};
use crate::commands::{mob_stats::lib::get_monster_slug, ranks::lib::{Context, Error}};

// Struct for the API response (slug)
#[derive(Deserialize)]
pub struct SlugApiResponse {
    pub data: Vec<SlugData>,
}

#[derive(Deserialize)]
pub struct SlugData {
    pub name: String,
    pub slug: String,
}

// Struct for the API response (id)
#[derive(Deserialize)]
struct MonsterInfoApiResponse {
    data: MonsterInfoData,
}

#[derive(Deserialize)]
pub struct MonsterInfoData {
    id: i32,
    image_filename: String,
}

/// 📂 Affiche les stats du monstre donné.
///
/// Displays the stats of a given mob.
///
/// Usage: `/mob_stats <mob_name>`
#[poise::command(slash_command, prefix_command)]
pub async fn get_mob_stats(ctx: Context<'_>,#[description="Nom du monstre"] mob_name: String) -> Result<(), Error>{

    let monster_slug = get_monster_slug(mob_name).await?;

    println!("Monster slug: {}", monster_slug.slug);

    // Get the monster ID
    // let mut mob_id: i32;
    // let monster_id_url = format!("https://api.swarena.gg/monster/{}/details", mob_formatted);
    // let response = reqwest::get(monster_id_url).await?;

    // if response.status().is_success() {
    //     let api_response: MonsterInfoApiResponse = response.json().await?;

    //     // Check if their is data in the response["data"]
    //     if api_response.data.id > 0 {
    //         mob_id = api_response.data.id;

    //         info!("Monster id: {}", mob_id);
    //     }
    //     else {
    //         monster_has_stats = false;
    //     }
    // }

    // if !monster_exists {

    //     let mut embed = serenity::CreateEmbed::default().title("Stats du monstre").color(serenity::Colour::from_rgb(255, 0, 0));

    //     embed.description("Le monstre n'existe pas.");

    //     let reply = CreateReply{
    //         embeds: vec![embed],
    //         ..Default::default()
    //     };

    //     ctx.send(reply).await?;
    // }
    // else if !monster_has_stats {

    //     let mut embed = serenity::CreateEmbed::default().title("Stats du monstre").color(serenity::Colour::from_rgb(255, 0, 0));

    //     embed.description("Le monstre n'a pas de stats.");

    //     let reply = CreateReply{
    //         embeds: vec![embed],
    //         ..Default::default()
    //     };

    //     ctx.send(reply).await?;
        
    // }
    // else {

    //     // Recovery of the thumbnail of the monster
    //     let monster_thumbnail = "aze";

    //     let mut embed = serenity::CreateEmbed::default().title("Stats du monstre").color(serenity::Colour::from_rgb(0, 255, 0)).thumbnail(monster_thumbnail);

    //     let reply = CreateReply{
    //         embeds: vec![embed],
    //         ..Default::default()
    //     };

    //     ctx.send(reply).await?;
    // };

    ctx.say(monster_slug.slug).await?;
    Ok(())
}