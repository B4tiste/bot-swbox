use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LucksackSeason {
    pub season_number: Option<i32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LucksackBuildResponse {
    pub rune_sets: Vec<LucksackRuneSet>,
    pub slot_stats: Vec<LucksackSlotStats>,

    pub artifact_type: Vec<LucksackArtifactStat>,
    pub artifact_arch: Vec<LucksackArtifactStat>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LucksackRuneSet {
    pub primary_set: i32,
    pub secondary_set: Option<i32>,
    pub tertiary_set: Option<i32>,
    pub pickrate: f32,
    pub winrate: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LucksackSlotStats {
    pub slot_two: i32,
    pub slot_four: i32,
    pub slot_six: i32,
    pub pickrate: f32,
    pub winrate: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LucksackArtifactStat {
    pub effect_id: i32,
    pub pickrate: f32,
}
