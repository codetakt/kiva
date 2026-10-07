//! Kiva server binary.

use std::net::{Ipv4Addr, SocketAddr};

/// Starts the Kiva HTTP server.
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .init();

    let address = listen_address()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    let router = kiva_server::router_from_env().await?;

    tracing::info!(%address, "starting Kiva server");
    axum::serve(listener, router).await
}

fn listen_address() -> Result<SocketAddr, std::io::Error> {
    match std::env::var("KIVA_LISTEN_ADDR") {
        Ok(value) => value.parse::<SocketAddr>().map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("KIVA_LISTEN_ADDR must be a socket address: {error}"),
            )
        }),
        Err(std::env::VarError::NotPresent) => Ok(SocketAddr::from((Ipv4Addr::LOCALHOST, 8080))),
        Err(error) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("failed to read KIVA_LISTEN_ADDR: {error}"),
        )),
    }
}
