use crate::commands::player_names::modal::{
    PlayerNamesInfosModalById, PlayerNamesInfosModalByName,
};
use crate::commands::player_names::models::{PlayerNamesModalData, PlayerSearchInput};
use crate::commands::player_names::utils::{
    get_current_detail_from_swrt, get_player_all_names, get_swrt_id_from_db_by_player_id,
    handle_modal, resolve_player_id,
};
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::send_log;
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

    let (input_data, _input_status) = match &modal_result {
        Ok(Some(data)) => (format!("{:?}", data), true),
        Ok(None) => ("No input provided".to_string(), false),
        Err(_) => ("Error obtaining modal".to_string(), false),
    };

    // 1) Résoudre l’ID "SWArena" (correspond au playerId dans ta collection)
    let player_id = match resolve_player_id(ctx, modal_result).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            send_log(&ctx, input_data, false, "No ID found").await?;
            return Ok(());
        }
        Err(_) => {
            send_log(&ctx, input_data, false, "Error resolving ID").await?;
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
                    Err(e) => {
                        // log doux mais on continue (fallback sur logo par défaut)
                        send_log(
                            &ctx,
                            format!("player_id={parsed_player_id}"),
                            false,
                            format!("SWRanking detail error: {e}"),
                        )
                        .await
                        .ok();
                    }
                }
            }
            Err(e) => {
                // log doux mais on continue
                send_log(
                    &ctx,
                    format!("player_id={parsed_player_id}"),
                    false,
                    format!("DB lookup error: {e}"),
                )
                .await
                .ok();
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

            send_log(&ctx, input_data, false, "No names found").await?;
        }

        Ok(names) if names.len() == 1 => {
            let mut embed = base_embed(
                "Username found",
                format!("The username for the player with ID **{}** is:", player_id),
            )
            .field("Username", &names[0], true)
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

            send_log(
                &ctx,
                input_data,
                true,
                format!("Name found: {}", names[0].clone()),
            )
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
            .field("Usernames", formatted_names, false)
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

            send_log(
                &ctx,
                input_data,
                true,
                format!("Names found: {}", names.join(", ")),
            )
            .await?;
        }

        Err(_) => {
            let embed = create_embed_error("Error retrieving usernames.");
            let reply = ctx.send(embed).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(&ctx, input_data, false, "Error retrieving usernames.").await?;
        }
    }

    Ok(())
}
