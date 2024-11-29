use poise::Modal;

#[derive(Debug, Modal)]
#[name = "Entrez le nom du monstre"]
pub struct SuggestionModal {
    // Titre
    #[name = "Nom de la fonctionnalité"]
    #[placeholder = "Entrez un titre pour votre suggestion"]
    pub name: String,
    // Description
    #[name = "Description"]
    #[placeholder = "Entrez une description pour votre suggestion"]
    #[paragraph]
    pub description: String,
    // Possibilité de joindre une image
    #[name = "Joindre une image"]
    #[placeholder = "Entrez un lien vers une image"]
    pub image: Option<String>,
}
