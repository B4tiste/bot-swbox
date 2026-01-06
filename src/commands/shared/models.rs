// use serde::Deserialize;

// #[derive(Deserialize, Clone, Default)]
// pub struct SlugData {
//     pub name: String,
//     pub slug: String,
// }

// #[derive(Deserialize, Clone, Default)]
// pub struct MonsterGeneralInfoData {
//     pub id: i32,
//     pub image_filename: String,
// }

// #[derive(Deserialize, Default)]
// pub struct MonsterRtaInfoData {
//     pub played: i32,
//     pub winner: i32,
//     pub banned: i32,
//     pub leader: i32,
//     pub play_rate: f32,
//     pub win_rate: f32,
//     pub ban_rate: f32,
//     pub lead_rate: f32,
// }

// #[derive(Deserialize, Default)]
// pub struct DuoStatsInfosData {
//     pub b_monster_image_filename: String,
//     pub win_against_rate: String,
//     pub win_together_rate: String,
// }

use serde::{Deserialize, Serialize};

#[derive(Debug, poise::ChoiceParameter)]
pub enum Mode {
    Classic,
    NoSpeedDetail,
    Anonymized,
    NoSpeedDetailAndAnonymized,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoggerDocument {
    pub username: String,
    pub command_name: String,
    pub server_name: String,
    pub command_result: bool,
    pub created_at: i64,
}

impl LoggerDocument {
    pub fn new(
        username: &str,
        command_name: &str,
        server_name: &str,
        command_result: bool,
        created_at: i64,
    ) -> Self {
        Self {
            username: username.to_string(),
            command_name: command_name.to_string(),
            server_name: server_name.to_string(),
            command_result,
            created_at,
        }
    }
}
