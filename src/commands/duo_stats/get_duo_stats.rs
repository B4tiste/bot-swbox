use poise::{
    serenity_prelude::{self as serenity, CreateEmbed, Error},
    Modal,
    CreateReply,
};

use crate::commands::shared::utils::{get_season, get_monster_general_info, get_monster_slug};
use crate::commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion};
use crate::commands::duo_stats::utils::{get_monsters_duo_stats, create_collage_from_urls};
use crate::commands::duo_stats::modal::DuoStatsInfosModal;

/// 📂 Affiche le winrate d'affrontement ou de coopération de deux monstres donnés
///
/// Displays the stats of a given mob.
///
/// Usage: `/get_mob_stats`
#[poise::command(slash_command)]
pub async fn get_duo_stats(ctx: poise::ApplicationContext<'_, (), Error>) -> Result<(), Error> {
    let modal_data: DuoStatsInfosModal = match DuoStatsInfosModal::execute(ctx).await {
        Ok(Some(data)) => data,
        Ok(None) => return Ok(()),
        Err(_) => {
            let error_message = "Erreur lors de l'exécution du modal.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    // Vérifier que les deux champs ne sont pas les mêmes
    if modal_data.name1.to_lowercase() == modal_data.name2.to_lowercase() {
        let error_message = "Les deux monstres ne peuvent pas être les mêmes.";
        let reply = ctx.send(create_embed_error(&error_message)).await?;
        schedule_message_deletion(reply, ctx).await?;
        return Ok(());
    }

    let mob_name_1 = modal_data.name1.as_str();
    // Récupération des 2 slugs
    let monster_1_slug = match get_monster_slug(mob_name_1.to_string()).await {
        Ok(slug) => slug,
        Err(_) => {
            let error_message = format!("Monstre 1 '**{}**' introuvable.", mob_name_1);
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let mob_name_2 = modal_data.name2.as_str();
    let monster_2_slug = match get_monster_slug(mob_name_2.to_string()).await {
        Ok(slug) => slug,
        Err(_) => {
            let error_message = format!("Monstre 2 '**{}**' introuvable.", mob_name_2);
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    // Récupération des IDs des deux monstres
    let monster_1_general_info = match get_monster_general_info(monster_1_slug.slug.clone()).await {
        Ok(info) => info,
        Err(_) => {
            let error_message = "Impossible de récupérer les informations générales du monstre.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let monster_2_general_info = match get_monster_general_info(monster_2_slug.slug.clone()).await {
        Ok(info) => info,
        Err(_) => {
            let error_message = "Impossible de récupérer les informations générales du monstre.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let season = match get_season(None).await {
        Ok(season) => season,
        Err(_) => {
            let error_message = "Impossible de récupérer la saison.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let monster_duo_stats = match get_monsters_duo_stats(monster_1_general_info.clone(), monster_2_slug.clone(), monster_2_general_info.clone(), season.clone()).await {
        Ok(info) => info,
        Err(_) => {
            let error_message = "Impossible de récupérer les statistiques des monstres";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let bdd_channel_id = serenity::ChannelId::new(1312040599263907840);

    // Créer un collage avec les images des deux monstres
    let image_urls: Vec<String> = vec![
        format!("https://swarfarm.com/static/herders/images/monsters/{}", monster_1_general_info.image_filename),
        format!("https://swarfarm.com/static/herders/images/monsters/{}", monster_duo_stats.b_monster_image_filename),
    ];
    let image_urls: Vec<&str> = image_urls.iter().map(|s| s.as_str()).collect();

    if let Err(_) = create_collage_from_urls(image_urls, "collage.png").await {
        let error_message = "Impossible de créer le collage.";
        let reply = ctx.send(create_embed_error(&error_message)).await?;
        schedule_message_deletion(reply, ctx).await?;
        return Ok(());
    }

    // Envoie du collage dans le channel BDD
    let attachment = serenity::CreateAttachment::path("collage.png").await?;
    let reply_handle = bdd_channel_id
        .send_message(&ctx.http(), serenity::CreateMessage::new().add_file(attachment))
        .await?;
    // Récupérer l'URL de l'attachement dans le message envoyé
    let attachment_url = reply_handle.attachments[0].url.clone();

    // Calcul des winrates en f32
    let with_rate_str = monster_duo_stats.win_together_rate.trim_matches('"');
    let with_winrate: f32 = with_rate_str.parse::<f32>().unwrap();

    let against_rate_str = monster_duo_stats.win_against_rate.trim_matches('"');
    let against_winrate: f32 = against_rate_str.parse::<f32>().unwrap();

    let embed = CreateEmbed::default()
        .title(format!("{} & {}", monster_1_slug.name, monster_2_slug.name))
        .thumbnail(attachment_url)
        .field(format!("WR {} avec {}", monster_1_slug.name, monster_2_slug.name), format!("{}%", with_winrate), false)
        .field(format!("WR {} contre {}", monster_1_slug.name, monster_2_slug.name), format!("{}%", against_winrate), false)
        .field("WR 🔄", format!("{}%", 100.0 - against_winrate), false);

        let reply = CreateReply {
            embeds: vec![embed],
            ..Default::default()
        };

        ctx.send(reply).await?;

    // Suppression du collage
    let _ = std::fs::remove_file("collage.png");

    Ok(())
}