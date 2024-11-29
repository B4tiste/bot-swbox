use crate::commands::player_names::modal::{
    PlayerNamesInfosModalById, PlayerNamesInfosModalByName,
};
use crate::commands::player_names::models::{PlayerNamesModalData, PlayerSearchInput};
use crate::commands::player_names::utils::handle_modal;
use poise::serenity_prelude::Error;

/// 📂 Affiche les différents noms d'utilisateurs que ce joueur a pu avoir.
///
/// Usage: `/track_player_names`
#[poise::command(slash_command)]
pub async fn track_player_names(
    ctx: poise::ApplicationContext<'_, (), Error>,
    #[description = "Sélectionnez le moyen de recherche"] mode: PlayerNamesModalData,
) -> Result<(), Error> {
    let modal_result = match mode {
        PlayerNamesModalData::Id => {
            handle_modal::<PlayerNamesInfosModalById, _>(ctx.clone(), |data| PlayerSearchInput {
                id: Some(data.id),
                name: None,
            })
            .await
        }
        PlayerNamesModalData::Name => {
            handle_modal::<PlayerNamesInfosModalByName, _>(ctx.clone(), |data| PlayerSearchInput {
                id: None,
                name: Some(data.name),
            })
            .await
        }
    };

    Ok(())
}
