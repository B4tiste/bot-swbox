// use std::collections::HashMap;
// use reqwest;
use serde_json::Value;

// Update the path to your local module, assuming you have a `utils` folder with `artifact.rs` in `src`
mod utils;
use crate::utils::artifact::{
    Artifact,
    ArtifactMainStat,
    ArtifactTypeId,
    Effect,
    ArtifactArchetypeId,
    ArtifactAttributeId,
};
use crate::utils::utils::{
    get_artifact_type_id_by_id, get_artifact_main_stat_id_by_id, get_artifact_effect_id_by_id, get_artifact_attribute_id_by_id, get_artifact_archetype_id_by_id
};

fn main() {
    // Analyse d'un fichier JSON (test.json)

    // 1. Ouverture et lecture d'une info
    /*
    {
    "command": "HubUserLogin",
    "ret_code": 0,
    "wizard_info": {
        "wizard_id": 1173973, <-
        "wizard_name": "B4tiste", <-
    */
    let file_content = std::fs::read_to_string("test.json").expect("Failed to read file");
    let json_data: Value = serde_json::from_str(&file_content).expect("Failed to parse JSON");
    let wizard_id = json_data["wizard_info"]["wizard_id"].as_u64().expect("Failed to get wizard_id");
    let wizard_name = json_data["wizard_info"]["wizard_name"].as_str().expect("Failed to get wizard_name");

    println!("Wizard ID: {}, Wizard Name: {}", wizard_id, wizard_name);

    // Parser les artifacts du compte
    let mut vec_artifacts: Vec<Artifact> = Vec::new();
    if let Some(unit_list) = json_data.get("unit_list") {
        for unit in unit_list.as_array().expect("Failed to get unit list") {
            if let Some(artifacts) = unit.get("artifacts") {
                for artifact in artifacts.as_array().expect("Failed to get artifacts in unit") {
                    if let Some(parsed_artifact) = extract_artifact(artifact) {
                        vec_artifacts.push(parsed_artifact);
                    }
                }
            }
        }
    }
    if let Some(artifacts) = json_data.get("artifacts") {
        for artifact in artifacts.as_array().expect("Failed to get artifacts") {
            if let Some(parsed_artifact) = extract_artifact(artifact) {
                vec_artifacts.push(parsed_artifact);
            }
        }
    }

    // Save artifacts to a file
    let artifacts_file_path = "artifacts.json";
    let artifacts_json = serde_json::to_string(&vec_artifacts).expect("Failed to serialize artifacts");
    std::fs::write(artifacts_file_path, artifacts_json).expect("Failed to write artifacts to file");
    println!("Artifacts saved to {}", artifacts_file_path);
}

fn extract_artifact(artifact: &Value) -> Option<Artifact> {
    let id = artifact.get("rid")?.as_u64()? as u32;
    let artifact_type_id = get_artifact_type_id_by_id(artifact.get("type")?.as_u64()? as u32);

    let (artifact_attribute_id, artifact_archetype_id) = if artifact_type_id == ArtifactTypeId::Attribute {
        (
            get_artifact_attribute_id_by_id(artifact.get("attribute")?.as_u64()? as u32)?,
            ArtifactArchetypeId::Attack, // Default
        )
    } else if artifact_type_id == ArtifactTypeId::Archetype {
        (
            ArtifactAttributeId::Water, // Default
            get_artifact_archetype_id_by_id(artifact.get("unit_style")?.as_u64()? as u32)?,
        )
    } else {
        return None;
    };

    let main_stat = if let Some(pri_effect) = artifact.get("pri_effect") {
        let pri_effect_array = pri_effect.as_array()?;
        let stat_id = get_artifact_main_stat_id_by_id(pri_effect_array[0].as_u64()? as u32);
        let value = pri_effect_array[1].as_u64()? as u32;
        ArtifactMainStat::new(stat_id, value)
    } else {
        return None;
    };

    let mut secondary_effects = Vec::new();
    if let Some(sec_eff) = artifact.get("sec_effects") {
        let sec_eff_array = sec_eff.as_array()?;
        for sec_eff in sec_eff_array {
            let sec_eff_array_p = sec_eff.as_array()?;
            let effect_id_opt = get_artifact_effect_id_by_id(sec_eff_array_p[0].as_u64()? as u32);
            let value = sec_eff_array_p[1].as_f64()? as f32;
            if let Some(effect_id) = effect_id_opt {
                secondary_effects.push(Effect::new(effect_id, value));
            }
        }
    }

    Some(Artifact {
        id,
        artifact_type: artifact_type_id,
        artifact_attribute: artifact_attribute_id,
        artifact_archetype: artifact_archetype_id,
        main_stat,
        secondary_effects,
    })
}
