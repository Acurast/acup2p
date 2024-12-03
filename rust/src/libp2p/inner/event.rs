use libp2p::{Multiaddr, PeerId};

use crate::base;

use super::super::node::NodeId;
use super::NodeInner;

impl NodeInner {
    pub(crate) async fn notify_error(&mut self, error: String) {
        self.notify(base::types::Event::Error(error)).await;
    }

    pub(crate) async fn notify_listening_on(&mut self, addr: &Multiaddr) {
        self.notify(base::types::Event::ListeningOn(addr.to_string()))
            .await;
    }

    pub(crate) async fn notify_connected(&mut self, peer_id: &PeerId) {
        self.notify(base::types::Event::Connected(base::types::NodeId::Peer(
            peer_id.to_string(),
        )))
        .await;
    }

    pub(crate) async fn notify_disconnected(&mut self, peer_id: &PeerId) {
        self.notify(base::types::Event::Disconnected(base::types::NodeId::Peer(
            peer_id.to_string(),
        )))
        .await;
    }

    pub(crate) async fn notify_inbound_request(
        &mut self,
        peer_id: &PeerId,
        protocol: String,
        request: Vec<u8>,
        request_id: String,
    ) {
        self.notify(base::types::Event::InboundRequest(
            base::types::NodeId::Peer(peer_id.to_string()),
            base::types::InboundProtocolRequest {
                protocol,
                bytes: request,
                id: request_id,
            },
        ))
        .await;
    }

    pub(crate) async fn notify_inbound_response(
        &mut self,
        peer_id: &PeerId,
        protocol: String,
        response: Vec<u8>,
        request_id: String,
    ) {
        self.notify(base::types::Event::InboundResponse(
            base::types::NodeId::Peer(peer_id.to_string()),
            base::types::InboundProtocolResponse {
                protocol,
                bytes: response,
                id: request_id,
            },
        ))
        .await;
    }

    pub(crate) async fn notify_outbound_request(
        &mut self,
        node: &NodeId,
        message: base::types::OutboundProtocolRequest,
    ) {
        self.notify(base::types::Event::OutboundRequest(node.into(), message))
            .await;
    }

    pub(crate) async fn notify_outbound_response(
        &mut self,
        node: &NodeId,
        message: base::types::OutboundProtocolResponse,
    ) {
        self.notify(base::types::Event::OutboundResponse(node.into(), message))
            .await;
    }

    async fn notify(&mut self, event: base::types::Event) {
        if let Err(e) = self.ext_event_tx.send(event.clone()).await {
            tracing::debug!(%event, error=%e, "failed to notify due to an error");
        };
    }
}
