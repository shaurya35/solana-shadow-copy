use solana_trade_diagnostics::{api, config::ServerConfig, rpc::RpcClient};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = ServerConfig::from_env()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .try_init()?;
    let rpc = RpcClient::new(&config)?;
    let listener = tokio::net::TcpListener::bind(config.bind_addr()).await?;

    info!(bind_address = %config.bind_addr(), "diagnostics API listening");
    axum::serve(listener, api::router(rpc))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "failed to install shutdown signal handler");
    }
}
