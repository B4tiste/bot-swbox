use crate::commands::player_names::modal::{
    PlayerNamesInfosModalById, PlayerNamesInfosModalByName,
};
use crate::commands::player_names::models::{PlayerNamesModalData, PlayerSearchInput};
use crate::commands::player_names::utils::{get_player_all_names, handle_modal, resolve_player_id};
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::send_log;
use crate::Data;
use poise::serenity_prelude::{CreateEmbed, Error};
use poise::CreateReply;

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

    let player_all_names = get_player_all_names(player_id.clone()).await;
    match player_all_names {
        Ok(names) if names.is_empty() => {
            let embed = CreateEmbed::default()
                .title("Username not found")
                .description(format!(
                    "We couldn't find any usernames for the player with ID **{}**.",
                    player_id
                ))
                .field(
                    "Tips",
                    "Check if the ID is correct or try another account.",
                    false,
                )
                .color(0xff0000)
                .thumbnail(
                    "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true",
                );

            let create_reply = CreateReply {
                embeds: vec![embed],
                ..Default::default()
            };
            ctx.send(create_reply).await?;

            send_log(&ctx, input_data, false, "No names found").await?;
        }
        Ok(names) if names.len() == 1 => {
            let embed = CreateEmbed::default()
                .title("Username found")
                .description(format!(
                    "The username for the player with ID **{}** is:",
                    player_id
                ))
                .field("Username", &names[0], true)
                .field("Total names", "1", true)
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
                format!("Name found: {}", names[0].clone()),
            )
            .await?;
        }
        Ok(names) => {
            let formatted_names = names
                .iter()
                .map(|name| format!("- {}", name))
                .collect::<Vec<String>>()
                .join("\n");

            let embed = CreateEmbed::default()
                .title("Usernames found")
                .description(format!(
                    "The usernames for the player with ID **{}** are:",
                    player_id
                ))
                .field("Usernames", formatted_names, false)
                .field("Total names", &names.len().to_string(), true)
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
