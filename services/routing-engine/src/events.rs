use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

/// Payment lifecycle events (replaces Kafka for MVP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentEvent {
    IntentCreated { intent_id: Uuid },
    RouteResolved { intent_id: Uuid, route_type: String },
    TransactionSubmitted { intent_id: Uuid, signature: String },
    TransactionConfirmed { intent_id: Uuid, signature: String },
    TransactionFailed { intent_id: Uuid, error: String },
}

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<PaymentEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self { sender }
    }

    pub fn publish(&self, event: PaymentEvent) {
        // Ignore error if no receivers
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PaymentEvent> {
        self.sender.subscribe()
    }
}
