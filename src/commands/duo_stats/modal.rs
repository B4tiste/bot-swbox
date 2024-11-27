use poise::Modal;

#[derive(Debug, Modal)]
#[name = "Entrez les noms des monstres"]
pub struct DuoStatsInfosModal {
    #[name = "Nom du premier monstre"]
    #[placeholder = "Entrez le nom du premier monstre (ex: Bella)"]
    pub name1: String,
    #[name = "Nom du deuxième monstre"]
    #[placeholder = "Entrez le nom du deuxième monstre (ex: Bella)"]
    pub name2: String
}
