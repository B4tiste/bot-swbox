use serde::Deserialize;

#[derive(Deserialize, Clone, Default)]
pub struct SlugData {
    pub name: String,
    pub slug: String,
}

#[derive(Deserialize, Default)]
pub struct MonsterGeneralInfoData {
    pub id: i32,
    pub image_filename: String,
}

#[derive(Deserialize, Default)]
pub struct MonsterRtaInfoData {
    pub played: i32,
    pub winner: i32,
    pub banned: i32,
    pub leader: i32,
    pub play_rate: f32,
    pub win_rate: f32,
    pub ban_rate: f32,
    pub lead_rate: f32,
}