use std::thread::current;

use poise::reply;
use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serenity::builder::{EditInteractionResponse, EditAttachments};
use serenity::{CreateInteractionResponse, CreateInteractionResponseMessage, Error};

use crate::commands::mob_stats::utils::create_level_buttons;
use crate::commands::mob_stats::utils::remap_monster_id;
use crate::commands::player_stats::utils::create_replay_image;
use crate::commands::replays::utils::{create_replays_embed, get_replays_data, create_loading_replays_embed};

use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::send_log;
use crate::{Data, API_TOKEN, CONQUEROR_EMOJI_ID, GUARDIAN_EMOJI_ID, PUNISHER_EMOJI_ID};

// Import de la map des monstres
use crate::MONSTER_MAP;

/// Autocomplete basé sur MONSTER_MAP
async fn autocomplete_monster<'a>(
    _ctx: poise::ApplicationContext<'a, Data, Error>,
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    let lower = partial.to_ascii_lowercase();
    MONSTER_MAP
        .keys()
        .filter(move |name| name.to_ascii_lowercase().contains(&lower))
        .take(10)
        .cloned()
}

/// 📂 Affiche les replays contenant une sélection
#[poise::command(slash_command)]
pub async fn get_replays(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[autocomplete = "autocomplete_monster"]
    #[description = "Monster 1"]
    monster1: String,
    #[autocomplete = "autocomplete_monster"]
    #[description = "Monster 2 (optional)"]
    monster2: Option<String>,
    #[autocomplete = "autocomplete_monster"]
    #[description = "Monster 3 (optional)"]
    monster3: Option<String>,
    #[autocomplete = "autocomplete_monster"]
    #[description = "Monster 4 (optional)"]
    monster4: Option<String>,
    #[autocomplete = "autocomplete_monster"]
    #[description = "Monster 5 (optional)"]
    monster5: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    // Récupérer les IDs des monstres via l'input
    let mut monster_ids = vec![];
    let mut monster_names = vec![monster1];
    if let Some(m2) = monster2 {
        monster_names.push(m2);
    }
    if let Some(m3) = monster3 {
        monster_names.push(m3);
    }
    if let Some(m4) = monster4 {
        monster_names.push(m4);
    }
    if let Some(m5) = monster5 {
        monster_names.push(m5);
    }

    let user_id = ctx.author().id;
    let input_data = format!(
        "User ID: {}\nMonsters: {}",
        user_id,
        monster_names.join(", ")
    );
    let monster_names_for_log = monster_names.clone();

    for name in &monster_names {
        match MONSTER_MAP.get(name) {
            Some(&id) => monster_ids.push(id as i32),
            None => {
                let msg = format!(
                    "❌ Cannot find '{}', please use the autocomplete feature for a perfect match.",
                    name
                );
                let reply = ctx.send(create_embed_error(&msg)).await?;
                schedule_message_deletion(reply, ctx).await?;
                send_log(&ctx, input_data, false, &msg).await?;
                return Ok(());
            }
        }
    }

    // Token pour les requetes
    let token = {
        let guard = API_TOKEN.lock().unwrap();
        guard.clone().ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing API token",
            ))
        })?
    };

    // Remap des IDs si nécessaire
    for id in &mut monster_ids {
        *id = remap_monster_id(*id);
    }

    let mut current_level = 1;

    // Récupérer les données initiales
    let replays = get_replays_data(&monster_ids, current_level)
        .await
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let replay_image_path = create_replay_image(replays, &token, 4, 4)
        .await
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    // Create attachment for the replay image
    let attachment = serenity::CreateAttachment::path(&replay_image_path).await?;

    let embed = create_replays_embed(&monster_names, current_level);

    let conqueror_id: u64 = CONQUEROR_EMOJI_ID.lock().unwrap().parse().unwrap();
    let guardian_id: u64 = GUARDIAN_EMOJI_ID.lock().unwrap().parse().unwrap();
    let punisher_id: u64 = PUNISHER_EMOJI_ID.lock().unwrap().parse().unwrap();

    let reply = ctx
        .send(CreateReply {
            embeds: vec![embed],
            attachments: vec![attachment],
            components: Some(vec![create_level_buttons(
                conqueror_id,
                guardian_id,
                punisher_id,
                current_level,
                false,
            )]),
            ..Default::default()
        })
        .await?;

    let message_id = reply.message().await?.id;
    let channel_id = ctx.channel_id();

    // Boucle de gestion des interactions avec les boutons
    while let Some(interaction) =
        serenity::ComponentInteractionCollector::new(&ctx.serenity_context.shard)
            .channel_id(channel_id)
            .message_id(message_id)
            .filter(move |i| i.user.id == user_id)
            .timeout(std::time::Duration::from_secs(600))
            .await
    {
        let selected_level = match interaction.data.custom_id.as_str() {
            "level_c1c3" => 0,
            "level_g1g2" => 1,
            "level_g3" => 3,
            "level_p1p3" => 4,
            _ => continue,
        };

        if selected_level == current_level {
            continue;
        }

        current_level = selected_level;

        // Afficher l'embed de chargement
        let loading_embed = create_loading_replays_embed(&monster_names, current_level);

        interaction
            .create_response(
                &ctx.serenity_context,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(loading_embed)
                        .components(vec![create_level_buttons(
                            conqueror_id,
                            guardian_id,
                            punisher_id,
                            current_level,
                            true, // Boutons désactivés pendant le chargement
                        )]),
                ),
            )
            .await?;

        // Récupérer les nouvelles données
        let new_replays = match get_replays_data(&monster_ids, current_level).await {
            Ok(data) => data,
            Err(e) => {
                interaction
                    .edit_response(
                        &ctx.serenity_context.http,
                        EditInteractionResponse::new()
                            .content(format!("❌ Error fetching replay data: {}", e))
                            .components(vec![])
                            .embeds(vec![]),
                    )
                    .await?;
                continue;
            }
        };

        // Créer la nouvelle image
        let new_replay_image_path = match create_replay_image(new_replays, &token, 4, 4).await {
            Ok(path) => path,
            Err(e) => {
                interaction
                    .edit_response(
                        &ctx.serenity_context.http,
                        EditInteractionResponse::new()
                            .content(format!("❌ Error creating replay image: {}", e))
                            .components(vec![])
                            .embeds(vec![]),
                    )
                    .await?;
                continue;
            }
        };

        // Créer le nouvel attachment
        let new_attachment = serenity::CreateAttachment::path(&new_replay_image_path).await?;

        // Créer l'embed final
        let final_embed = create_replays_embed(&monster_names, current_level);

        // Note: Discord ne permet pas de modifier les attachments via edit_response
        // Il faut créer un nouveau message ou utiliser une approche différente
        // Pour le moment, on met à jour juste l'embed sans l'image
        interaction
            .edit_response(
                &ctx.serenity_context.http,
                EditInteractionResponse::new()
                    .embeds(vec![final_embed])
                    .components(vec![create_level_buttons(
                        conqueror_id,
                        guardian_id,
                        punisher_id,
                        current_level,
                        false, // Boutons réactivés
                    )])
                    .attachments(EditAttachments::new().add(new_attachment))
            )
            .await?;

        // Optionnel: Envoyer un message de suivi avec la nouvelle image
        // ou implémenter une logique pour remplacer le message entier
    }

    send_log(
        &ctx,
        "Command: /get_replays".to_string(),
        true,
        format!(
            "User ID: {}\nMonsters: {}",
            user_id,
            monster_names_for_log.join(", ")
        ),
    )
    .await?;

    Ok(())
}