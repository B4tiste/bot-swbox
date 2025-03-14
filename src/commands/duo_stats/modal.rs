use poise::Modal;

#[derive(Debug, Modal)]
#[name = "Enter the names of the monsters"]
pub struct DuoStatsInfosModal {
    #[name = "Name of the first monster"]
    #[placeholder = "Enter the name of the first monster (e.g., Bella)"]
    pub name1: String,
    #[name = "Name of the second monster"]
    #[placeholder = "Enter the name of the second monster (e.g., Giana)"]
    pub name2: String,
}
