use poise::Modal;

#[derive(Debug, Modal)]
#[name = "Suggestion"]
pub struct SuggestionModal {
    #[name = "Feature name"]
    #[placeholder = "Enter a title for your suggestion"]
    pub name: String,
    #[name = "Description"]
    #[placeholder = "Enter a description for your suggestion"]
    #[paragraph]
    pub description: String,
    #[name = "Attach an image"]
    #[placeholder = "Enter a link to an image"]
    pub image: Option<String>,
}
