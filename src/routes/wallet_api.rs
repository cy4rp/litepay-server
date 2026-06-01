use axum::{extract::State, Json};

use crate::error::AppError;
use crate::models::wallet::{CreateWalletRequest, WalletResponse};
use crate::state::AppState;

/// POST /api/v1/wallets — create a new wallet
pub async fn create_wallet(
    State(state): State<AppState>,
    Json(req): Json<CreateWalletRequest>,
) -> Result<Json<WalletResponse>, AppError> {
    let wallet = crate::db::wallets::create(&state.db, &req.name).await?;
    Ok(Json(wallet.into()))
}

/// GET /api/v1/wallets — list all wallets (admin)
pub async fn list_wallets(
    State(state): State<AppState>,
) -> Result<Json<Vec<WalletResponse>>, AppError> {
    let wallets = crate::db::wallets::list(&state.db).await?;
    Ok(Json(wallets.into_iter().map(|w| w.into()).collect()))
}

/// GET /api/v1/wallet — get wallet details for the authenticated wallet
pub async fn get_wallet(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<WalletResponse>, AppError> {
    let wallet = extract_wallet(&state, &headers).await?;
    Ok(Json(wallet.into()))
}

pub async fn extract_wallet(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Result<crate::models::wallet::Wallet, AppError> {
    let key = headers
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("missing X-Api-Key header".to_string()))?;

    crate::db::wallets::get_by_any_key(&state.db, key)
        .await
        .map_err(|_| AppError::Unauthorized("invalid API key".to_string()))
}

pub async fn extract_admin_wallet(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Result<crate::models::wallet::Wallet, AppError> {
    let key = headers
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("missing X-Api-Key header".to_string()))?;

    crate::db::wallets::get_by_admin_key(&state.db, key)
        .await
        .map_err(|_| AppError::Unauthorized("admin key required".to_string()))
}
