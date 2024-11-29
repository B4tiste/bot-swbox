use poise::serenity_prelude::Error;
use poise::Modal;

use super::models::PlayerSearchInput;

pub async fn handle_modal<M, F>(
    ctx: poise::ApplicationContext<'_, (), Error>,
    transform: F,
) -> Result<Option<PlayerSearchInput>, Error>
where
    M: Modal + Send,
    F: Fn(M) -> PlayerSearchInput + Send,
{
    let result = M::execute(ctx).await?;
    Ok(result.map(transform))
}