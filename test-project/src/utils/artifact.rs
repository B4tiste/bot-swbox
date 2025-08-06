use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Artifact {
    pub id: u32,
    pub artifact_type: ArtifactTypeId,
    pub artifact_attribute: ArtifactAttributeId,
    pub artifact_archetype: ArtifactArchetypeId,
    pub main_stat: ArtifactMainStat,
    pub secondary_effects: Vec<Effect>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArtifactMainStat {
    pub id: ArtifactMainStatId,
    pub value: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ArtifactMainStatId {
    Hp,
    Atk,
    Def,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Effect {
    pub id: ArtifactEffectId,
    pub value: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ArtifactTypeId {
    Attribute,
    Archetype,
}

impl fmt::Display for ArtifactTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ArtifactAttributeId {
    Water,
    Fire,
    Wind,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ArtifactArchetypeId {
    Attack,
    Defense,
    Hp,
    Support,
}

#[derive(Serialize, Deserialize, Default, Debug, Copy, Clone, PartialEq)]
pub enum ArtifactEffectId {
    #[default]
    SpdIncreaseEffect,
    AddlDmgOfHp,
    AddlDmgOfAtk,
    AddlDmgOfDef,
    AddlDmgOfSpd,
    DDOFire,
    DDOWater,
    DDOWind,
    DDOLight,
    DDODark,
    DRFFire,
    DRFWater,
    DRFWind,
    DRFLight,
    DRFDark,
}

impl fmt::Display for ArtifactEffectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ArtifactMainStat {
    pub fn new(id: ArtifactMainStatId, value: u32) -> Self {
        Self { id, value }
    }
}

impl Effect {
    pub fn new(id: ArtifactEffectId, value: f32) -> Self {
        Self { id, value }
    }
}