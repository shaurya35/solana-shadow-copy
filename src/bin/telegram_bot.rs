use solana_trade_diagnostics::config::BotConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = BotConfig::from_env()?;

    println!(
        "Solana Trade Diagnostics Telegram Bot configured for {}",
        config.diagnostics_api_url()
    );

    Ok(())
}
