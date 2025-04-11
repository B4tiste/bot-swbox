#[derive(Debug, Clone)]
pub struct MonsterRtaInfoData {
    // pub monster_id: i32,
    pub monster_name: String,
    pub image_filename: String,
    pub pick_total: i32,
    pub play_rate: f32,
    pub win_rate: f32,
    pub ban_rate: f32,
    pub first_pick_rate: f32,
}
