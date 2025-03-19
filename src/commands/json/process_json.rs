use crate::commands::json::rune::{
    get_rune_set_id_by_id, get_rune_stat_id_by_id, get_stars_ammount_by_id, Rune,
};
use crate::commands::json::rune::{Property, RuneStatId};
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
            let sec_eff_array = sec_eff.as_array()?;
            let stat_id = get_rune_stat_id_by_id(sec_eff_array[0].as_u64()? as u32);
            let value = sec_eff_array[1].as_f64()? as f32;
            let has_been_replaced = sec_eff_array[2].as_u64()? == 1;
            let boost_value = sec_eff_array[3].as_u64()? as f32;
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

pub fn process_json(
    json: Value,
) -> (
    f32,
    HashMap<String, HashMap<String, u32>>,
    HashMap<String, HashMap<String, u32>>,
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

    // Replace file reading with hardcoded JSON content
    let efficiency_coeffs = serde_json::json!({
        "100": 1,
        "110": 2,
        "120": 3
    });
    let speed_coeffs = serde_json::json!({
        "23": 1,
        "26": 2,
        "29": 3,
        "32": 4
    });
    let set_coeffs = serde_json::json!({
        "Despair": 3,
        "Swift": 3,
        "Violent": 3,
        "Will": 2,
        "Intangible": 4
    });

    let mut score: f32 = 0.0;
    let mut map_score_eff: HashMap<String, HashMap<String, u32>> = HashMap::new();
    let mut map_score_spd: HashMap<String, HashMap<String, u32>> = HashMap::new();

    for rune in vec_runes.iter() {
        let set_id = rune.set_id.to_string();
        let mut coeff_set = 1;
        let set_category = if set_coeffs.get(set_id.as_str()).is_some() {
            set_id.clone()
        } else {
            "Other".to_string()
        };
        if let Some(set_coeff) = set_coeffs.get(set_id.as_str()) {
            coeff_set = set_coeff.as_u64().expect("coeff_set should be an integer") as u32;
        }
        let efficiency = rune.efficiency.unwrap_or_default();
        let mut coeff_eff = 0;
        let mut eff_key = "0".to_string();
        for (key, value) in efficiency_coeffs
            .as_object()
            .expect("Efficiency should be an object")
            .iter()
            .rev()
        {
            if efficiency >= key.parse::<f32>().expect("Invalid efficiency key") {
                coeff_eff = value.as_u64().expect("coeff_eff should be an integer") as u32;
                eff_key = key.clone();
                break;
            }
        }
        let mut speed = 0;
        if let Some(sub_stats) = Some(&rune.secondary_properties) {
            for stat in sub_stats {
                if stat.id == RuneStatId::Spd {
                    speed += stat.value as u32 + stat.boost_value.unwrap_or(0.0) as u32;
                }
            }
        }
        let mut coeff_spd = 0;
        let mut spd_key = "0".to_string();
        for (key, value) in speed_coeffs
            .as_object()
            .expect("Speed should be an object")
            .iter()
            .rev()
        {
            if speed >= key.parse::<u32>().expect("Invalid speed key") {
                coeff_spd = value.as_u64().expect("coeff_spd should be an integer") as u32;
                spd_key = key.clone();
                break;
            }
        }
        let eff_entry = map_score_eff
            .entry(set_category.clone())
            .or_insert_with(HashMap::new);
        *eff_entry.entry(eff_key).or_insert(0) += 1;
        let spd_entry = map_score_spd
            .entry(set_category.clone())
            .or_insert_with(HashMap::new);
        *spd_entry.entry(spd_key).or_insert(0) += 1;
        score += coeff_set as f32 * (coeff_spd as f32 + coeff_eff as f32);
    }
    (score, map_score_eff, map_score_spd, wizard_info_data)
}
