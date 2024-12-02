use core::fmt;
use std::time::Duration;

use libp2p::PeerId;
use tokio::time::sleep;

use crate::libp2p::context::Context;
use crate::libp2p::peer::Id;
use crate::types::message::OutboundProtocolMessage;

use super::{Message, NodeInner};

#[derive(Debug, Clone)]
pub(crate) enum Intent {
    DirectMessage {
        peer: Id,
        message: OutboundProtocolMessage<Context>,
    },
    Dial(Id),
    Disconnect(Id),
    Close,
}

impl fmt::Display for Intent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Intent::DirectMessage { peer, message } => write!(f, "Send {message} to {peer}"),
            Intent::Dial(peer) => write!(f, "Dial {peer}"),
            Intent::Disconnect(peer) => write!(f, "Disconnect from {peer}"),
            Intent::Close => write!(f, "Close"),
        }
    }
}

impl NodeInner {
    pub(crate) async fn on_intent(&mut self, intent: Intent) {
        match intent {
            Intent::DirectMessage {
                peer: node,
                message,
            } => {
                if let Err(e) = self.send_direct_message(node, message).await {
                    self.notify_error(e.to_string()).await;
                }
            }
            Intent::Dial(node) => {
                if let Err(e) = self.dial_node(node).await {
                    self.notify_error(e.to_string()).await;
                };
            }
            Intent::Disconnect(node) => {
                self.disconnect(node);
            }
            Intent::Close => {
                self.ext_intent_rx.close();
                self.self_msg_rx.close();

                self.disconnect_all();
                self.stop_listeners();

                self.is_active = false;
            }
        }
    }

    pub(crate) async fn send_dial_peer_intent(&mut self, peer_id: PeerId, delay: Option<Duration>) {
        self.send_dial_intent(Id::Peer(peer_id), delay).await;
    }

    pub(crate) async fn send_dial_intent(&mut self, node: Id, delay: Option<Duration>) {
        if let Some(delay) = delay {
            sleep(delay).await;
        }

        self.send_intent(Intent::Dial(node)).await;
    }

    async fn send_intent(&mut self, intent: Intent) {
        self.send_self_message(Message::Intent(intent.clone()))
            .await;
    }
}
