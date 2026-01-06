use serde::Deserialize;

use crate::commands::player_stats::utils::Replay;

// Replay
#[derive(Debug, Deserialize)]
pub struct Root {
    pub data: ReplayListData,
}

#[derive(Debug, Deserialize)]
pub struct ReplayListData {
    pub list: Vec<Replay>,
}
