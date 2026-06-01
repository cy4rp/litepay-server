use async_trait::async_trait;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{InvoiceResult, LnBackend, PayResult, PaymentStatus};

/// Mock Lightning backend for development & testing.
/// Generates fake invoices and auto-settles payments after creation.
pub struct MockBackend {
    invoices: Arc<Mutex<HashMap<String, PaymentStatus>>>,
}

impl Default for MockBackend {
    fn default() -> Self {
        Self {
            invoices: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl MockBackend {
    pub fn new() -> Self {
        Self::default()
    }

    fn random_hash() -> String {
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        hex::encode(bytes)
    }

    fn fake_bolt11(payment_hash: &str, amount_msat: i64) -> String {
        let amount_sat = amount_msat / 1000;
        format!("lnbc{}n1mock{}", amount_sat, &payment_hash[..20])
    }
}

#[async_trait]
impl LnBackend for MockBackend {
    async fn create_invoice(&self, amount_msat: i64, _memo: &str) -> anyhow::Result<InvoiceResult> {
        let payment_hash = Self::random_hash();
        let payment_request = Self::fake_bolt11(&payment_hash, amount_msat);

        self.invoices
            .lock()
            .await
            .insert(payment_hash.clone(), PaymentStatus::Pending);

        Ok(InvoiceResult {
            payment_hash,
            payment_request,
        })
    }

    async fn pay_invoice(&self, _bolt11: &str) -> anyhow::Result<PayResult> {
        let payment_hash = Self::random_hash();
        self.invoices
            .lock()
            .await
            .insert(payment_hash.clone(), PaymentStatus::Paid);

        Ok(PayResult {
            payment_hash,
            status: PaymentStatus::Paid,
        })
    }

    async fn check_payment(&self, payment_hash: &str) -> anyhow::Result<PaymentStatus> {
        let invoices = self.invoices.lock().await;
        Ok(invoices
            .get(payment_hash)
            .cloned()
            .unwrap_or(PaymentStatus::Pending))
    }

    fn name(&self) -> &str {
        "MockBackend"
    }
}
