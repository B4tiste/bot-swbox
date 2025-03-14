use crate::commands::ranks::utils::info_rank_sw;
use crate::commands::shared::logs::send_log;
use crate::{
    commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion},
    Data,
};
use poise::{
    serenity_prelude::{self as serenity, Error},
    CreateReply,
};
use std::vec;
/// 📂 Affiche les montants de points des rangs (C1 -> G3)
///
/// Displays the current scores for ranks.
///
/// Usage: `/ranks`
#[poise::command(slash_command)]
pub async fn get_ranks(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    let scores = match info_rank_sw().await {
        Ok(scores) => scores,
        Err(_) => {
            let error_message = "Impossible de récupérer les informations des ELOs.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(
                &ctx,
                "Command: /ranks".to_string(),
                false,
                error_message.to_string(),
            )
            .await?;
            return Ok(());
        }
    };

    let thumbnail = "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true";

    // Construction de la description avec les en-têtes de groupe
    let groups = ["Conquerant", "Punisher", "Gardien"];
    let mut description = String::new();

    for (i, group) in groups.iter().enumerate() {
        // Ajout de l'en-tête du groupe en gras
        description.push_str(&format!("{} :\n", group));
        // Pour chaque groupe, on ajoute trois lignes "Rank : Score"
        for j in 0..3 {
            let index = i * 3 + j;
            let (rank, score) = &scores[index];
            description.push_str(&format!("{} : **{}**\n", rank, score));
        }
        description.push('\n');
    }

    // Création de l'embed en utilisant la description
    let embed = serenity::CreateEmbed::default()
        .title("ELOs actuels")
        .color(serenity::Colour::from_rgb(0, 0, 255))
        .thumbnail(thumbnail)
        .description(description);

    let reply = CreateReply {
        embeds: vec![embed],
        ..Default::default()
    };

    ctx.send(reply).await?;

    send_log(
        &ctx,
        "Command: /ranks".to_string(),
        true,
        format!("Embed envoyé"),
    )
    .await?;

    Ok(())
}
