use serde::Deserialize;

/// Représente une entrée brute du fichier monsters.json
#[derive(Deserialize)]
pub struct MonsterEntry {
    pub com2us_id: u32,
    pub image_filename: String,
    pub element: String,
    pub awaken_level: u8,
    pub natural_stars: u8,
    pub name: String,
}

/// Wrapper si le JSON a une racine { "monsters": […] }
#[derive(Deserialize)]
pub struct MonstersFile {
    pub monsters: Vec<MonsterEntry>,
}

/// Structure finale ne gardant que les champs désirés
pub struct Monster {
    pub unit_master_id: u32,
    pub image_filename: String,
    pub element: String,
    pub awaken_level: u8,
    pub natural_stars: u8,
    pub name: String,
}