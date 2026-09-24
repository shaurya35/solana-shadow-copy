use solana_trade_diagnostics::{config::BotConfig, telegram::TelegramBot};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = BotConfig::from_env()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .try_init()?;
    let bot = TelegramBot::new(&config)?;

    info!(diagnostics_api = %config.diagnostics_api_url(), "Telegram bot started");
    tokio::select! {
        () = bot.run() => {}
        result = tokio::signal::ctrl_c() => {
            result?;
            info!("Telegram bot shutting down");
        }
    }

    Ok(())
}
