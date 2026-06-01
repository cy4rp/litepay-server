use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::state::AppState;

/// LNURL-pay step 1 response (LUD-06)
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LnurlPayResponse {
    pub tag: String,
    pub callback: String,
    pub min_sendable: i64,
    pub max_sendable: i64,
    pub metadata: String,
}

/// LNURL-pay step 2 (callback) response
#[derive(Serialize)]
pub struct LnurlPayCallbackResponse {
    pub pr: String,
    pub routes: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct LnurlPayCallbackParams {
    pub amount: i64, // millisatoshis
}

/// GET /lnurlp/:wallet_id — LNURL-pay endpoint (step 1)
pub async fn lnurl_pay(
    State(state): State<AppState>,
    Path(wallet_id): Path<String>,
) -> Result<Json<LnurlPayResponse>, AppError> {
    // Verify wallet exists
    crate::db::wallets::get_by_id(&state.db, &wallet_id)
        .await
        .map_err(|_| AppError::NotFound("wallet not found".to_string()))?;

    let metadata = lnurl_metadata(&wallet_id);

    let callback = format!(
        "http://{}:{}/lnurlp/{}/callback",
        state.config.host, state.config.port, wallet_id
    );

    Ok(Json(LnurlPayResponse {
        tag: "payRequest".to_string(),
        callback,
        min_sendable: 1_000,       // 1 sat in msats
        max_sendable: 100_000_000, // 100k sats in msats
        metadata,
    }))
}

/// GET /lnurlp/:wallet_id/callback?amount=<msats> — LNURL-pay callback (step 2)
pub async fn lnurl_pay_callback(
    State(state): State<AppState>,
    Path(wallet_id): Path<String>,
    Query(params): Query<LnurlPayCallbackParams>,
) -> Result<Json<LnurlPayCallbackResponse>, AppError> {
    let wallet = crate::db::wallets::get_by_id(&state.db, &wallet_id)
        .await
        .map_err(|_| AppError::NotFound("wallet not found".to_string()))?;

    if params.amount < 1_000 || params.amount > 100_000_000 {
        return Err(AppError::BadRequest(
            "amount out of range (1000-100000000 msat)".to_string(),
        ));
    }

    let metadata = lnurl_metadata(&wallet_id);
    let description_hash = Sha256::digest(metadata.as_bytes());
    let memo = format!("LitePay LNURL ({})", hex::encode(description_hash));

    let ln_result = state
        .ln
        .create_invoice(params.amount, &memo)
        .await
        .map_err(|e| AppError::LnBackend(e.to_string()))?;

    // Store payment in DB
    crate::db::payments::create(
        &state.db,
        &wallet.id,
        &ln_result.payment_request,
        &ln_result.payment_hash,
        params.amount,
        &memo,
        true,
    )
    .await?;

    Ok(Json(LnurlPayCallbackResponse {
        pr: ln_result.payment_request,
        routes: vec![],
    }))
}

fn lnurl_metadata(wallet_id: &str) -> String {
    serde_json::json!([
        ["text/plain", format!("Payment to LitePay wallet {}", wallet_id)]
    ])
    .to_string()
}
