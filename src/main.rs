#![allow(dead_code)]

mod config;
mod db;
mod error;
mod ln;
mod lnurl;
mod models;
mod routes;
mod state;
mod ws;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use config::{Config, LnBackendConfig};
use ln::{mock::MockBackend, lnd::LndBackend, LnBackend};
use state::AppState;
use ws::NotificationHub;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("litepay=info".parse()?))
        .init();

    // Load config
    let config = Config::load(None)?;
    tracing::info!("LitePay Server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Listening on {}:{}", config.host, config.port);

    // Initialize database
    let pool = db::init_pool(&config.database_url).await?;
    tracing::info!("Database initialized");

    // Initialize Lightning backend
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

    // App state
    let state = AppState {
        db: pool,
        ln: ln_backend,
        hub: NotificationHub::new(),
        config: Arc::new(config.clone()),
    };

    // Build router
    let app = Router::new()
        // LNbits-compatible API
        .route("/api/v1/wallet", get(routes::wallet_api::get_wallet))
        .route(
            "/api/v1/wallets",
            post(routes::wallet_api::create_wallet).get(routes::wallet_api::list_wallets),
        )
        .route(
            "/api/v1/payments",
            post(routes::payment_api::create_or_pay).get(routes::payment_api::list_payments),
        )
        .route(
            "/api/v1/payments/:checking_id",
            get(routes::payment_api::get_payment),
        )
        // WebSocket real-time notifications
        .route(
            "/api/v1/ws/:wallet_id",
            get(routes::ws_handler::ws_handler),
        )
        // LNURL-pay (LUD-06)
        .route("/lnurlp/:wallet_id", get(lnurl::pay::lnurl_pay))
        .route(
            "/lnurlp/:wallet_id/callback",
            get(lnurl::pay::lnurl_pay_callback),
        )
        // QR code generation
        .route("/api/v1/qr/:data", get(routes::qr::qr_png))
        .route("/lnurlp/:wallet_id/qr", get(routes::qr::lnurl_qr))
        // Health check
        .route("/health", get(health))
        .route("/api/v1/health", get(health))
        // Middleware
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server started at http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "server": "LitePay",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
