use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::send_log;
use crate::Data;
use poise::serenity_prelude::{Attachment, Error};
use reqwest;
use std::process::Command;
use tokio::{fs, task};

/// 📂 Upload a JSON file, execute "./runes PATHTOJSONFILE" and return the result
///
/// Usage: `/upload_json`
#[poise::command(slash_command)]
pub async fn upload_json(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    file: Option<Attachment>,
) -> Result<(), Error> {
    // Check that a file has been provided
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

    // Check the file extension
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

    // Download the file content
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

    // Save the content to a temporary file
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(&file.filename);
    if let Err(e) = fs::write(&file_path, &content).await {
        let error_message = format!("Failed to write file to disk: {}", e);
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

    // Execute the external command "./runes PATHTOJSONFILE"
    // Using spawn_blocking to avoid blocking the async runtime
    let file_path_clone = file_path.clone();
    let output = match task::spawn_blocking(move || {
        Command::new("src/commands/json/runes")
            .arg(file_path_clone)
            .output()
    })
    .await
    {
        Ok(res) => match res {
            Ok(output) => output,
            Err(e) => {
                let error_message = format!("Failed to execute command: {}", e);
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
        },
        Err(e) => {
            let error_message = format!("Task failed: {}", e);
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

    // Remove the temporary file (ignore errors)
    let _ = fs::remove_file(&file_path).await;

    // If the command failed, return the stderr as an error message
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_message = format!("Command failed: {}", stderr);
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

    // Convert the command's stdout to a string and send it as the reply
    let result = String::from_utf8_lossy(&output.stdout);
    ctx.say(format!("Command output:\n{}", result)).await?;
    send_log(
        &ctx,
        "Command: /upload_json".to_string(),
        true,
        format!("Executed './runes' successfully"),
    )
    .await?;
    Ok(())
}
