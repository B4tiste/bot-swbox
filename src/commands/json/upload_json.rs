use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::send_log;
use crate::Data;
use poise::serenity_prelude::{Attachment, Error};
use reqwest;

/// 📂 Upload a JSON file and return its name along with a preview of its content
///
/// Usage: `/upload_json`
#[poise::command(slash_command)]
pub async fn upload_json(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    file: Option<Attachment>,
) -> Result<(), Error> {
    // Vérifier qu'un fichier a bien été fourni
    let file = match file {
        Some(f) => f,
        None => {
            let error_message = "No file provided. Please attach a JSON file.";
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(
                &ctx,
                "Command: /upload_json".to_string(),
                false,
                error_message.to_string(),
            )
            .await?;
            return Ok(());
        }
    };

    // Vérifier l'extension du fichier
    if !file.filename.to_lowercase().ends_with(".json") {
        let error_message = "The provided file is not a JSON file.";
        let reply = ctx.send(create_embed_error(&error_message)).await?;
        schedule_message_deletion(reply, ctx).await?;
        send_log(
            &ctx,
            "Command: /upload_json".to_string(),
            false,
            error_message.to_string(),
        )
        .await?;
        return Ok(());
    }

    // Télécharger le contenu du fichier via son URL
    let response = match reqwest::get(&file.url).await {
        Ok(resp) => resp,
        Err(e) => {
            let error_message = format!("Failed to download the file: {}", e);
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(
                &ctx,
                "Command: /upload_json".to_string(),
                false,
                error_message,
            )
            .await?;
            return Ok(());
        }
    };

    let content = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            let error_message = format!("Failed to read the file content: {}", e);
            let reply = ctx.send(create_embed_error(&error_message)).await?;
            schedule_message_deletion(reply, ctx).await?;
            send_log(
                &ctx,
                "Command: /upload_json".to_string(),
                false,
                error_message,
            )
            .await?;
            return Ok(());
        }
    };

    // Extraire les 10 premiers mots pour une prévisualisation
    let preview = content
        .split_whitespace()
        .take(10)
        .collect::<Vec<_>>()
        .join(" ");

    // Envoyer le nom du fichier et la prévisualisation sur Discord
    ctx.say(format!(
        "File name: {}\nContent preview: {}",
        file.filename, preview
    ))
    .await?;
    send_log(
        &ctx,
        "Command: /upload_json".to_string(),
        true,
        format!("File {} received with preview", file.filename),
    )
    .await?;
    Ok(())
}
