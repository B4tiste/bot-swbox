use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serenity::{
    builder::{CreateEmbed, CreateEmbedFooter},
    Error,
};

use crate::commands::leaderboard::utils::{get_leaderboard_data, LeaderboardPlayer};
use crate::commands::player_stats::command::show_player_stats;
use crate::commands::player_stats::utils::get_lucksack_season_numbers;
use crate::commands::shared::logs::get_server_name;
use crate::commands::shared::logs::send_log;
use crate::commands::shared::models::LoggerDocument;
use crate::commands::shared::player_alias::PLAYER_ALIAS_MAP;
use crate::Data;

/// 📂 Displays the RTA leaderboard
#[poise::command(slash_command)]
pub async fn get_rta_leaderboard(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Page number to start from"] page: Option<i32>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let user_id = ctx.author().id;
    let mut page = page.unwrap_or(1).max(1);
    const PAGE_SIZE: i32 = 10;

    let seasons = get_lucksack_season_numbers().await.map_err(|e| {
        Error::from(std::io::Error::other(format!(
            "Failed to fetch seasons: {}",
            e
        )))
    })?;
    let season = seasons
        .first()
        .copied()
        .ok_or_else(|| Error::from(std::io::Error::other("No valid season found.")))?;

    let leaderboard = get_leaderboard_data(season, page, PAGE_SIZE)
        .await
        .map_err(|e| Error::from(std::io::Error::other(format!("API error: {}", e))))?;

    let mut players = leaderboard.data;
    let total_count = leaderboard.count;

    let response = ctx
        .send(CreateReply {
            embeds: vec![build_leaderboard_embed(&players, page, total_count)],
            components: Some(vec![
                create_pagination_buttons(page, total_count, PAGE_SIZE),
                create_player_select_menu(&players),
            ]),
            ..Default::default()
        })
        .await?;

    let message_id = response.message().await?.id;
    let channel_id = ctx.channel_id();

    while let Some(interaction) =
        serenity::ComponentInteractionCollector::new(&ctx.serenity_context.shard)
            .channel_id(channel_id)
            .message_id(message_id)
            .filter(move |i| i.user.id == user_id)
            .timeout(std::time::Duration::from_secs(600))
            .await
    {
        match interaction.data.custom_id.as_str() {
            "previous_page" if page > 1 => page -= 1,
            "next_page" => page += 1,

            "leaderboard_player_select" => {
                let selected_id =
                    if let serenity::ComponentInteractionDataKind::StringSelect { values } =
                        &interaction.data.kind
                    {
                        values.first()
                    } else {
                        None
                    };

                let Some(id) = selected_id else {
                    interaction
                        .create_response(
                            &ctx.serenity_context,
                            serenity::CreateInteractionResponse::Message(
                                serenity::CreateInteractionResponseMessage::new()
                                    .content("❌ No player selected.")
                                    .ephemeral(false),
                            ),
                        )
                        .await?;
                    continue;
                };

                let player_id: i64 = match id.parse() {
                    Ok(pid) => pid,
                    Err(_) => {
                        interaction
                            .create_response(
                                &ctx.serenity_context,
                                serenity::CreateInteractionResponse::Message(
                                    serenity::CreateInteractionResponseMessage::new()
                                        .content("❌ Invalid player ID format.")
                                        .ephemeral(false),
                                ),
                            )
                            .await?;
                        continue;
                    }
                };

                interaction
                    .create_response(
                        &ctx.serenity_context,
                        serenity::CreateInteractionResponse::Message(
                            serenity::CreateInteractionResponseMessage::new()
                                .content(
                                    "<a:loading:1358029412716515418> Retrieving player stats...",
                                )
                                .ephemeral(false),
                        ),
                    )
                    .await?;

                if let Err(e) = show_player_stats(&ctx, player_id, None).await {
                    ctx.say(format!("❌ Failed to load player stats: {}", e))
                        .await?;
                }

                continue;
            }

            _ => continue,
        }

        let leaderboard = match get_leaderboard_data(season, page, PAGE_SIZE).await {
            Ok(p) => p,
            Err(e) => {
                ctx.say(format!("Failed to load page {}: {}", page, e))
                    .await?;
                break;
            }
        };

        players = leaderboard.data;

        interaction
            .create_response(
                &ctx.serenity_context,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .add_embed(build_leaderboard_embed(&players, page, total_count))
                        .components(vec![
                            create_pagination_buttons(page, total_count, PAGE_SIZE),
                            create_player_select_menu(&players),
                        ]),
                ),
            )
            .await?;
    }

    // Disable buttons after timeout
    response
        .edit(
            poise::Context::Application(ctx),
            CreateReply {
                embeds: vec![build_leaderboard_embed(&players, page, total_count)],
                components: Some(vec![serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new("previous_page")
                        .label("⬅️ Previous")
                        .style(serenity::ButtonStyle::Primary)
                        .disabled(true),
                    serenity::CreateButton::new("next_page")
                        .label("➡️ Next")
                        .style(serenity::ButtonStyle::Primary)
                        .disabled(true),
                ])]),
                ..Default::default()
            },
        )
        .await?;

    send_log(LoggerDocument::new(
        &ctx.author().name,
        "get_leaderboard",
        &get_server_name(&ctx).await?,
        true,
        chrono::Utc::now().timestamp(),
    ))
    .await?;

    Ok(())
}

fn build_leaderboard_embed(
    players: &[LeaderboardPlayer],
    page: i32,
    total_count: i64,
) -> serenity::CreateEmbed {
    let mut description = String::new();

    for player in players {
        let alias_str = PLAYER_ALIAS_MAP
            .get(&player.player_id)
            .map(|alias| format!(" aka **{}**", alias))
            .unwrap_or_default();

        description.push_str(&format!(
            "{}. :flag_{}: {} - `{}`{}\n",
            player.rank,
            player.country.to_lowercase(),
            player.current_score,
            player.username,
            alias_str,
        ));
    }

    CreateEmbed::default()
        .title(format!("Leaderboard - Page {}", page))
        .description(description)
        .field("Players", total_count.to_string(), true)
        .field(
            "💡 Tip",
            "Use the menu below to view a player's stats.",
            false,
        )
        .field(
            "⚠️ Note",
            "Interaction buttons are disabled after 10 minutes.",
            false,
        )
        .footer(CreateEmbedFooter::new("Data is gathered from lucksack.gg"))
        .color(serenity::Colour::from_rgb(0, 255, 0))
}

fn create_pagination_buttons(
    page: i32,
    total_count: i64,
    page_size: i32,
) -> serenity::CreateActionRow {
    let last_page = ((total_count + page_size as i64 - 1) / page_size as i64).max(1) as i32;

    let previous_button = serenity::CreateButton::new("previous_page")
        .label("⬅️ Previous")
        .style(serenity::ButtonStyle::Primary)
        .disabled(page <= 1);

    let next_button = serenity::CreateButton::new("next_page")
        .label("➡️ Next")
        .style(serenity::ButtonStyle::Primary)
        .disabled(page >= last_page);

    serenity::CreateActionRow::Buttons(vec![previous_button, next_button])
}

fn create_player_select_menu(players: &[LeaderboardPlayer]) -> serenity::CreateActionRow {
    let options: Vec<serenity::CreateSelectMenuOption> = players
        .iter()
        .map(|player| {
            let emoji = if player.country.to_uppercase() == "UNKNOWN" {
                serenity::ReactionType::Unicode("❌".to_string())
            } else {
                serenity::ReactionType::Unicode(country_code_to_flag_emoji(&player.country))
            };

            let label = if let Some(alias) = PLAYER_ALIAS_MAP.get(&player.player_id) {
                format!("{} aka {}", player.username, alias)
            } else {
                player.username.clone()
            };

            serenity::CreateSelectMenuOption::new(label, player.player_id.to_string())
                .description(format!(
                    "Rank #{} | Elo: {}",
                    player.rank, player.current_score
                ))
                .emoji(emoji)
        })
        .collect();

    let select_menu = serenity::CreateSelectMenu::new(
        "leaderboard_player_select",
        serenity::CreateSelectMenuKind::String { options },
    )
    .placeholder("📊 Select a player to view stats");

    serenity::CreateActionRow::SelectMenu(select_menu)
}

fn country_code_to_flag_emoji(country_code: &str) -> String {
    country_code
        .to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| char::from_u32(0x1F1E6 + (c as u32 - 'A' as u32)).unwrap_or('∅'))
        .collect()
}
