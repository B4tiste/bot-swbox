use crate::commands::upload_json::rune::Property;
use crate::commands::upload_json::rune::Rune;
use crate::commands::upload_json::utils::{
    get_rune_set_id_by_id, get_rune_stat_id_by_id, get_stars_ammount_by_id,
};
use serde_json::Value;
use std::collections::HashMap;

/// Fonction qui extrait une rune à partir d'un objet JSON
fn extract_rune(rune: &Value) -> Option<Rune> {
    let class = rune.get("class")?.as_u64()? as u32;
    if class < 5 {
        return None;
    }

    let id = rune.get("rune_id")?.as_u64()? as u32;
    let slot_location = rune.get("slot_no")?.as_u64()? as u32;
    let class_enum = get_stars_ammount_by_id(class);
    let antic = class / 10 == 1;
    let set_id = get_rune_set_id_by_id(rune.get("set_id")?.as_u64()? as u32);
    let upgrade_limit = rune.get("upgrade_limit")?.as_u64()? as u32;
    let upgrade_current = rune.get("upgrade_curr")?.as_u64()? as u32;
    if upgrade_current < 12 {
        return None;
    }

    let primary_property = if let Some(pri_eff) = rune.get("pri_eff") {
        let pri_eff_array = pri_eff.as_array()?;
        let stat_id = get_rune_stat_id_by_id(pri_eff_array[0].as_u64()? as u32);
        let value = pri_eff_array[1].as_f64()? as f32;
        Property::new(stat_id, value, None, None)
    } else {
        Property::default()
    };

    let innate_property = if let Some(prefix_eff) = rune.get("prefix_eff") {
        let prefix_eff_array = prefix_eff.as_array()?;
        let stat_id = get_rune_stat_id_by_id(prefix_eff_array[0].as_u64()? as u32);
        let value = prefix_eff_array[1].as_f64()? as f32;
        Property::new(stat_id, value, None, None)
    } else {
        Property::default()
    };

    let mut secondary_properties: Vec<Property> = Vec::new();
    if let Some(sec_eff) = rune.get("sec_eff") {
        let sec_eff_array = sec_eff.as_array()?;
        for sec_eff in sec_eff_array {
            let sec_eff_array_p = sec_eff.as_array()?;
            let stat_id = get_rune_stat_id_by_id(sec_eff_array_p[0].as_u64()? as u32);
            let value = sec_eff_array_p[1].as_f64()? as f32;
            let has_been_replaced = sec_eff_array_p[2].as_u64()? == 1;
            let boost_value = sec_eff_array_p[3].as_u64()? as f32;
            secondary_properties.push(Property::new(
                stat_id,
                value,
                Some(has_been_replaced),
                Some(boost_value),
            ));
        }
    }

    Some(Rune::new(
        id,
        slot_location,
        class_enum,
        antic,
        set_id,
        upgrade_limit,
        upgrade_current,
        primary_property,
        innate_property,
        secondary_properties,
    ))
}

/// Fonction qui traite un objet JSON et retourne un tuple contenant le score, les statistiques de runes et les informations du joueur
pub fn process_json(
    json: Value,
) -> (
    f32,
    f32,
    f32,
    f32,
    HashMap<String, HashMap<String, u32>>,
    HashMap<String, HashMap<String, u32>>,
    HashMap<&'static str, Value>,
    HashMap<&'static str, Value>,
) {
    let mut vec_runes: Vec<Rune> = Vec::new();
    if let Some(unit_list) = json.get("unit_list") {
        for unit in unit_list.as_array().expect("unit_list should be an array") {
            if let Some(runes) = unit.get("runes") {
                for rune in runes.as_array().expect("runes should be an array") {
                    if let Some(parsed_rune) = extract_rune(rune) {
                        vec_runes.push(parsed_rune);
                    }
                }
            }
        }
    }
    if let Some(runes) = json.get("runes") {
        for rune in runes.as_array().expect("runes should be an array") {
            if let Some(parsed_rune) = extract_rune(rune) {
                vec_runes.push(parsed_rune);
            }
        }
    }
    let mut wizard_info_data = HashMap::new();
    if let Some(wizard_info) = json.get("wizard_info") {
        if let Some(wizard_name) = wizard_info.get("wizard_name") {
            wizard_info_data.insert("wizard_name", wizard_name.clone());
        }
        if let Some(wizard_id) = wizard_info.get("wizard_id") {
            wizard_info_data.insert("wizard_id", wizard_id.clone());
        }
        if let Some(wizard_last_login) = wizard_info.get("wizard_last_login") {
            wizard_info_data.insert("wizard_last_login", wizard_last_login.clone());
        }
    }
    let mut account_info_data = HashMap::new();
    if let Some(account_info) = json.get("account_info") {
        if let Some(channel_uid) = account_info.get("channel_uid") {
            account_info_data.insert("channel_uid", channel_uid.clone());
        }
    }

    // Coefficients globaux
    let global_efficiency_coeffs = serde_json::json!({
        "100": 0.5,
        "110": 2,
        "120": 3,
        "130": 4,
    });
    let global_speed_coeffs = serde_json::json!({
        "24": 0.5,
        "26": 1,
        "30": 3,
        "34": 4,
        "36": 6
    });

    // Coefficients RTA
    let rta_set_eff_coeffs = serde_json::json!({
        "Despair": 2,
        "Swift": 1,
        "Violent": 3,
        "Will": 3,
        "Intangible": 3
    });
    let rta_set_spd_coeffs = serde_json::json!({
        "Despair": 2,
        "Swift": 4,
        "Intangible": 4,
        "Violent": 3,
        "Will": 3,
    });

    // Coefficients Siege
    let siege_set_eff_coeffs = serde_json::json!({
        "Despair": 2,
        "Swift": 1,
        "Violent": 3,
        "Will": 3,
        "Intangible": 3,
        "Destroy": 3,
        "Shield": 2,
        "Seal": 2,
        "Nemesis" : 2
    });
    let siege_set_spd_coeffs = serde_json::json!({
        "Despair": 2,
        "Swift": 4,
        "Intangible": 4,
        "Violent": 3,
        "Will": 3,
        "Destroy": 3,
        "Shield": 3,
        "Seal" : 2,
        "Nemesis" : 2
    });

    // --- Initialisation des scores ---
    let mut rta_score_eff: f32 = 0.0; // global_efficiency_coeff * rta_set_eff_coeff
    let mut siege_score_eff: f32 = 0.0; // global_efficiency_coeff * siege_set_eff_coeff
    let mut rta_score_spd: f32 = 0.0; // global_speed_coeff * rta_set_spd_coeff
    let mut siege_score_spd: f32 = 0.0; // global_speed_coeff * siege_set_spd_coeff

    // --- Initialisation des maps de statistiques ---
    let mut map_score_eff: HashMap<String, HashMap<String, u32>> = HashMap::new(); // Old map
    let mut map_score_spd: HashMap<String, HashMap<String, u32>> = HashMap::new(); // Old map

    for rune in vec_runes.iter() {

        // RTA and Siege spd/eff coefficients
        let set_id = rune.set_id.to_string();
        // let coeff_set = 1; // Old coeff
        let mut rta_set_eff_coeff = 1;
        let mut siege_set_eff_coeff = 1;
        let mut rta_set_spd_coeff = 1;
        let mut siege_set_spd_coeff = 1;

        // Set category
        let set_category = if siege_set_spd_coeffs.get(set_id.as_str()).is_some() {
            set_id.clone()
        } else {
            "Other".to_string()
        };
        // RTA
        if let Some(set_coeff) = rta_set_eff_coeffs.get(set_id.as_str()) {
            rta_set_eff_coeff = set_coeff.as_u64().expect("rta_set_eff_coeff should be an integer") as u32;
        }
        if let Some(set_coeff) = rta_set_spd_coeffs.get(set_id.as_str()) {
            rta_set_spd_coeff = set_coeff.as_u64().expect("rta_set_spd_coeff should be an integer") as u32;
        }
        // Siege
        if let Some(set_coeff) = siege_set_eff_coeffs.get(set_id.as_str()) {
            siege_set_eff_coeff = set_coeff.as_u64().expect("siege_set_eff_coeff should be an integer") as u32;
        }
        if let Some(set_coeff) = siege_set_spd_coeffs.get(set_id.as_str()) {
            siege_set_spd_coeff = set_coeff.as_u64().expect("siege_set_spd_coeff should be an integer") as u32;
        }

        // Global_efficiency_coeff
        let efficiency = rune.efficiency.unwrap_or_default();

        let mut global_coeff_eff = 0.0;
        let mut global_eff_key = "0".to_string();

        for (key, value) in global_efficiency_coeffs
            .as_object()
            .expect("Efficiency should be an object")
            .iter()
            .rev()
        {
            if efficiency >= key.parse::<f32>().expect("Invalid efficiency key") {
                global_coeff_eff = value.as_f64().expect("Invalid efficiency value") as f32;
                global_eff_key = key.clone();
                break;
            }
        }

        // Global_speed_coeff
        let speed = rune.speed_value.unwrap_or_default();

        let mut global_coeff_spd = 0.0;
        let mut global_spd_key = "0".to_string();

        for (key, value) in global_speed_coeffs
            .as_object()
            .expect("Speed should be an object")
            .iter()
            .rev()
        {
            if speed >= key.parse::<u32>().expect("Invalid speed key") {
                global_coeff_spd = value.as_f64().expect("Invalid efficiency value") as f32;
                global_spd_key = key.clone();
                break;
            }
        }

        // Mapping set category
        let eff_entry = map_score_eff
            .entry(set_category.clone())
            .or_insert_with(HashMap::new);
        *eff_entry.entry(global_eff_key).or_insert(0) += 1;
        let spd_entry = map_score_spd
            .entry(set_category.clone())
            .or_insert_with(HashMap::new);
        *spd_entry.entry(global_spd_key).or_insert(0) += 1;

        // RTA
        rta_score_eff += rta_set_eff_coeff as f32 * global_coeff_eff as f32;
        rta_score_spd += rta_set_spd_coeff as f32 * global_coeff_spd as f32;

        // Siege
        siege_score_eff += siege_set_eff_coeff as f32 * global_coeff_eff as f32;
        siege_score_spd += siege_set_spd_coeff as f32 * global_coeff_spd as f32;
    }
    (
        rta_score_eff,
        rta_score_spd,
        siege_score_eff,
        siege_score_spd,
        map_score_eff,
        map_score_spd,
        wizard_info_data,
        account_info_data,
    )
}
