use core::fmt;

use libp2p::PeerId;

use super::intent::Intent;
use super::NodeInner;

#[derive(Debug, Clone)]
pub(crate) enum Message {
    ListenersReady,
    RelayConnected(PeerId),
    Intent(Intent),
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::ListenersReady => write!(f, "Listeners are ready"),
            Message::RelayConnected(peer_id) => write!(
                f,
                "Successfully established a connection to relay {peer_id}"
            ),
            Message::Intent(intent) => write!(f, "Received intent: {intent}"),
        }
    }
}

impl NodeInner {
    pub(crate) async fn on_self_message(&mut self, event: Message) {
        match event {
            Message::ListenersReady => {
                self.dial_relays().await;
            }
            Message::RelayConnected(peer_id) => {
                if let Err(e) = self.listen_on_relay(&peer_id) {
                    tracing::debug!(relay=%peer_id, error=%e, "failed to set listener on relay");
                }
            }
            Message::Intent(intent) => {
                self.on_intent(intent).await;
            }
        }
    }

    pub(crate) async fn notify_listeners_ready(&mut self) {
        self.send_self_message(Message::ListenersReady).await;
    }

    pub(crate) async fn notify_relay_connected(&mut self, peer_id: PeerId) {
        self.send_self_message(Message::RelayConnected(peer_id))
            .await;
    }

    pub(crate) async fn send_self_message(&mut self, message: Message) {
        if let Err(e) = self.self_msg_tx.send(message.clone()).await {
            tracing::debug!(%message, error=%e, "failed to send the message due to an error");
        }
    }
}
