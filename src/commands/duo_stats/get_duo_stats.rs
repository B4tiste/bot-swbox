use poise::{
    serenity_prelude::{self as serenity, CreateEmbed, Error},
    Modal,
    CreateReply,
};

use crate::commands::shared::utils::{get_season, get_monster_general_info, get_monster_slug};
use crate::commands::shared::embed_error_handling::{create_embed_error, schedule_message_deletion};
// use crate::commands::mob_stats::utils::get_monster_rta_info;