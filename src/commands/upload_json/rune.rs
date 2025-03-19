use serde::{Deserialize, Serialize};
use std::fmt;

use crate::commands::upload_json::utils::{
    calculate_eff_stat_6, get_main_stat_max_value_by_id_5, get_main_stat_max_value_by_id_6,
};

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
