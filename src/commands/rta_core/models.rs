use serde::{Deserialize, Serialize};

#[derive(Debug, poise::ChoiceParameter)]
pub enum Rank {
    #[name = "P1"]
    P1,
    #[name = "P2-P3"]
    P2P3,
    #[name = "G1-G3"]
    G1G2G3,
    #[name = "G3"]
    G3,
}

impl Rank {
    pub fn lucksack_rank(&self) -> i32 {
        match self {
            Rank::P1 => 11,
            Rank::P2P3 => 103,
            Rank::G1G2G3 => 102,
            Rank::G3 => 16,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Rank::P1 => "P1",
            Rank::P2P3 => "P2-P3",
            Rank::G1G2G3 => "G1-G3",
            Rank::G3 => "G3",
        }
    }
}

#[derive(Debug, poise::ChoiceParameter)]
pub enum Sort {
    #[name = "Most Played"]
    MostPlayed,
    #[name = "Best Winrate"]
    BestWinrate,
}

/// Représente une entrée brute du fichier monsters.json
#[derive(Deserialize)]
pub struct MonsterEntry {
    pub com2us_id: u32,
    // pub image_filename: String,
    pub element: String,
    pub awaken_level: u8,
    pub natural_stars: u8,
}

/// Wrapper si le JSON a une racine { "monsters": […] }
#[derive(Deserialize)]
pub struct MonstersFile {
    pub monsters: Vec<MonsterEntry>,
}

/// Structure finale ne gardant que les champs désirés
pub struct Monster {
    pub unit_master_id: u32,
    // pub image_filename: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TierListData {
    pub level: u8,
    #[serde(rename = "sssMonster")]
    pub sss_monster: Vec<MonsterStat>,
    #[serde(rename = "ssMonster")]
    pub ss_monster: Vec<MonsterStat>,
    #[serde(rename = "smonster")]
    pub s_monster: Vec<MonsterStat>,
    #[serde(rename = "amonster")]
    pub a_monster: Vec<MonsterStat>,
    #[serde(rename = "bmonster")]
    pub b_monster: Vec<MonsterStat>,
    #[serde(rename = "cmonster")]
    pub c_monster: Vec<MonsterStat>,
    #[serde(rename = "createDate")]
    pub date: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct MonsterStat {
    #[serde(rename = "monsterId")]
    pub monster_id: u32,
    #[serde(rename = "monsterHeadImg")]
    pub monster_head_img: String,
    #[serde(rename = "pickTotal")]
    pub pick_total: u32,
    #[serde(rename = "firstPickTotal")]
    pub first_pick_total: u32,
    #[serde(rename = "secondPickTotal")]
    pub second_pick_total: u32,
    #[serde(rename = "thirdPickTotal")]
    pub third_pick_total: u32,
    #[serde(rename = "fourthPickTotal")]
    pub fourth_pick_total: u32,
    #[serde(rename = "fifthPickTotal")]
    pub fifth_pick_total: u32,
    #[serde(rename = "lastPickTotal")]
    pub last_pick_total: u32,
}

/// Modèle local normalisé pour un trio
#[derive(Clone)]
pub struct TrioStat {
    pub ids: [u32; 3],
    pub count: u32,
    pub win_rate: f32,
}

/// Réponse statistics/trio
#[derive(Deserialize, Clone)]
pub struct LucksackTrioResponse {
    pub records: Vec<LucksackTrioRecord>,
}

#[derive(Deserialize, Clone)]
pub struct LucksackTrioRecord {
    pub monster_id: Vec<u32>,
    pub played_count: u32,
    pub win_rate: f32,
}

/// Réponse monsters/{id}/with-trio
#[derive(Deserialize, Clone)]
pub struct LucksackWithTrioResponse {
    pub records: Vec<LucksackWithTrioRecord>,
}

#[derive(Deserialize, Clone)]
pub struct LucksackWithTrioRecord {
    pub units1: LucksackUnitRef,
    pub units2: LucksackUnitRef,
    pub units3: LucksackUnitRef,
    pub appearances: u32,
    pub winrate: f32,
}

#[derive(Deserialize, Clone)]
pub struct LucksackUnitRef {
    pub monster_id: u32,
}

/// Réponse patches
#[derive(Deserialize, Clone)]
pub struct LucksackPatch {
    pub patch_id: i32,
    pub patch_order: i32,
}
