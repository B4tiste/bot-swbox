use poise::Modal;

#[derive(Debug, Modal)]
#[name = "Enter the player's username"]
pub struct PlayerNamesInfosModalByName {
    #[name = "Player's username"]
    #[placeholder = "Enter the player's username (e.g., Reynca)"]
    pub name: String,
}

#[derive(Debug, Modal)]
#[name = "Enter the player's ID"]
pub struct PlayerNamesInfosModalById {
    #[name = "Player's ID"]
    #[placeholder = "Enter the player's ID (e.g., 123456789)"]
    pub id: String,
}
