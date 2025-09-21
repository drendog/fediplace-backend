use std::error::Error;
use std::net::SocketAddr;

use tokio::net::TcpListener;
use tokio::signal;
use tracing::{error, info};

use server::bootstrap::router::create_router;
use server::bootstrap::state::AppState;
use server::config_loader;
use server::observability;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let config = config_loader::load_config()?;

    observability::tracing::setup_logging(&config)?;

    info!("Starting FediPlace Backend Server");
    info!("Configuration loaded successfully");
    info!("Database URL: {}", config.db.redacted_url());

    let state = AppState::new(config.clone()).await?;

    let app = create_router(state.clone())
        .await?
        .into_make_service_with_connect_info::<SocketAddr>();

    let listener = TcpListener::bind(&config.server_address()).await?;
    info!("Server listening on http://{}", config.server_address());

    observability::startup_info::print_api_info(&config);

    let result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;

    if let Err(e) = result {
        error!("Server error: {}", e);
        return Err(e.into());
    }

    info!("Server shutdown completed");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = signal::ctrl_c().await {
            error!("Failed to install Ctrl+C handler: {}", e);
        }
    };

    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                error!("Failed to install signal handler: {}", e);
            }
        }
    };

    tokio::select! {
        () = ctrl_c => {
            info!("Received Ctrl+C signal, starting graceful shutdown...");
        },
        () = terminate => {
            info!("Received terminate signal, starting graceful shutdown...");
        },
    }
}
