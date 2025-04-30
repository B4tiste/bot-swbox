use crate::commands::shared::logs::send_log;
use crate::commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion};
use crate::Data;
use poise::serenity_prelude::{Attachment, Error};

use crate::commands::rta_core::utils::get_monsters_from_json_bytes;

/// Commande get_rta_core
#[poise::command(slash_command)]
pub async fn get_rta_core(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    file: Attachment,
) -> Result<(), Error> {
    // Évite le timeout de 3 s
    ctx.defer().await?;

    // Vérification présence de fichier
    if file.url.is_empty() {
        let err = "No file provided. Please attach a JSON file.";
        let reply = ctx.send(create_embed_error(err)).await?;
        schedule_message_deletion(reply, ctx).await?;
        // On passe un &str pour T et &str pour G, les deux implémentent Debug
        send_log(&ctx, "Command: /get_rta_core", false, err).await?;
        return Ok(());
    }

    // Vérification extension
    if !file.filename.to_lowercase().ends_with(".json") {
        let err = "The provided file is not a JSON file.";
        let reply = ctx.send(create_embed_error(err)).await?;
        schedule_message_deletion(reply, ctx).await?;
        send_log(&ctx, "Command: /get_rta_core", false, err).await?;
        return Ok(());
    }

    // Téléchargement
    let bytes = match file.download().await {
        Ok(b) => b,
        Err(e) => {
            let err_msg = format!("Impossible de télécharger : {}", e);
            let reply = ctx.send(create_embed_error(&err_msg)).await?;
            schedule_message_deletion(reply, ctx).await?;
            // On passe &err_msg pour G (String implémente Debug)
            send_log(&ctx, "Command: /get_rta_core", false, &err_msg).await?;
            return Ok(());
        }
    };

    // Extraction des monsters
    match get_monsters_from_json_bytes(&bytes, "monsters.json") {
        Ok(monsters) => {
            // Ajouter la liste des monstres
            let mut msg = String::new();
            for monster in monsters {
                msg.push_str(&format!(
                    "ID: {}, Nom: {}, Éveil: {}, Étoiles: {}, Élément: {}\n",
                    monster.unit_master_id,
                    monster.name,
                    monster.awaken_level,
                    monster.natural_stars,
                    monster.element
                ));
            }
            // truncate le message si trop long
            if msg.len() > 1000 {
                msg.truncate(1000);
                msg.push_str("\n... (message tronqué)");
            }
            let _reply = ctx.say(msg).await?;
        }
        Err(e) => {
            let err_msg = format!("Erreur : {}", e);
            let reply = ctx.send(create_embed_error(&err_msg)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(&ctx, "Command: /get_rta_core", false, &err_msg).await?;
        }
    }

    Ok(())
}
