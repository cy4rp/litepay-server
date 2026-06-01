pub mod lnd;
pub mod mock;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceResult {
    pub payment_hash: String,
    pub payment_request: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentStatus {
    Pending,
    Paid,
    Failed,
}

impl PaymentStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Paid => "paid",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayResult {
    pub payment_hash: String,
    pub status: PaymentStatus,
}

#[async_trait]
pub trait LnBackend: Send + Sync {
    /// Create a new invoice (receive).
    async fn create_invoice(&self, amount_msat: i64, memo: &str) -> anyhow::Result<InvoiceResult>;

    /// Pay an existing bolt11 invoice (send).
    async fn pay_invoice(&self, bolt11: &str) -> anyhow::Result<PayResult>;

    /// Check the status of a payment by its hash.
    async fn check_payment(&self, payment_hash: &str) -> anyhow::Result<PaymentStatus>;

    /// Return backend name for diagnostics.
    fn name(&self) -> &str;
}
