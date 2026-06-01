#![allow(dead_code)]

pub mod config;
pub mod db;
pub mod error;
pub mod ln;
pub mod lnurl;
pub mod models;
pub mod routes;
pub mod state;
pub mod ws;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use config::Config;
use ln::mock::MockBackend;
use state::AppState;
use ws::NotificationHub;

/// Build the axum Router from a pre-configured AppState.
pub fn build_router(state: AppState) -> Router {
    Router::new()
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
        .route("/api/v1/ws/:wallet_id", get(routes::ws_handler::ws_handler))
        .route("/lnurlp/:wallet_id", get(lnurl::pay::lnurl_pay))
        .route(
            "/lnurlp/:wallet_id/callback",
            get(lnurl::pay::lnurl_pay_callback),
        )
        .route("/api/v1/qr/:data", get(routes::qr::qr_png))
        .route("/lnurlp/:wallet_id/qr", get(routes::qr::lnurl_qr))
        .route("/health", get(health))
        .route("/api/v1/health", get(health))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Build a complete app from just a database URL (for tests).
pub async fn build_app(database_url: &str) -> anyhow::Result<Router> {
    let pool = db::init_pool(database_url).await?;
    let ln_backend: Arc<dyn ln::LnBackend> = Arc::new(MockBackend::new());

    let config = Config {
        host: "0.0.0.0".to_string(),
        port: 9000,
        database_url: database_url.to_string(),
        ln_backend: config::LnBackendConfig::Mock,
        site_title: "LitePay Server".to_string(),
    };

    let state = AppState {
        db: pool,
        ln: ln_backend,
        hub: NotificationHub::new(),
        config: Arc::new(config),
    };

    Ok(build_router(state))
}

pub async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "server": "LitePay",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
