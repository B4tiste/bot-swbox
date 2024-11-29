use crate::commands::player_names::modal::{
    PlayerNamesInfosModalById, PlayerNamesInfosModalByName,
};
use crate::commands::player_names::models::{PlayerNamesModalData, PlayerSearchInput};
use crate::commands::player_names::utils::{get_player_all_names, handle_modal, resolve_player_id};
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::send_log;
use poise::serenity_prelude::{CreateEmbed, Error};
use poise::CreateReply;

/// 📂 Affiche les différents noms d'utilisateurs que ce joueur a pu avoir.
///
/// Usage: /track_player_names
#[poise::command(slash_command)]
pub async fn track_player_names(
    ctx: poise::ApplicationContext<'_, (), Error>,
    #[description = "Sélectionnez le moyen de recherche"] mode: PlayerNamesModalData,
) -> Result<(), Error> {
    let modal_result = match mode {
        PlayerNamesModalData::Id => {
            handle_modal::<PlayerNamesInfosModalById, _>(ctx.clone(), |data| PlayerSearchInput {
                id: Some(data.id),
                name: None,
            })
            .await
        }
        PlayerNamesModalData::Name => {
            handle_modal::<PlayerNamesInfosModalByName, _>(ctx.clone(), |data| PlayerSearchInput {
                id: None,
                name: Some(data.name),
            })
            .await
        }
    };

    let (input_data, _input_status) = match &modal_result {
        Ok(Some(data)) => (format!("{:?}", data), true),
        Ok(None) => ("Aucun input fourni".to_string(), false),
        Err(_) => ("Erreur dans l'obtention du modal".to_string(), false),
    };

    let player_id = match resolve_player_id(ctx, modal_result).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            send_log(&ctx, input_data, false, "Aucun ID trouvé").await?;
            return Ok(());
        }
        Err(_) => {
            send_log(&ctx, input_data, false, "Erreur lors de la résolution").await?;
            return Ok(());
        }
    };

    let player_all_names = get_player_all_names(player_id.clone()).await;
    match player_all_names {
        Ok(names) if names.is_empty() => {
            let embed = CreateEmbed::default()
                .title("Nom d'utilisateur introuvable")
                .description(format!(
                    "Nous n'avons retrouvé aucun nom d'utilisateur pour le joueur portant l'ID **{}**.",
                    player_id
                ))
                .field("Astuces", "Vérifiez que l'ID est correct ou essayez avec un autre compte.", false)
                .color(0xff0000)
                .thumbnail("https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true");

            let create_reply = CreateReply {
                embeds: vec![embed],
                ..Default::default()
            };
            ctx.send(create_reply).await?;

            send_log(&ctx, input_data, false, "Aucun nom trouvé").await?;
        }
        Ok(names) if names.len() == 1 => {
            let embed = CreateEmbed::default()
                .title("Nom d'utilisateur trouvé")
                .description(format!(
                    "Le nom d'utilisateur pour le joueur portant l'ID **{}** est :",
                    player_id
                ))
                .field("Nom d'utilisateur", &names[0], true)
                .field("Total des noms", "1", true)
                .color(0x00ff00)
                .thumbnail(
                    "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true",
                );

            let create_reply = CreateReply {
                embeds: vec![embed],
                ..Default::default()
            };
            ctx.send(create_reply).await?;

            send_log(&ctx, input_data, true, format!("Nom trouvé: {}", names[0].clone())).await?;
        }
        Ok(names) => {
            let formatted_names = names
                .iter()
                .map(|name| format!("- {}", name))
                .collect::<Vec<String>>()
                .join("\n");

            let embed = CreateEmbed::default()
                .title("Noms d'utilisateur retrouvés")
                .description(format!(
                    "Les noms d'utilisateur pour le joueur portant l'ID **{}** sont :",
                    player_id
                ))
                .field("Noms d'utilisateurs", formatted_names, false)
                .field("Total des noms", &names.len().to_string(), true)
                .color(0x00ff00)
                .thumbnail(
                    "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true",
                );

            let create_reply = CreateReply {
                embeds: vec![embed],
                ..Default::default()
            };
            ctx.send(create_reply).await?;

            send_log(
                &ctx,
                input_data,
                true,
                format!("Noms trouvés : {}", names.join(", ")),
            )
            .await?;
        }

        Err(_) => {
            let embed =
                create_embed_error("Erreur lors de la récupération des noms d'utilisateur.");
            let reply = ctx.send(embed).await?;
            schedule_message_deletion(reply, ctx).await?;

            send_log(
                &ctx,
                input_data,
                false,
                "Erreur lors de la récupération des noms d'utilisateur.",
            ).await?;
        }
    }

    Ok(())
}
