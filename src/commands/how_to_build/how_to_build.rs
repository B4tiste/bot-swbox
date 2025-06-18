use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serenity::Error;

use crate::commands::mob_stats::get_mob_stats::autocomplete_monster;
use crate::commands::shared::embed_error_handling::{
    create_embed_error, schedule_message_deletion,
};
use crate::commands::shared::logs::send_log;
use crate::Data;

use crate::commands::how_to_build::utils::{
    format_monster_stats, load_monster_images, load_monster_stats,
};

/// 📂 Look a monster RTA build
#[poise::command(slash_command)]
pub async fn how_to_build(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[autocomplete = "autocomplete_monster"]
    #[description = "Name of the monster"]
    monster_name: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let monster_data = match load_monster_stats("assets/avg_runes/average_monster_stats.json") {
        Ok(data) => data,
        Err(_) => {
            let reply = ctx
                .send(create_embed_error("❌ Failed to load monster data"))
                .await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    let image_data = match load_monster_images("monsters_elements.json") {
        Ok(data) => data,
        Err(_) => {
            let reply = ctx
                .send(create_embed_error("❌ Failed to load monster image data"))
                .await?;
            schedule_message_deletion(reply, ctx).await?;
            return Ok(());
        }
    };

    if let Some(stats) = monster_data.get(&monster_name) {
        let image_url = image_data
            .get(&monster_name.to_lowercase())
            .map(|filename| {
                format!(
                    "https://swarfarm.com/static/herders/images/monsters/{}",
                    filename
                )
            });

        let embed = format_monster_stats(&monster_name, stats, image_url);
        ctx.send(CreateReply::default().embed(embed)).await?;
    } else {
        let msg = format!("No data found for monster: **{}**", monster_name);
        let reply = ctx.send(create_embed_error(&msg)).await?;
        schedule_message_deletion(reply, ctx).await?;
        send_log(&ctx, monster_name, false, &msg).await?;
    }

    Ok(())
}
