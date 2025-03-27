use crate::commands::upload_json::rune::{Property, RuneSetId, RuneStatId, StarsAmmount};

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
        24 => RuneSetId::Seal,
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
