use serde::{Deserialize, Serialize};

#[derive(Debug, poise::ChoiceParameter)]
pub enum Rank {
    C1,
    C2,
    C3,
    P1,
    P2,
    P3,
    G1,
    G2,
    G3,
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

/// Représente une entrée de la réponse highdata (duo pour un monstre de base)
#[derive(Deserialize, Clone)]
pub struct MonsterDuoStat {
    #[serde(rename = "teamMonsterOneId")]
    pub team_one_id: u32,
    // #[serde(rename = "teamOneImgFilename")]
    // pub team_one_img: String,
    #[serde(rename = "teamMonsterTwoId")]
    pub team_two_id: u32,
    // #[serde(rename = "teamTwoImgFilename")]
    // pub team_two_img: String,
    // #[serde(rename = "winTotal")]
    // pub win_total: u32,
    #[serde(rename = "pickTotal")]
    pub pick_total: u32,
    #[serde(rename = "winRate")]
    pub win_rate: String,
}

/// Modèle local pour un trio, avec métrique pondérée
pub struct Trio {
    pub base: u32,
    pub one: u32,
    pub two: u32,
    pub win_rate: f32,      // ex. 0.55
    pub pick_total: u32,    // ex. 1432
    pub weighted_score: f32, // win_rate * pick_total
    pub emojis: Option<String> // emojis
}