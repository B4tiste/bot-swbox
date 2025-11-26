use crate::Data;
use poise::serenity_prelude::Error;

use super::handler::help_handler;

/// 📂 Displays the available commands.
///
/// Returns: An embed listing all available commands.
///
/// Examples:
/// - `/help` - Displays the help embed.
///
/// Permissions: None
#[poise::command(slash_command)]
pub async fn help(
    ctx: poise::ApplicationContext<'_, Data, Error>
) -> Result<(), Error> {
    help_handler(ctx).await
}
