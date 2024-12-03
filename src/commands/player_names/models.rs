#[derive(Debug, poise::ChoiceParameter)]
pub enum PlayerNamesModalData {
    Name,
    Id,
}

#[derive(Debug, Clone)]
pub struct PlayerSearchInput {
    pub id: Option<String>,
    pub name: Option<String>,
}
