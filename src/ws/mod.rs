use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::models::payment::PaymentResponse;

/// Real-time payment notification hub.
/// Clients subscribe to a wallet_id and receive payment updates via WebSocket / SSE.
#[derive(Clone)]
pub struct NotificationHub {
    senders: Arc<RwLock<HashMap<String, broadcast::Sender<PaymentEvent>>>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PaymentEvent {
    pub wallet_id: String,
    pub payment: PaymentResponse,
}

impl NotificationHub {
    pub fn new() -> Self {
        Self {
            senders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe to payment events for a specific wallet.
    pub async fn subscribe(&self, wallet_id: &str) -> broadcast::Receiver<PaymentEvent> {
        let mut senders = self.senders.write().await;
        let sender = senders
            .entry(wallet_id.to_string())
            .or_insert_with(|| broadcast::channel(64).0);
        sender.subscribe()
    }

    /// Notify all subscribers of a wallet about a payment event.
    pub async fn notify(&self, event: PaymentEvent) {
        let senders = self.senders.read().await;
        if let Some(sender) = senders.get(&event.wallet_id) {
            // Ignore send errors (no active receivers)
            let _ = sender.send(event);
        }
    }
}
