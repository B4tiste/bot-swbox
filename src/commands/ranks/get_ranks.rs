use std::vec;
use poise::{
    serenity_prelude::{self as serenity, Error},
    CreateReply,
};
use crate::{commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion}, Data};
use crate::commands::ranks::utils::info_rank_sw;
use crate::commands::shared::logs::send_log;
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
            send_log(&ctx, "Command: /ranks".to_string(), false, error_message.to_string()).await?;
            return Ok(());
        }
    };

    let thumbnail = "https://github.com/B4tiste/SWbox/blob/master/src/assets/logo.png?raw=true";
    let mut embed = serenity::CreateEmbed::default()
        .title("ELOs actuels")
        .color(serenity::Colour::from_rgb(0, 0, 255))
        .thumbnail(thumbnail);

    for (rank, score) in &scores {
        embed = embed.field(rank, score.to_string(), true);
    }

    let reply = CreateReply {
        embeds: vec![embed.clone()],
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
