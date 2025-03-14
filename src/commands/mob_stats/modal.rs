use poise::Modal;

#[derive(Debug, Modal)]
#[name = "Enter the monster's name"]
pub struct MobStatsInfosModal {
    #[name = "Monster's name"]
    #[placeholder = "Enter the monster's name (e.g., Bella)"]
    pub name: String,
    #[name = "Season (Optional)"]
    #[placeholder = "Enter the season (e.g., 32)"]
    pub season: Option<String>,
}
