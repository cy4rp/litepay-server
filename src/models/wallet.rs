use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Wallet {
    pub id: String,
    pub name: String,
    pub admin_key: String,
    pub invoice_key: String,
    pub balance_msat: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWalletRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct WalletResponse {
    pub id: String,
    pub name: String,
    pub balance_msat: i64,
    pub admin_key: String,
    pub invoice_key: String,
}

impl From<Wallet> for WalletResponse {
    fn from(w: Wallet) -> Self {
        Self {
            id: w.id,
            name: w.name,
            balance_msat: w.balance_msat,
            admin_key: w.admin_key,
            invoice_key: w.invoice_key,
        }
    }
}

pub fn generate_api_key() -> String {
    Uuid::new_v4().to_string().replace('-', "")
}
