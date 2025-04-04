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
    format_player_emojis_only, format_player_monsters, get_user_detail, search_users,
};
use crate::{Data, API_TOKEN};

#[poise::command(slash_command)]
pub async fn get_player_stats(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "Nom du joueur"] pseudo: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let token = {
        let guard = API_TOKEN.lock().unwrap();
        guard.clone().ok_or_else(|| {
            Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Token API manquant",
            ))
        })?
    };

    let joueurs = search_users(&token, &pseudo).await.map_err(|e| {
        Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Erreur API: {}", e),
        ))
    })?;

    if joueurs.is_empty() {
        ctx.say("Aucun joueur trouvé.").await?;
        return Ok(());
    }

    if joueurs.len() == 1 {
        let details = get_user_detail(&token, &joueurs[0].swrt_player_id)
            .await
            .map_err(|e| {
                Error::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Erreur lors de la récupération des détails du joueur: {}",
                        e
                    ),
                ))
            })?;

        // Embed sans emojis d'abord
        let embed = create_player_embed(
            &details,
            vec!["Chargement des emojis...".to_string()],
            vec!["Chargement des top monstres...".to_string()],
        );
        let reply_handle = ctx
            .send(CreateReply {
                embeds: vec![embed],
                ..Default::default()
            })
            .await?;

        // ✅ Ensuite, on récupère les emojis
        let ld_emojis = format_player_emojis_only(&details).await;
        let top_monsters = format_player_monsters(&details).await;

        // ✅ Et on édite le message pour mettre à jour
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

    let options: Vec<CreateSelectMenuOption> = joueurs
        .iter()
        .take(25)
        .map(|player| CreateSelectMenuOption::new(&player.name, player.swrt_player_id.to_string()))
        .collect();

    let select_menu =
        CreateSelectMenu::new("select_player", CreateSelectMenuKind::String { options });
    let action_row = CreateActionRow::SelectMenu(select_menu);

    let msg = ctx
        .send(CreateReply {
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

        msg.edit(
            poise::Context::Application(ctx),
            CreateReply {
                content: Some("⏳ Récupération des données...".to_string()),
                components: Some(vec![]),
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
                format!(
                    "Erreur lors de la récupération des détails du joueur: {}",
                    e
                ),
            ))
        })?;

        // Embed sans emojis d'abord
        let embed = create_player_embed(
            &details,
            vec!["Chargement des emojis...".to_string()],
            vec!["Chargement des top monstres...".to_string()],
        );
        let reply_handle = ctx
            .send(CreateReply {
                embeds: vec![embed],
                ..Default::default()
            })
            .await?;

        // ✅ Ensuite, on récupère les emojis
        let ld_emojis = format_player_emojis_only(&details).await;
        let top_monsters = format_player_monsters(&details).await;

        // ✅ Et on édite le message pour mettre à jour
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
    } else {
        ctx.say("⏰ Temps écoulé ou aucune sélection.").await?;
    }

    Ok(())
}

/// Crée un embed à partir des infos joueur + emojis
fn create_player_embed(
    details: &crate::commands::player_stats::utils::PlayerDetail,
    ld_emojis: Vec<String>,
    top_monsters: Vec<String>,
) -> CreateEmbed {
    // Join emojis safely
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
            "Aucun".to_string()
        } else {
            result
        }
    };

    let ld_display = format_emojis(ld_emojis);
    let top_display = format_emojis(top_monsters);

    let embed = CreateEmbed::default();
    embed
        .title(format!("Statistiques du joueur {}", details.name))
        .thumbnail(details.headImg.clone().unwrap_or_default())
        .color(serenity::Colour::from_rgb(0, 180, 255))
        .field(
            "WinRate",
            format!("{:.2}%", details.winRate.unwrap_or(0.0) * 100.0),
            true,
        )
        .field("Score", details.playerScore.unwrap_or(0).to_string(), true)
        .field("Rank", details.playerRank.unwrap_or(0).to_string(), true)
        .field("✨ Monstres LD", ld_display, false)
        .field("🔥 Monstres Joués", top_display, false)
        .footer(CreateEmbedFooter::new("Données issues de SWRanking.com"))
        .clone()
}
