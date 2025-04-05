use poise::serenity_prelude as serenity;
use poise::serenity_prelude::CreateSelectMenuKind;
use poise::CreateReply;
use serenity::{
    builder::{
        CreateActionRow, CreateEmbed, CreateEmbedFooter, CreateSelectMenu, CreateSelectMenuOption,
    },
    Error,
};

use crate::commands::player_stats::utils::{
    format_player_ld_monsters_emojis, format_player_monsters, get_user_detail, search_users,
};
use crate::{Data, API_TOKEN};

/// 📂 Displays the RTA stats of the given player. (LD & most used monsters)
///
/// Usage: `/get_player_stats`
#[poise::command(slash_command)]
pub async fn get_player_stats(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Player name"] player_name: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let token = {
        let guard = API_TOKEN.lock().unwrap();
        guard.clone().ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing API token",
            ))
        })?
    };

    let players = search_users(&token, &player_name).await.map_err(|e| {
        Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("API error: {}", e),
        ))
    })?;

    if players.is_empty() {
        ctx.say("No players found.").await?;
        return Ok(());
    }

    if players.len() == 1 {
        let details = get_user_detail(&token, &players[0].swrt_player_id).await.map_err(|e| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error retrieving player details: {}", e),
            ))
        })?;

        let embed = create_player_embed(
            &details,
            vec!["<a:loading:1358029412716515418> Loading emojis...".to_string()],
            vec!["<a:loading:1358029412716515418> Loading top monsters...".to_string()],
        );
        let reply_handle = ctx
            .send(CreateReply {
                embeds: vec![embed],
                ..Default::default()
            })
            .await?;

        let ld_emojis = format_player_ld_monsters_emojis(&details).await;
        let top_monsters = format_player_monsters(&details).await;

        let updated_embed = create_player_embed(&details, ld_emojis, top_monsters);
        reply_handle
            .edit(
                poise::Context::Application(ctx),
                CreateReply {
                    embeds: vec![updated_embed],
                    ..Default::default()
                },
            )
            .await?;

        return Ok(());
    }

    let options: Vec<CreateSelectMenuOption> = players
        .iter()
        .take(25)
        .map(|player| CreateSelectMenuOption::new(&player.name, player.swrt_player_id.to_string()))
        .collect();

    let select_menu =
        CreateSelectMenu::new("select_player", CreateSelectMenuKind::String { options });
    let action_row = CreateActionRow::SelectMenu(select_menu);

    let msg = ctx
        .send(CreateReply {
            content: Some(
                "🧙 Several players match the given username, please select a player :"
                    .to_string(),
            ),
            components: Some(vec![action_row]),
            ..Default::default()
        })
        .await?;

    let user_id = ctx.author().id;
    if let Some(component_interaction) =
        serenity::ComponentInteractionCollector::new(&ctx.serenity_context.shard)
            .filter(move |i| i.data.custom_id == "select_player" && i.user.id == user_id)
            .timeout(std::time::Duration::from_secs(30))
            .await
    {
        component_interaction
            .create_response(
                &ctx.serenity_context,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::default(),
                ),
            )
            .await?;

        // Step 1: update message to show loading
        msg.edit(
            poise::Context::Application(ctx),
            CreateReply {
                content: Some("<a:loading:1358029412716515418> Retrieving data...".to_string()),
                components: Some(vec![]),
                embeds: vec![],
                ..Default::default()
            },
        )
        .await?;

        let selected_id = if let serenity::ComponentInteractionDataKind::StringSelect { values } =
            &component_interaction.data.kind
        {
            values.get(0).cloned().unwrap_or_default()
        } else {
            String::new()
        };

        let selected_id: i64 = selected_id.parse().map_err(|_| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid player ID format",
            ))
        })?;

        let details = get_user_detail(&token, &selected_id).await.map_err(|e| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error retrieving player details: {}", e),
            ))
        })?;

        // Embed initial loading state
        let embed = create_player_embed(
            &details,
            vec!["<a:loading:1358029412716515418> Loading emojis...".to_string()],
            vec!["<a:loading:1358029412716515418> Loading top monsters...".to_string()],
        );
        msg.edit(
            poise::Context::Application(ctx),
            CreateReply {
                content: Some("".to_string()),
                embeds: vec![embed],
                ..Default::default()
            },
        )
        .await?;

        // Step 2: load emojis + monsters
        let ld_emojis = format_player_ld_monsters_emojis(&details).await;
        let top_monsters = format_player_monsters(&details).await;

        let updated_embed = create_player_embed(&details, ld_emojis, top_monsters);
        msg.edit(
            poise::Context::Application(ctx),
            CreateReply {
                content: Some("".to_string()),
                embeds: vec![updated_embed],
                ..Default::default()
            },
        )
        .await?;
    } else {
        ctx.say("⏰ Time expired or no selection.").await?;
    }

    Ok(())
}

/// Creates an embed from player info + emojis
fn create_player_embed(
    details: &crate::commands::player_stats::utils::PlayerDetail,
    ld_emojis: Vec<String>,
    top_monsters: Vec<String>,
) -> CreateEmbed {
    let format_emojis = |mut list: Vec<String>| {
        let mut result = list.join(" ");
        while result.len() > 1020 && !list.is_empty() {
            list.pop();
            result = list.join(" ");
        }
        if result.len() >= 1020 {
            result.push_str(" …");
        }
        if result.is_empty() {
            "None".to_string()
        } else {
            result
        }
    };

    let ld_display = format_emojis(ld_emojis);
    let top_display = format_emojis(top_monsters);

    let embed = CreateEmbed::default();
    embed
        .title(format!("{} RTA Statistics", details.name))
        .thumbnail(details.head_img.clone().unwrap_or_default())
        .color(serenity::Colour::from_rgb(0, 180, 255))
        .description(
            "⚠️ Stats are not 100% accurate ➡️ The very last battle is not included in the elo/rank, and people around 1300 elo will have weird stats (missing games, weird winrates) ⚠️",
        )
        .field(
            "WinRate",
            format!("{:.2} %", details.win_rate.unwrap_or(0.0) * 100.0),
            true,
        )
        .field(
            "Elo",
            details.player_score.unwrap_or(0).to_string(),
            true,
        )
        .field(
            "Rank",
            details.player_rank.unwrap_or(0).to_string(),
            true,
        )
        .field(
            "Matches Played",
            details.season_count.unwrap_or(0).to_string(),
            true,
        )
        .field("✨ LD Monsters (RTA only)", ld_display, false)
        .field("🔥 Most Used Units Winrate", top_display, false)
        .footer(CreateEmbedFooter::new(
            "Please use /send_suggestion to report any issue.",
        ))
        .clone()
}
