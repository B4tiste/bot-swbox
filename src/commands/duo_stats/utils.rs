use crate::commands::shared::models::{DuoStatsInfosData, MonsterGeneralInfoData, SlugData};
use image::{GenericImage, ImageBuffer};
use reqwest::Client;


pub async fn get_monsters_duo_stats(mob_a_info: MonsterGeneralInfoData, mob_b_slug: SlugData, mob_b_info: MonsterGeneralInfoData, season: i64) -> Result<DuoStatsInfosData, String> {
    let monster_duo_stats_url = format!("https://api.swarena.gg/monster/{}/pairs?season={}&isG3=false&searchPairName={}&orderBy=win_against_rate&orderDirection=DESC&minPlayedAgainst=0&minPlayedTogether=0&limit=5&offset=0", mob_a_info.id, season, mob_b_slug.slug.to_lowercase());
    let response = reqwest::get(monster_duo_stats_url).await.map_err(|_| "Failed to send request".to_string())?;

    if response.status().is_success() {
        let api_response: serde_json::Value = response.json().await.map_err(|_| "Failed to parse JSON".to_string())?;

        // Vérifie que les données sont présentes
        if !api_response["data"].is_null() {
            // Trouver l'entrée avec le bon b_monster_id
            for i in 0..api_response["data"].as_array().unwrap().len() {
                let data = &api_response["data"][i];
                if data["b_monster_id"].as_i64().unwrap() == mob_b_info.id as i64 {
                    return Ok(DuoStatsInfosData {
                        b_monster_image_filename: data["b_monster_image_filename"].as_str().ok_or("Missing b_monster_image_filename")?.to_string(),
                        win_against_rate: data["win_against_rate"].to_string(),
                        win_together_rate: data["win_together_rate"].to_string(),
                    });
                }
            }
        }
    }

    Err("Data not found".to_string())
}

pub async fn create_collage_from_urls(image_urls: Vec<&str>, output_path: &str) -> Result<(), String> {
    // Créer un client HTTP asynchrone
    let client = Client::new();

    // Télécharger les images depuis les URLs de manière asynchrone
    let mut images = Vec::new();
    for url in image_urls {
        let response = client.get(url).send().await.map_err(|e| e.to_string())?; // Requête HTTP asynchrone
        let bytes = response.bytes().await.map_err(|e| e.to_string())?; // Lire la réponse comme des octets
        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?; // Charger l'image en mémoire
        images.push(img);
    }

    // Calculer la taille du collage
    let width = images[0].width() * images.len() as u32; // Alignement horizontal
    let height = images[0].height();

    // Créer une nouvelle image pour le collage
    let mut collage = ImageBuffer::new(width, height);

    // Coller chaque image dans le collage
    for (i, img) in images.iter().enumerate() {
        collage.copy_from(img, i as u32 * img.width(), 0).map_err(|e| e.to_string())?;
    }

    // Sauvegarder le collage
    collage.save(output_path).map_err(|e| e.to_string())?;

    Ok(())
}