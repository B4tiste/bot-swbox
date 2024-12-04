use poise::Modal;

#[derive(Debug, Modal)]
#[name = "Entrez le pseudo du joueur"]
pub struct PlayerNamesInfosModalByName {
    #[name = "Pseudo du joueur"]
    #[placeholder = "Entrez le pseudo du joueur (ex: Reynca)"]
    pub name: String,
}

#[derive(Debug, Modal)]
#[name = "Entrez l'ID du joueur"]
pub struct PlayerNamesInfosModalById {
    #[name = "ID du joueur"]
    #[placeholder = "Entrez l'ID du joueur (ex: 123456789)"]
    pub id: String,
}
