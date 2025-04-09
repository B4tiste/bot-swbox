use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serenity::{
    builder::{CreateEmbed, CreateEmbedFooter},
    Error,
};

use crate::commands::leaderboard::utils::{get_leaderboard_data, LeaderboardPlayer};
use crate::commands::player_stats::utils::{
    create_player_embed, format_player_ld_monsters_emojis, format_player_monsters, get_user_detail,
};
use crate::commands::shared::logs::send_log;
use crate::commands::shared::player_alias::PLAYER_ALIAS_MAP;
use crate::{Data, API_TOKEN};

/// 📂 Displays the RTA leaderboard
#[poise::command(slash_command)]
pub async fn get_leaderboard(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Page number to start from"] page: Option<i32>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let user_id = ctx.author().id;
    let mut page = page.unwrap_or(1).max(1);

    let token = {
        let guard = API_TOKEN.lock().unwrap();
        guard.clone().ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing API token",
            ))
        })?
    };

    let players = get_leaderboard_data(&token, &page).await.map_err(|e| {
        Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("API error: {}", e),
        ))
    })?;

    let response = ctx
        .send(CreateReply {
            embeds: vec![build_leaderboard_embed(&players, page)],
            components: Some(vec![
                create_pagination_buttons(page),
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
                        values.get(0).cloned()
                    } else {
                        None
                    };

                if let Some(id) = selected_id {
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

                    // 🧠 Étape 1 : répondre rapidement avec message "chargement"
                    interaction
            .create_response(
                &ctx.serenity_context,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("<a:loading:1358029412716515418> Retrieving player stats...")
                        .ephemeral(false),
                ),
            )
            .await?;

                    // On récupère le message d’interaction à modifier
                    let mut followup = interaction.get_response(&ctx.serenity_context).await?;

                    // Étape 2 : charger les données et mettre à jour
                    match get_user_detail(&token, &player_id).await {
                        Ok(details) => {
                            let ld_emojis = format_player_ld_monsters_emojis(&details).await;
                            let top_monsters = format_player_monsters(&details).await;
                            let embed = create_player_embed(&details, ld_emojis, top_monsters);

                            followup
                                .edit(
                                    &ctx.serenity_context,
                                    serenity::builder::EditMessage::new()
                                        .content("")
                                        .embed(embed),
                                )
                                .await?;
                        }
                        Err(e) => {
                            followup
                                .edit(
                                    &ctx.serenity_context,
                                    serenity::builder::EditMessage::new()
                                        .content(format!("❌ Failed to load player stats: {}", e)),
                                )
                                .await?;
                        }
                    }
                } else {
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
                }

                continue;
            }

            _ => continue,
        }

        let players = match get_leaderboard_data(&token, &page).await {
            Ok(p) => p,
            Err(e) => {
                ctx.say(format!("Failed to load page {}: {}", page, e))
                    .await?;
                break;
            }
        };

        interaction
            .create_response(
                &ctx.serenity_context,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .add_embed(build_leaderboard_embed(&players, page))
                        .components(vec![
                            create_pagination_buttons(page),
                            create_player_select_menu(&players),
                        ]),
                ),
            )
            .await?;
    }

    response
        .edit(
            poise::Context::Application(ctx),
            CreateReply {
                embeds: vec![build_leaderboard_embed(&players, page)],
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

    send_log(
        &ctx,
        "Command: /get_leaderboard".to_string(),
        true,
        format!("Displayed leaderboard page {}", page),
    )
    .await?;

    Ok(())
}

fn build_leaderboard_embed(players: &[LeaderboardPlayer], page: i32) -> serenity::CreateEmbed {
    let mut description = String::new();

    for (rank, player) in players.iter().enumerate() {
        let position = rank + 1 + ((page - 1) * 10) as usize;

        let alias_str = PLAYER_ALIAS_MAP
            .get(&player.swrt_player_id)
            .map(|alias| format!("aka **{}**", alias))
            .unwrap_or_default();

        description.push_str(&format!(
            "{}. :flag_{}: {} - `{}`{}\n",
            position,
            player.player_country.to_lowercase(),
            player.player_elo,
            player.name,
            alias_str,
        ));
    }

    CreateEmbed::default()
        .title(format!("Leaderboard - Page {}", page))
        .description(description)
        .field("💡 Tip", "Use the menu below to view a player's stats.", false)
        .field("⚠️ Note", "Interaction buttons are disabled after 10 minutes.", false)
        .footer(CreateEmbedFooter::new("Use /send_suggestion to report issues."))
        .color(serenity::Colour::from_rgb(0, 255, 0))
}


fn create_pagination_buttons(page: i32) -> serenity::CreateActionRow {
    let previous_button = serenity::CreateButton::new("previous_page")
        .label("⬅️ Previous")
        .style(serenity::ButtonStyle::Primary)
        .disabled(page <= 1);

    let next_button = serenity::CreateButton::new("next_page")
        .label("➡️ Next")
        .style(serenity::ButtonStyle::Primary);

    serenity::CreateActionRow::Buttons(vec![previous_button, next_button])
}

fn create_player_select_menu(players: &[LeaderboardPlayer]) -> serenity::CreateActionRow {
    let options: Vec<serenity::CreateSelectMenuOption> = players
        .iter()
        .map(|player| {
            let emoji = if player.player_country.to_uppercase() == "UNKNOWN" {
                serenity::ReactionType::Unicode("❌".to_string())
            } else {
                serenity::ReactionType::Unicode(country_code_to_flag_emoji(&player.player_country))
            };

            let label = if let Some(alias) = PLAYER_ALIAS_MAP.get(&player.swrt_player_id) {
                format!("{} aka {}", player.name, alias)
            } else {
                player.name.clone()
            };

            serenity::CreateSelectMenuOption::new(label, player.swrt_player_id.to_string())
                .description(format!("Elo: {}", player.player_elo))
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
