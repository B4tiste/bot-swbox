use lazy_static::lazy_static;
use serde_json::json;
use std::collections::HashMap;

lazy_static! {
    /// Map utilisée dans `/get_leaderboard` pour afficher (alias)
    pub static ref PLAYER_ALIAS_MAP: HashMap<i64, String> = {
        let mut m = HashMap::new();
        for player in ALIAS_DATA["players"].as_array().unwrap() {
            let swrt_id = player["swrt_id"].as_i64().unwrap();
            let en_names = player["en_names"].as_array().unwrap();
            if let Some(first_alias) = en_names.get(0) {
                m.insert(swrt_id, first_alias.as_str().unwrap().to_string());
            }
        }
        m
    };

    /// Map utilisée pour faire la recherche par alias dans `/get_player_stats`
    pub static ref ALIAS_LOOKUP_MAP: HashMap<String, i64> = {
        let mut m = HashMap::new();
        for player in ALIAS_DATA["players"].as_array().unwrap() {
            let swrt_id = player["swrt_id"].as_i64().unwrap();
            let en_names = player["en_names"].as_array().unwrap();
            for alias in en_names {
                m.insert(alias.as_str().unwrap().to_lowercase(), swrt_id);
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
                "swrt_id": 54175
            },
            {
                "og_name": "未生",
                "en_names": ["Tars"],
                "swarena_id": 19979062,
                "swrt_id": 48169
            },
            {
                "og_name": "鮭  　　　 ",
                "en_names": ["Lest"],
                "swarena_id": 6489096,
                "swrt_id": 30389
            },
            {
                "og_name": "스킷:)",
                "en_names": ["sk!t", "skit", "skit!"],
                "swarena_id": 647538,
                "swrt_id": 8123
            },
            {
                "og_name": "XカブレラX",
                "en_names": ["Cabrera"],
                "swarena_id": 1176597,
                "swrt_id": 11026
            },
            // Custom
            {
                "og_name": "Salvandar~",
                "en_names": ["Salvodar"],
                "swarena_id": 1148532,
                "swrt_id": 10871
            },
            {
                "og_name": "ᴅᴀᴍ~",
                "en_names": ["Le débilos"],
                "swarena_id": 2419842,
                "swrt_id": 17817
            },
            {
                "og_names": "?",
                "en_names": ["Compte Tyteii"],
                "swarena_id": 935484,
                "swrt_id": 31998085,
            },
            {
                "og_names": "Falthazard",
                "en_names": ["Falzar"],
                "swarena_id": 11934958,
                "swrt_id": 38089,
            },
            {
                "og_names": "?",
                "en_names": ["Compte 1piss"],
                "swarena_id": 4670983,
                "swrt_id": 26158,
            },
            {
                "og_names": "?",
                "en_names": ["Compte Villipyty"],
                "swarena_id": 2669729,
                "swrt_id": 19043,
            },
            {
                "og_names": "?",
                "en_names": ["Compte Sapyn"],
                "swarena_id": 1931431,
                "swrt_id": 15145,
            },
            {
                "og_names": "?",
                "en_names": ["Compte Tututuh", "Tuh", "222"],
                "swarena_id": 10315887,
                "swrt_id": 35724,
            },
            {
                "og_names": "?",
                "en_names": ["Compte Ruiwen"],
                "swarena_id": 9398325,
                "swrt_id": 34690,
            },
            {
                "og_names": "HippoCos",
                "en_names": ["Compte Hippo"],
                "swarena_id": 3797770,
                "swrt_id": 23576,
            },
            {
                "og_names": "B4tiste",
                "en_names": ["BOT Developer"],
                "swarena_id": 1173973,
                "swrt_id": 11011,
            },
            {
                "og_names": "?",
                "en_names": ["track mat 1"],
                "swarena_id": 13428699,
                "swrt_id": 2286873,
            },
            {
                "og_names": "?",
                "en_names": ["track mat 2"],
                "swarena_id": 5743823,
                "swrt_id": 28752,
            },
            {
                "og_names": "Pinkroid",
                "en_names": ["#FreePinkroid"],
                "swarena_id": 2196070,
                "swrt_id": 16494,
            },
            {
                "og_names": "?",
                "en_names": ["Compte Zinthos"],
                "swarena_id": 7729068,
                "swrt_id": 32489,
            },
            {
                "og_names": "?",
                "en_names": ["Compte montage"],
                "swarena_id": 2759178,
                "swrt_id": 19482,
            },
            {
                "og_names": "?",
                "en_names": ["Compte Hextro"],
                "swarena_id": 187546,
                "swrt_id": 4043,
            },
            {
                "og_names": "?",
                "en_names": ["Compte Tagy"],
                "swarena_id": 2635681,
                "swrt_id": 383855,
            }
        ]
    });
}
