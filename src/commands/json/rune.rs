use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Rune {
    id: u32,
    slot_location: u32,
    class: StarsAmmount,
    antic: bool,
    pub set_id: RuneSetId,
    upgrade_limit: u32,
    upgrade_current: u32,
    primary_property: Property,
    innate_property: Property,
    pub secondary_properties: Vec<Property>,
    pub efficiency: Option<f32>,
    pub speed_value: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Property {
    pub id: RuneStatId,
    pub value: f32,
    has_been_replaced: Option<bool>,
    pub boost_value: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StarsAmmount {
    Five,
    Six,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RuneSetId {
    Energy,
    Guard,
    Swift,
    Blade,
    Rage,
    Focus,
    Endure,
    Fatal,
    Despair,
    Vampire,
    Violent,
    Nemesis,
    Will,
    Shield,
    Revenge,
    Destroy,
    Fight,
    Determination,
    Enhance,
    Accuracy,
    Tolerance,
    Sceal,
    Intangible,
}

impl fmt::Display for RuneSetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Copy, Clone, PartialEq)]
pub enum RuneStatId {
    #[default]
    None,
    Hp,
    HpPct,
    Atk,
    AtkPtc,
    Def,
    DefPtc,
    Spd,
    CritRatePct,
    CritDmgPct,
    ResistPct,
    AccuracyPct,
}

impl Rune {
    pub fn new(
        id: u32,
        slot_location: u32,
        class: StarsAmmount,
        antic: bool,
        set_id: RuneSetId,
        upgrade_limit: u32,
        upgrade_current: u32,
        primary_property: Property,
        innate_property: Property,
        secondary_properties: Vec<Property>,
    ) -> Self {
        let mut rune = Rune {
            id,
            slot_location,
            class,
            antic,
            set_id,
            upgrade_limit,
            upgrade_current,
            primary_property,
            innate_property,
            secondary_properties,
            efficiency: None,
            speed_value: None,
        };
        rune.efficiency = Some(rune.calculate_efficiency());
        rune.speed_value = rune.get_speed_value();
        return rune;
    }

    fn calculate_efficiency(&self) -> f32 {
        let eff_main;
        let mut eff_innate = 0.0;
        let mut eff_subs = 0.0;

        match self.class {
            StarsAmmount::Five => {
                // 5* rune efficiency
                // Main stat efficiency
                eff_main = get_main_stat_max_value_by_id_5(self.primary_property.id)
                    / get_main_stat_max_value_by_id_6(self.primary_property.id);

                // Innate stat efficiency
                if self.innate_property.id != RuneStatId::None {
                    eff_innate = calculate_eff_stat_6(&self.innate_property);
                }

                // Sub stats efficiency
                for stat in self.secondary_properties.iter() {
                    eff_subs += calculate_eff_stat_6(stat);
                }

                // Return the sum of all efficiencies
                return ((eff_main + eff_innate + eff_subs) / 2.8) * 100.0;
            }

            StarsAmmount::Six => {
                // 6* rune efficiency
                // Main stat efficiency
                eff_main = get_main_stat_max_value_by_id_6(self.primary_property.id)
                    / get_main_stat_max_value_by_id_6(self.primary_property.id);

                // Innate stat efficiency
                if self.innate_property.id != RuneStatId::None {
                    eff_innate = calculate_eff_stat_6(&self.innate_property);
                }

                // Sub stats efficiency
                for stat in self.secondary_properties.iter() {
                    eff_subs += calculate_eff_stat_6(stat);
                }

                // Return the sum of all efficiencies
                return ((eff_main + eff_innate + eff_subs) / 2.8) * 100.0;
            }
        }
    }
    fn get_speed_value(&self) -> Option<u32> {
        let mut speed_value = 0;
        for stat in self.secondary_properties.iter() {
            if stat.id == RuneStatId::Spd {
                speed_value += stat.value as u32;
                if let Some(boost_value) = stat.boost_value {
                    speed_value += boost_value as u32;
                }
            }
        }
        Some(speed_value)
    }
}

impl Property {
    pub fn new(
        id: RuneStatId,
        value: f32,
        has_been_replaced: Option<bool>,
        boost_value: Option<f32>,
    ) -> Self {
        Property {
            id,
            value,
            has_been_replaced,
            boost_value,
        }
    }
}

pub fn get_stars_ammount_by_id(id: u32) -> StarsAmmount {
    match id {
        5 | 15 => StarsAmmount::Five,
        6 | 16 => StarsAmmount::Six,
        _ => panic!("Invalid stars ammount"),
    }
}

pub fn get_rune_set_id_by_id(id: u32) -> RuneSetId {
    match id {
        1 => RuneSetId::Energy,
        2 => RuneSetId::Guard,
        3 => RuneSetId::Swift,
        4 => RuneSetId::Blade,
        5 => RuneSetId::Rage,
        6 => RuneSetId::Focus,
        7 => RuneSetId::Endure,
        8 => RuneSetId::Fatal,
        10 => RuneSetId::Despair,
        11 => RuneSetId::Vampire,
        13 => RuneSetId::Violent,
        14 => RuneSetId::Nemesis,
        15 => RuneSetId::Will,
        16 => RuneSetId::Shield,
        17 => RuneSetId::Revenge,
        18 => RuneSetId::Destroy,
        19 => RuneSetId::Fight,
        20 => RuneSetId::Determination,
        21 => RuneSetId::Enhance,
        22 => RuneSetId::Accuracy,
        23 => RuneSetId::Tolerance,
        24 => RuneSetId::Sceal,
        25 => RuneSetId::Intangible,
        _ => panic!("Invalid runes set id"),
    }
}

pub fn get_rune_stat_id_by_id(id: u32) -> RuneStatId {
    match id {
        0 => RuneStatId::None,
        1 => RuneStatId::Hp,
        2 => RuneStatId::HpPct,
        3 => RuneStatId::Atk,
        4 => RuneStatId::AtkPtc,
        5 => RuneStatId::Def,
        6 => RuneStatId::DefPtc,
        8 => RuneStatId::Spd,
        9 => RuneStatId::CritRatePct,
        10 => RuneStatId::CritDmgPct,
        11 => RuneStatId::ResistPct,
        12 => RuneStatId::AccuracyPct,
        _ => panic!("Invalid rune stat id"),
    }
}

pub fn get_max_value_stat_6(id: RuneStatId) -> f32 {
    match id {
        RuneStatId::Hp => 750.0,
        RuneStatId::Atk | RuneStatId::Def => 40.0,
        RuneStatId::HpPct
        | RuneStatId::AtkPtc
        | RuneStatId::DefPtc
        | RuneStatId::ResistPct
        | RuneStatId::AccuracyPct => 8.0,
        RuneStatId::Spd | RuneStatId::CritRatePct => 6.0,
        RuneStatId::CritDmgPct => 7.0,
        _ => 0.0,
    }
}

pub fn calculate_eff_stat_6(stat: &Property) -> f32 {
    return (stat.value + stat.boost_value.unwrap_or(0.0)) / (get_max_value_stat_6(stat.id) * 5.0);
}

pub fn get_main_stat_max_value_by_id_5(id: RuneStatId) -> f32 {
    match id {
        RuneStatId::Hp => 2088.0,
        RuneStatId::HpPct => 51.0,
        RuneStatId::Atk => 135.0,
        RuneStatId::AtkPtc => 51.0,
        RuneStatId::Def => 135.0,
        RuneStatId::DefPtc => 51.0,
        RuneStatId::Spd => 39.0,
        RuneStatId::CritRatePct => 47.0,
        RuneStatId::CritDmgPct => 65.0,
        RuneStatId::ResistPct => 51.0,
        RuneStatId::AccuracyPct => 51.0,
        _ => 0.0,
    }
}

pub fn get_main_stat_max_value_by_id_6(id: RuneStatId) -> f32 {
    match id {
        RuneStatId::Hp => 2448.0,
        RuneStatId::HpPct => 63.0,
        RuneStatId::Atk => 160.0,
        RuneStatId::AtkPtc => 63.0,
        RuneStatId::Def => 160.0,
        RuneStatId::DefPtc => 63.0,
        RuneStatId::Spd => 42.0,
        RuneStatId::CritRatePct => 58.0,
        RuneStatId::CritDmgPct => 80.0,
        RuneStatId::ResistPct => 64.0,
        RuneStatId::AccuracyPct => 64.0,
        _ => 0.0,
    }
}
