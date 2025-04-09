use std::{collections::HashMap, fs};
use serde::Deserialize;
use lazy_static::lazy_static;

#[derive(Debug, Deserialize)]
pub struct PlayerAliasList {
    players: Vec<PlayerAlias>,
}

#[derive(Debug, Deserialize)]
pub struct PlayerAlias {
    // og_name: String,
    en_names: Vec<String>,
    // swarena_id: i64,
    swrt_id: i64,
}

lazy_static! {
    pub static ref PLAYER_ALIAS_MAP: HashMap<i64, String> = {
        let file_content = fs::read_to_string(format!(
            "{}/src/commands/shared/player_alias.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("Failed to read player_alias.json");

        let alias_list: PlayerAliasList = serde_json::from_str(&file_content)
            .expect("Failed to parse player_alias.json");

        alias_list
            .players
            .into_iter()
            .filter_map(|p| p.en_names.get(0).map(|alias| (p.swrt_id, alias.clone())))
            .collect()
    };
}