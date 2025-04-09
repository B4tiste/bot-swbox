use std::collections::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PLAYER_ALIAS_MAP: HashMap<i64, String> = {
        let mut m = HashMap::new();

        m.insert(54175, "Kelianbao".to_string());
        m.insert(48169, "tars".to_string());
        m.insert(30389, "Lest".to_string());
        m.insert(8123, "sk!t".to_string());
        m.insert(11026, "cabrera".to_string());

        m
    };
}
