use std::sync::Arc;

use tracing_subscriber::EnvFilter;

use litepay_server::build_router;
use litepay_server::config::{Config, LnBackendConfig};
use litepay_server::db;
use litepay_server::ln::{lnd::LndBackend, mock::MockBackend, LnBackend};
use litepay_server::state::AppState;
use litepay_server::ws::NotificationHub;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("litepay=info".parse()?))
        .init();

    let config = Config::load(None)?;
    tracing::info!("LitePay Server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Listening on {}:{}", config.host, config.port);

    let pool = db::init_pool(&config.database_url).await?;
    tracing::info!("Database initialized");

    let ln_backend: Arc<dyn LnBackend> = match &config.ln_backend {
        LnBackendConfig::Mock => {
            tracing::info!("Using Mock Lightning backend (development mode)");
            Arc::new(MockBackend::new())
        }
        LnBackendConfig::Lnd {
            url,
            macaroon,
            tls_cert,
        } => {
            tracing::info!("Using LND backend at {}", url);
            Arc::new(LndBackend::new(url, macaroon, tls_cert.as_deref())?)
        }
    };

    let state = AppState {
        db: pool,
        ln: ln_backend,
        hub: NotificationHub::new(),
        config: Arc::new(config.clone()),
    };

    let app = build_router(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server started at http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
