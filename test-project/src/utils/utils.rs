use crate::utils::artifact::{ArtifactMainStatId, ArtifactTypeId, ArtifactEffectId, ArtifactAttributeId, ArtifactArchetypeId};

pub fn get_artifact_type_id_by_id(id: u32) -> ArtifactTypeId {
    match id {
        1 => ArtifactTypeId::Attribute,
        2 => ArtifactTypeId::Archetype,
        _ => panic!("Invalid artifact type id"),
    }
}

pub fn get_artifact_attribute_id_by_id(id: u32) -> Option<ArtifactAttributeId> {
    match id {
        1 => Some(ArtifactAttributeId::Water),
        2 => Some(ArtifactAttributeId::Fire),
        3 => Some(ArtifactAttributeId::Wind),
        4 => Some(ArtifactAttributeId::Light),
        5 => Some(ArtifactAttributeId::Dark),
        _ => None,
    }
}

pub fn get_artifact_archetype_id_by_id(id: u32) -> Option<ArtifactArchetypeId> {
    match id {
        1 => Some(ArtifactArchetypeId::Attack),
        2 => Some(ArtifactArchetypeId::Defense),
        3 => Some(ArtifactArchetypeId::Hp),
        4 => Some(ArtifactArchetypeId::Support),
        _ => None,
    }
}

pub fn get_artifact_main_stat_id_by_id(id: u32) -> ArtifactMainStatId {
    match id {
        100 => ArtifactMainStatId::Hp,
        101 => ArtifactMainStatId::Atk,
        102 => ArtifactMainStatId::Def,
        _ => panic!("Invalid artifact main stat id"),
    }
}

pub fn get_artifact_effect_id_by_id(id: u32) -> Option<ArtifactEffectId> {
    match id {
        206 => Some(ArtifactEffectId::SpdIncreaseEffect),
        218 => Some(ArtifactEffectId::AddlDmgOfHp),
        219 => Some(ArtifactEffectId::AddlDmgOfAtk),
        220 => Some(ArtifactEffectId::AddlDmgOfDef),
        221 => Some(ArtifactEffectId::AddlDmgOfSpd),
        300 => Some(ArtifactEffectId::DDOFire),
        301 => Some(ArtifactEffectId::DDOWater),
        302 => Some(ArtifactEffectId::DDOWind),
        303 => Some(ArtifactEffectId::DDOLight),
        304 => Some(ArtifactEffectId::DDODark),
        305 => Some(ArtifactEffectId::DRFFire),
        306 => Some(ArtifactEffectId::DRFWater),
        307 => Some(ArtifactEffectId::DRFWind),
        308 => Some(ArtifactEffectId::DRFLight),
        309 => Some(ArtifactEffectId::DRFDark),
        _ => None,
    }
}