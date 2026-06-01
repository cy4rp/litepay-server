use async_trait::async_trait;
use base64::Engine as _;
use serde::Deserialize;

use super::{InvoiceResult, LnBackend, PayResult, PaymentStatus};

/// LND REST API backend.
pub struct LndBackend {
    url: String,
    macaroon_hex: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct LndAddInvoiceResponse {
    r_hash: Option<String>,
    payment_request: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LndPayResponse {
    payment_hash: Option<String>,
    payment_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LndLookupResponse {
    settled: Option<bool>,
    state: Option<String>,
}

impl LndBackend {
    pub fn new(url: &str, macaroon_hex: &str, tls_cert: Option<&str>) -> anyhow::Result<Self> {
        let mut builder = reqwest::Client::builder();

        if let Some(cert_pem) = tls_cert {
            let cert = reqwest::Certificate::from_pem(cert_pem.as_bytes())?;
            builder = builder.add_root_certificate(cert);
        }

        // LND self-signed certs: allow for dev environments
        builder = builder.danger_accept_invalid_certs(true);

        let client = builder.build()?;

        Ok(Self {
            url: url.trim_end_matches('/').to_string(),
            macaroon_hex: macaroon_hex.to_string(),
            client,
        })
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Grpc-Metadata-macaroon",
            self.macaroon_hex
                .parse()
                .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("")),
        );
        headers
    }
}

#[async_trait]
impl LnBackend for LndBackend {
    async fn create_invoice(&self, amount_msat: i64, memo: &str) -> anyhow::Result<InvoiceResult> {
        let body = serde_json::json!({
            "value_msat": amount_msat.to_string(),
            "memo": memo,
        });

        let resp = self
            .client
            .post(format!("{}/v1/invoices", self.url))
            .headers(self.headers())
            .json(&body)
            .send()
            .await?;

        let data: LndAddInvoiceResponse = resp.json().await?;

        let r_hash_b64 = data
            .r_hash
            .ok_or_else(|| anyhow::anyhow!("missing r_hash"))?;
        let hash_bytes = base64::engine::general_purpose::STANDARD.decode(&r_hash_b64)?;
        let payment_hash = hex::encode(hash_bytes);

        let payment_request = data
            .payment_request
            .ok_or_else(|| anyhow::anyhow!("missing payment_request"))?;

        Ok(InvoiceResult {
            payment_hash,
            payment_request,
        })
    }

    async fn pay_invoice(&self, bolt11: &str) -> anyhow::Result<PayResult> {
        let body = serde_json::json!({
            "payment_request": bolt11,
        });

        let resp = self
            .client
            .post(format!("{}/v1/channels/transactions", self.url))
            .headers(self.headers())
            .json(&body)
            .send()
            .await?;

        let data: LndPayResponse = resp.json().await?;

        if let Some(err) = data.payment_error {
            if !err.is_empty() {
                return Err(anyhow::anyhow!("LND payment error: {}", err));
            }
        }

        let hash = data.payment_hash.unwrap_or_default();

        Ok(PayResult {
            payment_hash: hash,
            status: PaymentStatus::Paid,
        })
    }

    async fn check_payment(&self, payment_hash: &str) -> anyhow::Result<PaymentStatus> {
        let hash_bytes = hex::decode(payment_hash)?;
        let r_hash_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&hash_bytes);

        let resp = self
            .client
            .get(format!("{}/v1/invoice/{}", self.url, r_hash_b64))
            .headers(self.headers())
            .send()
            .await?;

        let data: LndLookupResponse = resp.json().await?;

        if data.settled == Some(true) {
            return Ok(PaymentStatus::Paid);
        }

        match data.state.as_deref() {
            Some("SETTLED") => Ok(PaymentStatus::Paid),
            Some("CANCELED") | Some("CANCELLED") => Ok(PaymentStatus::Failed),
            _ => Ok(PaymentStatus::Pending),
        }
    }

    fn name(&self) -> &str {
        "LND"
    }
}
