pub type Context<'a> = poise::Context<'a, (), Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;