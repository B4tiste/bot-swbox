use serde::Deserialize;

#[derive(Deserialize)]
pub struct MonsterStats {
    #[serde(rename = "HP")]
    pub hp: String,
    #[serde(rename = "ATK")]
    pub atk: String,
    #[serde(rename = "DEF")]
    pub def: String,
    #[serde(rename = "SPD")]
    pub speed: String,
    #[serde(rename = "CRate")]
    pub crit_rate: String,
    #[serde(rename = "CDmg")]
    pub crit_damage: String,
    #[serde(rename = "RES")]
    pub resistance: String,
    #[serde(rename = "ACC")]
    pub accuracy: String,
    #[serde(rename = "Set1")]
    pub set1: Option<String>,
    #[serde(rename = "Set2")]
    pub set2: Option<String>,
    #[serde(rename = "Set3")]
    pub set3: Option<String>,
    #[serde(rename = "Arti 1")]
    pub arti_1: Option<Vec<String>>,

    #[serde(rename = "Arti 2")]
    pub arti_2: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct MonsterElementData {
    pub name: String,
    pub image_filename: String,
}

#[derive(Deserialize)]
pub struct MonsterElementList {
    pub monsters: Vec<MonsterElementData>,
}