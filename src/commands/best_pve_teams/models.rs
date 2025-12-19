use serde::Deserialize;

#[derive(Debug, Clone, poise::ChoiceParameter)]
pub enum Dungeon {
    #[name = "Giant's Keep"]
    GiantsKeep,

    #[name = "Dragon's Lair"]
    DragonsLair,

    #[name = "Necropolis"]
    Necropolis,

    #[name = "Steel Fortress"]
    SteelFortress,

    #[name = "Punisher's Crypt"]
    PunishersCrypt,

    #[name = "Spiritual Realm"]
    SpiritualRealm,

    #[name = "Karzhan - Forest of Roaring Beasts"]
    KarzhanForest,

    #[name = "Ellunia - Sanctuary of Dreaming Fairies"]
    ElluniaSanctuary,

    #[name = "Lumel - Cliff of Tough Beast Men"]
    LumelCliff,

    #[name = "Fire Beast"]
    FireBeast,

    #[name = "Dark Beast"]
    DarkBeast,
}

#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub data: Vec<DungeonTeamData>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DungeonTeamData {
    pub id: u32,

    #[serde(deserialize_with = "deserialize_members_ids")]
    pub members: Vec<String>,

    pub rank: f64,

    #[serde(deserialize_with = "deserialize_runs")]
    pub runs: Vec<RunData>,

    #[serde(default)]
    pub average_time_ms: u32,
    #[serde(default)]
    pub success_rate: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunData {
    pub success: bool,
    pub duration_ms: u32,
}

impl Dungeon {
    pub const fn id(self) -> u32 {
        match self {
            Dungeon::GiantsKeep => 8011,
            Dungeon::DragonsLair => 9011,
            Dungeon::Necropolis => 6021,
            Dungeon::SteelFortress => 9511,
            Dungeon::PunishersCrypt => 9512,
            Dungeon::SpiritualRealm => 9513,
            Dungeon::KarzhanForest => 2101,
            Dungeon::ElluniaSanctuary => 1101,
            Dungeon::LumelCliff => 3101,
            Dungeon::FireBeast => 2001,
            Dungeon::DarkBeast => 5001,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Dungeon::GiantsKeep => "Giant's Keep",
            Dungeon::DragonsLair => "Dragon's Lair",
            Dungeon::Necropolis => "Necropolis",
            Dungeon::SteelFortress => "Steel Fortress",
            Dungeon::PunishersCrypt => "Punisher's Crypt",
            Dungeon::SpiritualRealm => "Spiritual Realm",
            Dungeon::KarzhanForest => "Karzhan - Forest of Roaring Beasts",
            Dungeon::ElluniaSanctuary => "Ellunia - Sanctuary of Dreaming Fairies",
            Dungeon::LumelCliff => "Lumel - Cliff of Tough Beast Men",
            Dungeon::FireBeast => "Fire Beast",
            Dungeon::DarkBeast => "Dark Beast",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Dungeon::GiantsKeep => "giants-keep",
            Dungeon::DragonsLair => "dragons-lair",
            Dungeon::Necropolis => "necropolis",
            Dungeon::SteelFortress => "steel-fortress",
            Dungeon::PunishersCrypt => "punishers-crypt",
            Dungeon::SpiritualRealm => "spiritual-realm",
            Dungeon::KarzhanForest => "karzhan",
            Dungeon::ElluniaSanctuary => "ellunia",
            Dungeon::LumelCliff => "lumel",
            Dungeon::FireBeast => "fire-beast",
            Dungeon::DarkBeast => "dark-beast",
        }
    }
}

fn deserialize_runs<'de, D>(deserializer: D) -> Result<Vec<RunData>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Chaque run est un tableau JSON: [ts, success, duration, ignored]
    let raw: Vec<Vec<serde_json::Value>> = Vec::deserialize(deserializer)?;

    let mut out = Vec::with_capacity(raw.len());
    for row in raw {
        if row.len() < 3 {
            continue; // ou return Err(...) si tu veux être strict
        }

        let success = row[1]
            .as_bool()
            .ok_or_else(|| serde::de::Error::custom("runs[i][1] (success) must be bool"))?;

        let duration_ms = row[2]
            .as_u64()
            .ok_or_else(|| serde::de::Error::custom("runs[i][2] (duration) must be u64"))?
            as u32;

        out.push(RunData {
            success,
            duration_ms,
        });
    }

    Ok(out)
}

fn deserialize_members_ids<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let urls: Vec<String> = Vec::deserialize(deserializer)?;

    let mut ids = Vec::with_capacity(urls.len());

    for url in urls {
        match extract_monster_id(&url) {
            Some(id) => ids.push(id),
            None => {
                return Err(serde::de::Error::custom(format!(
                    "Invalid monster URL: {}",
                    url
                )));
            }
        }
    }

    Ok(ids)
}

fn extract_monster_id(url: &str) -> Option<String> {
    let filename = url.split('/').last()?;

    // unit_icon_0080_1_1-thumb.jpg
    let filename = filename.strip_prefix("unit_icon_")?;

    let core = filename.split('-').next()?; // 0080_1_1
    Some(core.to_string())
}
