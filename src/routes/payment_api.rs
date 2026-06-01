use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::AppError;
use crate::models::payment::{
    CreateInvoiceRequest, CreateInvoiceResponse, PayInvoiceRequest, PaymentResponse,
};
use crate::routes::wallet_api::{extract_admin_wallet, extract_wallet};
use crate::state::AppState;
use crate::ws::PaymentEvent;

/// POST /api/v1/payments — create invoice (incoming) or pay invoice (outgoing)
/// LNbits compatible: if `bolt11` field present → pay, else → create invoice
pub async fn create_or_pay(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    if body.get("bolt11").is_some() {
        // Pay invoice (requires admin key)
        let wallet = extract_admin_wallet(&state, &headers).await?;
        let req: PayInvoiceRequest =
            serde_json::from_value(body).map_err(|e| AppError::BadRequest(e.to_string()))?;
        let result = pay_invoice_inner(&state, &wallet, &req).await?;
        Ok(Json(serde_json::to_value(result).unwrap()))
    } else {
        // Create invoice (invoice key or admin key)
        let wallet = extract_wallet(&state, &headers).await?;
        let req: CreateInvoiceRequest =
            serde_json::from_value(body).map_err(|e| AppError::BadRequest(e.to_string()))?;
        let result = create_invoice_inner(&state, &wallet, &req).await?;
        Ok(Json(serde_json::to_value(result).unwrap()))
    }
}

async fn create_invoice_inner(
    state: &AppState,
    wallet: &crate::models::wallet::Wallet,
    req: &CreateInvoiceRequest,
) -> Result<CreateInvoiceResponse, AppError> {
    let amount_msat = req.amount * 1000; // sats → msat
    let memo = req.memo.as_deref().unwrap_or("LitePay invoice");

    let ln_result = state
        .ln
        .create_invoice(amount_msat, memo)
        .await
        .map_err(|e| AppError::LnBackend(e.to_string()))?;

    let payment = crate::db::payments::create(
        &state.db,
        &wallet.id,
        &ln_result.payment_request,
        &ln_result.payment_hash,
        amount_msat,
        memo,
        true,
    )
    .await?;

    Ok(CreateInvoiceResponse {
        payment_hash: payment.payment_hash,
        payment_request: payment.bolt11,
        checking_id: payment.checking_id,
    })
}

async fn pay_invoice_inner(
    state: &AppState,
    wallet: &crate::models::wallet::Wallet,
    req: &PayInvoiceRequest,
) -> Result<PaymentResponse, AppError> {
    let pay_result = state
        .ln
        .pay_invoice(&req.bolt11)
        .await
        .map_err(|e| AppError::LnBackend(e.to_string()))?;

    // Estimate amount from the bolt11 (simplified; in production decode bolt11)
    let amount_msat = 0_i64; // Will be populated from bolt11 decoding in future

    let payment = crate::db::payments::create(
        &state.db,
        &wallet.id,
        &req.bolt11,
        &pay_result.payment_hash,
        amount_msat,
        "outgoing payment",
        false,
    )
    .await?;

    // Update status
    crate::db::payments::update_status(&state.db, &payment.checking_id, pay_result.status.as_str())
        .await?;

    // Debit wallet
    if pay_result.status == crate::ln::PaymentStatus::Paid {
        crate::db::wallets::update_balance(&state.db, &wallet.id, -amount_msat).await?;
    }

    let updated = crate::db::payments::get_by_checking_id(&state.db, &payment.checking_id).await?;
    let resp: PaymentResponse = updated.into();

    // Notify WebSocket subscribers
    state
        .hub
        .notify(PaymentEvent {
            wallet_id: wallet.id.clone(),
            payment: resp.clone(),
        })
        .await;

    Ok(resp)
}

/// GET /api/v1/payments — list payments for the authenticated wallet
pub async fn list_payments(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<PaymentResponse>>, AppError> {
    let wallet = extract_wallet(&state, &headers).await?;
    let payments = crate::db::payments::list_by_wallet(&state.db, &wallet.id).await?;
    Ok(Json(payments.into_iter().map(|p| p.into()).collect()))
}

/// GET /api/v1/payments/:checking_id — get a specific payment (LNbits compat)
pub async fn get_payment(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(checking_id): Path<String>,
) -> Result<Json<PaymentResponse>, AppError> {
    let _wallet = extract_wallet(&state, &headers).await?;

    let payment = crate::db::payments::get_by_checking_id(&state.db, &checking_id)
        .await
        .map_err(|_| AppError::NotFound("payment not found".to_string()))?;

    // If pending, check with LN backend for updates
    if payment.status == "pending" {
        let ln_status = state
            .ln
            .check_payment(&payment.payment_hash)
            .await
            .unwrap_or(crate::ln::PaymentStatus::Pending);

        if ln_status != crate::ln::PaymentStatus::Pending {
            crate::db::payments::update_status(&state.db, &checking_id, ln_status.as_str())
                .await?;

            // If incoming payment is now paid, credit wallet
            if ln_status == crate::ln::PaymentStatus::Paid && payment.is_incoming {
                crate::db::wallets::update_balance(
                    &state.db,
                    &payment.wallet_id,
                    payment.amount_msat,
                )
                .await?;

                let updated =
                    crate::db::payments::get_by_checking_id(&state.db, &checking_id).await?;
                let resp: PaymentResponse = updated.into();

                // Notify WebSocket subscribers
                state
                    .hub
                    .notify(PaymentEvent {
                        wallet_id: payment.wallet_id.clone(),
                        payment: resp.clone(),
                    })
                    .await;

                return Ok(Json(resp));
            }
        }
    }

    let payment = crate::db::payments::get_by_checking_id(&state.db, &checking_id).await?;
    Ok(Json(payment.into()))
}
