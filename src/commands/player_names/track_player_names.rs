use crate::commands::player_names::modal::{
    PlayerNamesInfosModalById, PlayerNamesInfosModalByName,
};
use crate::commands::player_names::models::{PlayerNamesModalData, PlayerSearchInput};
use crate::commands::player_names::utils::{
    get_current_detail_from_swrt, get_player_all_names, get_swrt_id_from_db_by_player_id,
    handle_modal, resolve_player_id,
};
use crate::commands::shared::logs::{get_server_name, send_log};
use crate::commands::shared::models::LoggerDocument;
use crate::Data;
use poise::serenity_prelude::{CreateEmbed, Error};
use poise::CreateReply;

const DEFAULT_LOGO: &str =
    "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true";

/// 📂 Displays the different usernames this player may have had (SWARENA profile required).
///
/// Usage: /track_player_names
#[poise::command(slash_command)]
pub async fn track_player_names(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Select the search method"] mode: PlayerNamesModalData,
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

    let (_input_data, _input_status) = match &modal_result {
        Ok(Some(data)) => (format!("{:?}", data), true),
        Ok(None) => ("No input provided".to_string(), false),
        Err(_) => ("Error obtaining modal".to_string(), false),
    };

    // 1) Résoudre l’ID "SWArena" (correspond au playerId dans ta collection)
    let player_id = match resolve_player_id(ctx, modal_result).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"track_player_names".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            return Ok(());
        }
        Err(_) => {
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"track_player_names".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
            return Ok(());
        }
    };

    // 2) DB → swrtPlayerId
    let mut current_name: Option<String> = None;
    let mut head_img_url: Option<String> = None;

    if let Ok(parsed_player_id) = player_id.parse::<i64>() {
        match get_swrt_id_from_db_by_player_id(parsed_player_id).await {
            Ok(swrt_id) => {
                // 3) SWRanking → current name + headImg
                match get_current_detail_from_swrt(swrt_id).await {
                    Ok((name, head_img)) => {
                        current_name = Some(name);
                        head_img_url = head_img;
                    }
                    Err(_e) => {
                        // log doux mais on continue (fallback sur logo par défaut)
                        send_log(LoggerDocument::new(
                            &ctx.author().name,
                            &"track_player_names".to_string(),
                            &get_server_name(&ctx).await?,
                            false,
                            chrono::Utc::now().timestamp(),
                        ))
                        .await?;
                    }
                }
            }
            Err(_e) => {
                // log doux mais on continue
                send_log(LoggerDocument::new(
                    &ctx.author().name,
                    &"track_player_names".to_string(),
                    &get_server_name(&ctx).await?,
                    false,
                    chrono::Utc::now().timestamp(),
                ))
                .await?;
            }
        }
    }

    // 4) Recherche des pseudos SWArena (existant)
    let player_all_names = get_player_all_names(player_id.clone()).await;

    // Petit helper pour bâtir l’embed (sans "Current in-game name")
    let base_embed = |title: &str, description: String| {
        CreateEmbed::default()
            .title(title)
            .description(description)
            .thumbnail(
                head_img_url
                    .clone()
                    .unwrap_or_else(|| DEFAULT_LOGO.to_string()),
            )
    };

    match player_all_names {
        Ok(names) if names.is_empty() => {
            let mut embed = base_embed(
                "Username not found",
                format!(
                    "We couldn't find any usernames for the player with ID **{}**.",
                    player_id
                ),
            )
            .field(
                "Tips",
                "Check if the ID is correct or try another account.",
                false,
            )
            .color(0xff0000);

            // 👉 Ajouter le current name à la fin (s’il existe)
            if let Some(ref cname) = current_name {
                embed = embed.field("Current in-game name", cname, true);
            }

            let create_reply = CreateReply {
                embeds: vec![embed],
                ..Default::default()
            };
            ctx.send(create_reply).await?;

            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"track_player_names".to_string(),
                &get_server_name(&ctx).await?,
                false,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
        }

        Ok(names) if names.len() == 1 => {
            let mut embed = base_embed(
                "Username found",
                format!("The username for the player with ID **{}** is:", player_id),
            )
            .field("Past Username", &names[0], true)
            .color(0x00ff00);

            // 👉 Ajouter le current name après l’historique
            if let Some(ref cname) = current_name {
                embed = embed.field("Current in-game name", cname, true);
            }

            let create_reply = CreateReply {
                embeds: vec![embed],
                ..Default::default()
            };
            ctx.send(create_reply).await?;

            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"track_player_names".to_string(),
                &get_server_name(&ctx).await?,
                true,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
        }

        Ok(names) => {
            let formatted_names = names
                .iter()
                .map(|n| format!("- {}", n))
                .collect::<Vec<_>>()
                .join("\n");

            let mut embed = base_embed(
                "Usernames found",
                format!(
                    "The usernames for the player with ID **{}** are:",
                    player_id
                ),
            )
            .field("Past Usernames", formatted_names, false)
            .color(0x00ff00);

            // 👉 Ajouter le current name après la liste des pseudos
            if let Some(ref cname) = current_name {
                embed = embed.field("Current in-game name", cname, true);
            }

            let create_reply = CreateReply {
                embeds: vec![embed],
                ..Default::default()
            };
            ctx.send(create_reply).await?;

            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"track_player_names".to_string(),
                &get_server_name(&ctx).await?,
                true,
                chrono::Utc::now().timestamp(),
            ))
            .await?;
        }

        Err(_e) => {
            // Fallback: on envoie un embed "partiel" avec le current name si disponible
            let mut embed = base_embed(
                "Couldn't retrieve username history",
                "We couldn't fetch the username history from SWArena right now.".to_string(),
            )
            .field(
                "Info",
                "This player never reached G1, hence no public profile on SWArena.",
                false,
            )
            .color(0xffa500); // orange "warning"

            // 👉 Toujours ajouter le current name s’il existe (récupéré via DB → SWRanking)
            if let Some(ref cname) = current_name {
                embed = embed.field("Current in-game name", cname, true);
            }

            // Si on n'a même pas le current name, on garde quand même un message d’erreur convivial
            // (le thumbnail reste le logo par défaut via base_embed)

            let create_reply = CreateReply {
                embeds: vec![embed],
                ..Default::default()
            };
            ctx.send(create_reply).await?;

            // Logging plus informatif
            send_log(LoggerDocument::new(
                &ctx.author().name,
                &"track_player_names".to_string(),
                &get_server_name(&ctx).await?,
                current_name.is_some(),
                chrono::Utc::now().timestamp(),
            ))
            .await?;
        }
    }

    Ok(())
}
