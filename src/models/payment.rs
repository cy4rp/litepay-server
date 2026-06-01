use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Payment {
    pub checking_id: String,
    pub wallet_id: String,
    pub bolt11: String,
    pub payment_hash: String,
    pub amount_msat: i64,
    pub memo: String,
    pub status: String, // "pending", "paid", "failed"
    pub is_incoming: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvoiceRequest {
    /// Amount in satoshis (LNbits compat: "amount" field is sats)
    pub amount: i64,
    #[serde(default)]
    pub memo: Option<String>,
    #[serde(default)]
    pub webhook: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PayInvoiceRequest {
    pub bolt11: String,
}

#[derive(Debug, Serialize)]
pub struct CreateInvoiceResponse {
    pub payment_hash: String,
    pub payment_request: String,
    pub checking_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentResponse {
    pub checking_id: String,
    pub payment_hash: String,
    pub bolt11: String,
    pub amount: i64,
    pub memo: String,
    pub status: String,
    #[serde(rename = "pending")]
    pub is_pending: bool,
}

impl From<Payment> for PaymentResponse {
    fn from(p: Payment) -> Self {
        Self {
            checking_id: p.checking_id,
            payment_hash: p.payment_hash.clone(),
            bolt11: p.bolt11,
            amount: p.amount_msat / 1000, // msat -> sats for LNbits compat
            memo: p.memo,
            is_pending: p.status == "pending",
            status: p.status,
        }
    }
}
