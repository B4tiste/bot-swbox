use poise::serenity_prelude as serenity;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};

use mongodb::bson::Document;
use mongodb::Collection;

use crate::commands::rta_core::models::MonsterStat;
use crate::commands::rta_core::utils::get_emoji_from_id;

/// Construit une ligne d'emojis pour un tier donné
pub async fn build_tier_line(
    monsters: &[MonsterStat],
    collection: &Collection<Document>,
) -> String {
    let mut line = String::new();

    for m in monsters {
        let emoji = get_emoji_from_id(collection, m.monster_id)
            .await
            .unwrap_or_default();

        if !emoji.is_empty() {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(&emoji);
        }
    }

    if line.is_empty() {
        ":no_entry_sign:".to_string()
    } else {
        line
    }
}

fn level_to_label(level: i32) -> &'static str {
    match level {
        0 => "C1-P3",
        1 => "G1-G2",
        3 => "G3",
        _ => "Inconnu",
    }
}

/// Embed principal de la méta
pub fn create_meta_embed(
    api_level: i32,
    sss_line: &str,
    ss_line: &str,
    s_line: &str,
    a_line: &str,
    b_line: &str,
    date: &str,
) -> CreateEmbed {
    let thumbnail = "https://raw.githubusercontent.com/B4tiste/landing-page-bot/refs/heads/main/src/assets/images/old_bot_logo.gif";
    let level_label = level_to_label(api_level);

    CreateEmbed::default()
        .title("📊 RTA Meta Tier List")
        .color(serenity::Colour::from_rgb(255, 255, 255))
        .thumbnail(thumbnail)
        .description(format!(
            "Displaying the current meta for rank **{}**.\n\nLast updated: **{}**",
            level_label, date
        ))
        .field("SSS", sss_line, false)
        .field("SS", ss_line, false)
        .field("S", s_line, false)
        .field("A", a_line, false)
        .field("B", b_line, false)
        .footer(CreateEmbedFooter::new(
            "Join our community on discord.gg/AfANrTVaDJ to share feedback, get support, and connect with others!",
        ))
}

/// Embed de chargement quand on change de niveau
pub fn create_loading_meta_embed(level: i32) -> CreateEmbed {
    let level_label = level_to_label(level);

    CreateEmbed::default()
        .title("📊 RTA Meta Tier List")
        .description(format!(
            "Loading meta tier list for: **{}**...\n\n<a:loading:1358029412716515418> Retrieving new tier list data...",
            level_label
        ))
        .color(serenity::Colour::from_rgb(255, 165, 0)) // Orange
        .footer(CreateEmbedFooter::new(
            "Please wait while the meta is loading...",
        ))
}

/// Boutons pour changer de niveau de méta (P1-P3, G1-G2, G3)
pub fn create_meta_level_buttons(
    guardian_id: u64,
    punisher_id: u64,
    selected_level: i32,
    disabled: bool,
) -> serenity::CreateActionRow {
    let style_for = |level| {
        if level == selected_level {
            serenity::ButtonStyle::Primary
        } else {
            serenity::ButtonStyle::Secondary
        }
    };

    serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new("meta_level_c1p3")
            .label("C1-P3")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: punisher_id.into(),
                name: Some("punisher".to_string()),
            })
            .style(style_for(0)),
        serenity::CreateButton::new("meta_level_g1g2")
            .label("G1-G2")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: guardian_id.into(),
                name: Some("guardian".to_string()),
            })
            .style(style_for(1)),
        serenity::CreateButton::new("meta_level_g3")
            .label("G3")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: guardian_id.into(),
                name: Some("guardian".to_string()),
            })
            .style(style_for(3)),
    ])
}
