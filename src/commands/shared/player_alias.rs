use lazy_static::lazy_static;
use serde_json::json;
use std::collections::HashMap;

lazy_static! {
    /// Map utilisée dans `/get_leaderboard` pour afficher (alias)
    pub static ref PLAYER_ALIAS_MAP: HashMap<i64, String> = {
        let mut m = HashMap::new();
        for player in ALIAS_DATA["players"].as_array().unwrap() {
            let lucksack_id = player["lucksack_id"].as_i64().unwrap();
            let en_names = player["en_names"].as_array().unwrap();
            if let Some(first_alias) = en_names.first() {
                m.insert(lucksack_id, first_alias.as_str().unwrap().to_string());
            }
        }
        m
    };

    /// Map utilisée pour faire la recherche par alias dans `/get_player_stats`
    pub static ref ALIAS_LOOKUP_MAP: HashMap<String, i64> = {
        let mut m = HashMap::new();
        for player in ALIAS_DATA["players"].as_array().unwrap() {
            let lucksack_id = player["lucksack_id"].as_i64().unwrap();
            let en_names = player["en_names"].as_array().unwrap();
            for alias in en_names {
                m.insert(alias.as_str().unwrap().to_lowercase(), lucksack_id);
            }
        }
        m
    };

    static ref ALIAS_DATA: serde_json::Value = json!({
        "players": [
            // Gros joueurs
            {
                "og_name": "沙比版本策划",
                "en_names": ["Kelianbao", "Kelian bao"],
                "swarena_id": 28964534,
                "lucksack_id": 283644
            },
            {
                "og_name": "未生",
                "en_names": ["Tars"],
                "swarena_id": 19979062,
                "lucksack_id": 283646
            },
            {
                "og_name": "鮭  　　　 ",
                "en_names": ["Lest"],
                "swarena_id": 6489096,
                "lucksack_id": 283647
            },
            {
                "og_name": "스킷:)",
                "en_names": ["sk!t", "skit", "skit!"],
                "swarena_id": 647538,
                "lucksack_id": 283652
            },
            {
                "og_name": "XカブレラX",
                "en_names": ["Cabrera"],
                "swarena_id": 1176597,
                "lucksack_id": 251008
            },
            // Custom
            {
                "og_names": "?",
                "en_names": ["Compte Tyteii"],
                "swarena_id": 935484,
                "lucksack_id": 205832,
            },
            {
                "og_names": "Falthazard",
                "en_names": ["Falzar"],
                "swarena_id": 11934958,
                "lucksack_id": 234779,
            },
            {
                "og_names": "?",
                "en_names": ["Compte 1piss"],
                "swarena_id": 4670983,
                "lucksack_id": 209377,
            },
            {
                "og_names": "?",
                "en_names": ["Compte Villipyty"],
                "swarena_id": 2669729,
                "lucksack_id": 244796,
            },
            {
                "og_names": "?",
                "en_names": ["Compte Sapyn"],
                "swarena_id": 1931431,
                "lucksack_id": 236914,
            },
            {
                "og_names": "?",
                "en_names": ["Compte Tututuh", "Tuh", "222"],
                "swarena_id": 10315887,
                "lucksack_id": 283683,
            },
            {
                "og_names": "?",
                "en_names": ["Compte Ruiwen"],
                "swarena_id": 9398325,
                "lucksack_id": 247925,
            },
            {
                "og_names": "HippoCos",
                "en_names": ["Compte Hippo"],
                "swarena_id": 3797770,
                "lucksack_id": 209335,
            },
            {
                "og_names": "B4tiste",
                "en_names": ["BOT Developer"],
                "swarena_id": 1173973,
                "lucksack_id": 173531,
            },
            {
                "og_names": "Pinkroid",
                "en_names": ["#FreePinkroid"],
                "swarena_id": 2196070,
                "lucksack_id": 240389,
            },
            {
                "og_names": "?",
                "en_names": ["Compte Hextro"],
                "swarena_id": 187546,
                "lucksack_id": 234135,
            },
            {
                "og_names": "?",
                "en_names": ["Craig Boosted"],
                "swarena_id": 21744282,
                "lucksack_id": 260486,
            },
            {
                "og_names": "?",
                "en_names": ["Compte Raigeki"],
                "lucksack_id": 213111,
            }
        ]
    });
}
