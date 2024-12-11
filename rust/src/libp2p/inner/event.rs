use libp2p::{Multiaddr, PeerId};

use crate::base;

use super::super::node::NodeId;
use super::NodeInner;

impl NodeInner {
    pub(super) async fn notify_error(&mut self, error: String) {
        self.notify(base::types::Event::Error { cause: error })
            .await;
    }

    pub(super) async fn notify_listening_on(&mut self, addr: &Multiaddr) {
        self.notify(base::types::Event::ListeningOn {
            address: addr.to_string(),
        })
        .await;
    }

    pub(super) async fn notify_connected(&mut self, addr: &Multiaddr) {
        self.notify(base::types::Event::Connected {
            node: base::types::NodeId::Address { address: addr.to_string() },
        })
        .await;
    }

    pub(super) async fn notify_disconnected(&mut self, addr: &Multiaddr) {
        self.notify(base::types::Event::Disconnected {
            node: base::types::NodeId::Address { address: addr.to_string() },
        })
        .await;
    }

    pub(super) async fn notify_inbound_request(
        &mut self,
        peer_id: &PeerId,
        protocol: String,
        request: Vec<u8>,
        request_id: String,
    ) {
        self.notify(base::types::Event::InboundRequest {
            sender: base::types::NodeId::Peer { peer_id: peer_id.to_string() },
            request: base::types::InboundProtocolRequest {
                protocol,
                bytes: request,
                id: request_id,
            },
        })
        .await;
    }

    pub(super) async fn notify_inbound_response(
        &mut self,
        peer_id: &PeerId,
        protocol: String,
        response: Vec<u8>,
        request_id: String,
    ) {
        self.notify(base::types::Event::InboundResponse {
            sender: base::types::NodeId::Peer { peer_id: peer_id.to_string() },
            response: base::types::InboundProtocolResponse {
                protocol,
                bytes: response,
                id: request_id,
            },
        })
        .await;
    }

    pub(super) async fn notify_outbound_request(
        &mut self,
        node: &NodeId,
        message: base::types::OutboundProtocolRequest,
    ) {
        self.notify(base::types::Event::OutboundRequest {
            receiver: node.into(),
            request: message,
        })
        .await;
    }

    pub(super) async fn notify_outbound_response(
        &mut self,
        node: &NodeId,
        message: base::types::OutboundProtocolResponse,
    ) {
        self.notify(base::types::Event::OutboundResponse {
            receiver: node.into(),
            response: message,
        })
        .await;
    }

    async fn notify(&mut self, event: base::types::Event) {
        if let Err(e) = self.ext_event_tx.send(event.clone()).await {
            tracing::debug!(%event, error=%e, "failed to notify due to an error");
        };
    }
}
