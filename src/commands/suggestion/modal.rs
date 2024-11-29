use poise::Modal;

#[derive(Debug, Modal)]
#[name = "Entrez le nom du monstre"]
pub struct SuggestionModal {
    #[name = "Nom de la fonctionnalité"]
    #[placeholder = "Entrez un titre pour votre suggestion"]
    pub name: String,
    #[name = "Description"]
    #[placeholder = "Entrez une description pour votre suggestion"]
    #[paragraph]
    pub description: String,
    #[name = "Joindre une image"]
    #[placeholder = "Entrez un lien vers une image"]
    pub image: Option<String>,
}
