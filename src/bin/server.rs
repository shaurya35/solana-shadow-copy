use solana_trade_diagnostics::config::ServerConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::from_env()?;

    println!(
        "Solana Trade Diagnostics API configured on {}",
        config.bind_addr()
    );

    Ok(())
}
