use std::collections::HashMap;
use lazy_static::lazy_static;
use serde_json::json;

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
                "en_names": ["kelianbao", "kelian bao"],
                "swarena_id": 28964534,
                "swrt_id": 54175
            },
            {
                "og_name": "未生",
                "en_names": ["tars"],
                "swarena_id": 19979062,
                "swrt_id": 48169
            },
            {
                "og_name": "鮭  　　　 ",
                "en_names": ["lest"],
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
                "en_names": ["cabrera"],
                "swarena_id": 1176597,
                "swrt_id": 11026
            },
            // Custom
            {
                "og_name": "Salvandar~",
                "en_names": ["salvodar"],
                "swarena_id": 1148532,
                "swrt_id": 10871
            },
            {
                "og_name": "Chiikoup~",
                "en_names": ["autowin"],
                "swarena_id": 17461686,
                "swrt_id": 11401858
            },
            {
                "og_name": "ᴅᴀᴍ~",
                "en_names": ["Le débilos"],
                "swarena_id": 2419842,
                "swrt_id": 17817
            },
            {
                "og_name": "ҜΞИZФ™",
                "en_names": ["yaz le fou"],
                "swarena_id": 155825,
                "swrt_id": 1353033
            },
{
"og_names": "?",
"en_names": ["compte tyteii"],
"swarena_id": 935484,
"swrt_id": 31998085,
},
        ]
    });
}
