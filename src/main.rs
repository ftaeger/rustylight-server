use anyhow::{Context, Result};
use axum_server::Handle;
use device::{manager, SharedState};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::signal;

mod api;
mod config;
mod device;
mod logging;
mod tls;

#[tokio::main]
async fn main() -> Result<()> {
    let mut cfg = config::load_or_create(config::CONFIG_PATH).context("loading configuration")?;

    logging::init(&cfg.logging.level, &cfg.logging.log_file).context("initialising logging")?;

    tracing::info!("rustylight-server starting");

    config::ensure_psk(&mut cfg, config::CONFIG_PATH).context("ensuring PSK is set")?;

    tls::load_or_generate(&cfg.tls.cert_file, &cfg.tls.key_file)
        .context("ensuring TLS certificate")?;

    let shared = Arc::new(Mutex::new(SharedState::default()));
    manager::spawn_usb_manager(Arc::clone(&shared));

    let state = api::AppState {
        psk: Arc::new(cfg.auth.psk.clone()),
        shared: Arc::clone(&shared),
    };

    let router = api::build_router(state);
    let addr: SocketAddr = format!("0.0.0.0:{}", cfg.server.port).parse()?;
    let tls_config = tls::rustls_config(&cfg.tls.cert_file, &cfg.tls.key_file)
        .await
        .context("loading TLS config")?;

    let handle = Handle::new();
    let handle_clone = handle.clone();

    tokio::spawn(async move {
        shutdown_signal().await;
        tracing::info!("shutdown signal received");
        handle_clone.graceful_shutdown(Some(std::time::Duration::from_secs(5)));
    });

    tracing::info!("listening on https://{addr}");
    axum_server::bind_rustls(addr, tls_config)
        .handle(handle)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .context("server error")?;

    tracing::info!("rustylight-server stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler")
    };
    #[cfg(unix)]
    let sigterm = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {}
        _ = sigterm => {}
    }
}
