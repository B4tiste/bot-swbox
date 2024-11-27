use poise::Modal;

#[derive(Debug, Modal)]
#[name = "Entrez le nom du monstre"]
pub struct MobStatsInfosModal {
    #[name = "Nom du monstre"]
    #[placeholder = "Entrez le nom du monstre (ex: Bella)"]
    pub name: String,
    #[name = "Saison (Optionnel)"]
    #[placeholder = "Entrez la saison (ex: 31)"]
    pub season: Option<String>,
}
