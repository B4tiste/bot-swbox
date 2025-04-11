use crate::commands::mob_stats::models::{MonsterMatchup, MonsterRtaInfoData};
use crate::commands::player_stats::utils::{get_emoji_from_filename, get_mob_emoji_collection};
use poise::serenity_prelude as serenity;
use reqwest::Client;

pub async fn get_monster_stats_swrt(
    monster_id: i32,
    season: i64,
    version: &str,
    token: &str,
    level: i32,
) -> Result<MonsterRtaInfoData, String> {
    let url = format!(
        "https://m.swranking.com/api/monster/statistical?season={}&version={}&monsterId={}&level={}&real=0",
        season, version, monster_id, level
    );

    let client = Client::new();
    let response = client
        .get(&url)
        .header("Authentication", token)
        .header("Referer", "https://m.swranking.com/")
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .map_err(|_| "Failed to send request to SWRT".to_string())?;

    let body = response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| "Failed to parse SWRT JSON".to_string())?;

    if body["retCode"] == 0
        && body["data"]["list"].is_array()
        && !body["data"]["list"].as_array().unwrap().is_empty()
    {
        let item = &body["data"]["list"][0];

        return Ok(MonsterRtaInfoData {
            // monster_id: item["monsterId"].as_i64().unwrap_or_default() as i32,
            monster_name: item["monsterName"].as_str().unwrap_or_default().to_string(),
            image_filename: item["imageFilename"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            pick_total: item["pickTotal"].as_i64().unwrap_or_default() as i32,
            play_rate: item["pickRate"].as_f64().unwrap_or_default() as f32,
            win_rate: item["winRate"].as_f64().unwrap_or_default() as f32,
            ban_rate: item["banRate"].as_f64().unwrap_or_default() as f32,
            first_pick_rate: item["firstPickRate"].as_f64().unwrap_or_default() as f32,
        });
    }

    Err("No data returned from SWRT API".to_string())
}

pub async fn get_swrt_settings(token: &str) -> Result<(i64, String), String> {
    let url = "https://m.swranking.com/api/setting/settingMap";

    let client = Client::new();
    let response = client
        .get(url)
        .header("Authentication", token)
        .header("Referer", "https://m.swranking.com/")
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .map_err(|_| "Failed to contact SWRT for settings".to_string())?;

    let json = response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| "Failed to parse SWRT settings JSON".to_string())?;

    let season_str = json["data"]["nowSeason"]
        .as_str()
        .ok_or("Missing nowSeason".to_string())?;

    let version = json["data"]["nowVersion"]
        .as_str()
        .ok_or("Missing nowVersion".to_string())?
        .to_string();

    let season = season_str
        .trim_start_matches('S')
        .parse::<i64>()
        .map_err(|_| "Invalid season format".to_string())?;

    Ok((season, version))
}

pub async fn get_monster_matchups_swrt(
    monster_id: i32,
    season: i64,
    version: &str,
    level: i32,
    token: &str,
) -> Result<(Vec<MonsterMatchup>, Vec<MonsterMatchup>), String> {
    let url = format!(
        "https://m.swranking.com/api/monster/highdata?pageNum=1&pageSize=10&monsterId={}&season={}&version={}&level={}&factor=0.01&real=0",
        monster_id, season, version, level
    );

    let client = Client::new();
    let res = client
        .get(&url)
        .header("Authentication", token)
        .header("Referer", "https://m.swranking.com/")
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .map_err(|_| "Failed to fetch matchup data".to_string())?;

    let json = res
        .json::<serde_json::Value>()
        .await
        .map_err(|_| "Invalid matchup JSON".to_string())?;

    let collection = get_mob_emoji_collection()
        .await
        .map_err(|_| "DB error".to_string())?;

    let high =
        extract_matchups_from_json(&json["data"]["highOneWithTwoList"], &collection, true).await;
    let low =
        extract_matchups_from_json(&json["data"]["lowOneVsTwoList"], &collection, false).await;

    Ok((high, low))
}

pub async fn extract_matchups_from_json(
    arr: &serde_json::Value,
    collection: &mongodb::Collection<mongodb::bson::Document>,
    is_high: bool, // ✅ ajoute ce bool pour différencier les structures
) -> Vec<MonsterMatchup> {
    let mut result = vec![];

    if let Some(list) = arr.as_array() {
        for item in list {
            let (img1_field, img2_field) = if is_high {
                ("teamOneImgFilename", "teamTwoImgFilename")
            } else {
                ("oppoOneImgFilename", "oppoTwoImgFilename")
            };

            let emoji1 = get_emoji_from_filename(
                collection,
                item.get(img1_field).and_then(|v| v.as_str()).unwrap_or(""),
            )
            .await;

            let emoji2 = get_emoji_from_filename(
                collection,
                item.get(img2_field).and_then(|v| v.as_str()).unwrap_or(""),
            )
            .await;

            let win_rate = item
                .get("winRate")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.0)
                * 100.0;

            result.push(MonsterMatchup {
                emoji1,
                emoji2,
                win_rate
            });
        }
    }

    result
}

fn truncate_field(s: String, max_len: usize) -> String {
    if s.len() > max_len {
        let mut truncated = s.chars().take(max_len - 3).collect::<String>();
        truncated.push_str("...");
        truncated
    } else {
        s
    }
}

pub fn format_good_matchups(monster_emoji: &str, matchups: &[MonsterMatchup]) -> String {
    if matchups.is_empty() {
        return "No good teammates data.".to_string();
    }

    let entries: Vec<String> = matchups
        .iter()
        .enumerate()
        .map(|(i, m)| {
            format!(
                "{}. {} {} {} **{:.2}%WR**",
                i + 1,
                monster_emoji,
                m.emoji1.clone().unwrap_or("❓".to_string()),
                m.emoji2.clone().unwrap_or("❓".to_string()),
                m.win_rate
            )
        })
        .collect();

    truncate_field(entries.join("\n"), 1024)
}

pub fn format_bad_matchups(monster_emoji: &str, matchups: &[MonsterMatchup]) -> String {
    if matchups.is_empty() {
        return "No bad matchup data.".to_string();
    }

    let entries: Vec<String> = matchups
        .iter()
        .enumerate()
        .map(|(i, m)| {
            format!(
                "{}. {} {} 🆚 {} **{:.2}%WR**",
                i + 1,
                m.emoji1.clone().unwrap_or("❓".to_string()),
                m.emoji2.clone().unwrap_or("❓".to_string()),
                monster_emoji,
                m.win_rate
            )
        })
        .collect();

    truncate_field(entries.join("\n"), 1024)
}

pub async fn build_monster_stats_embed(
    monster_stats: &MonsterRtaInfoData,
    season: i64,
    level: i32,
) -> serenity::CreateEmbed {
    let thumbnail = format!(
        "https://swarfarm.com/static/herders/images/monsters/{}",
        monster_stats.image_filename
    );

    let level_str = match level {
        0 => "C1-C3",
        1 => "G1-G2",
        3 => "G3",
        4 => "P1-P3",
        _ => "Unknown",
    };

    serenity::CreateEmbed::default()
        .title(format!(
            "Monster stats {} - Season {}",
            monster_stats.monster_name, season
        ))
        .description(format!("Level: {}", level_str))
        .color(serenity::Colour::from_rgb(0, 255, 128))
        .thumbnail(thumbnail)
        .field(
            "Play Rate",
            format!(
                "{:.2}% ({})",
                monster_stats.play_rate * 100.0,
                monster_stats.pick_total
            ),
            true,
        )
        .field(
            "Win Rate",
            format!("{:.2}%", monster_stats.win_rate * 100.0),
            true,
        )
        .field(
            "Ban Rate",
            format!("{:.2}%", monster_stats.ban_rate * 100.0),
            true,
        )
        .field(
            "First Pick Rate",
            format!("{:.2}%", monster_stats.first_pick_rate * 100.0),
            true,
        )
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Use /send_suggestion to report issues.",
        ))
}

pub fn create_level_buttons(
    conqueror_id: u64,
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
        serenity::CreateButton::new("level_c1c3")
            .label("C1-C3")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: conqueror_id.into(),
                name: Some("conqueror".to_string()),
            })
            .style(style_for(0)),
        serenity::CreateButton::new("level_p1p3")
            .label("P1-P3")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: punisher_id.into(),
                name: Some("punisher".to_string()),
            })
            .style(style_for(4)),
        serenity::CreateButton::new("level_g1g2")
            .label("G1-G2")
            .disabled(disabled)
            .emoji(serenity::ReactionType::Custom {
                animated: false,
                id: guardian_id.into(),
                name: Some("guardian".to_string()),
            })
            .style(style_for(1)),
        serenity::CreateButton::new("level_g3")
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
