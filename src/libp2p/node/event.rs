use libp2p::{Multiaddr, PeerId};
use libp2p_request_response::{InboundRequestId, OutboundRequestId};

use crate::context::Empty;
use crate::libp2p::context::Context;
use crate::types::event::Event;
use crate::types::message::ProtocolMessage;
use crate::types::node;

use super::{Id, NodeInner};

impl NodeInner {
    pub(crate) async fn notify_error(&mut self, error: String) {
        self.notify(Event::Error(error)).await;
    }

    pub(crate) async fn notify_listening_on(&mut self, addr: &Multiaddr) {
        self.notify(Event::ListeningOn(addr.to_string())).await;
    }

    pub(crate) async fn notify_connected(&mut self, peer_id: &PeerId) {
        self.notify(Event::Connected(node::Id::Peer(peer_id.to_string())))
            .await;
    }

    pub(crate) async fn notify_disconnected(&mut self, peer_id: &PeerId) {
        self.notify(Event::Disconnected(node::Id::Peer(peer_id.to_string())))
            .await;
    }

    pub(crate) async fn notify_inbound_request(
        &mut self,
        peer_id: &PeerId,
        protocol: String,
        request: Vec<u8>,
        request_id: InboundRequestId,
    ) {
        self.notify(Event::InboundRequest(
            node::Id::Peer(peer_id.to_string()),
            ProtocolMessage::new_with_context(protocol, request, request_id),
        ))
        .await;
    }

    pub(crate) async fn notify_inbound_response(
        &mut self,
        peer_id: &PeerId,
        protocol: String,
        response: Vec<u8>,
        request_id: OutboundRequestId,
    ) {
        self.notify(Event::InboundResponse(
            node::Id::Peer(peer_id.to_string()),
            ProtocolMessage::new_with_context(protocol, response, request_id),
        ))
        .await;
    }

    pub(crate) async fn notify_outbound_request(
        &mut self,
        node: &Id,
        message: ProtocolMessage<Empty>,
    ) {
        self.notify(Event::OutboundRequest(node.into(), message))
            .await;
    }

    pub(crate) async fn notify_outbound_response(
        &mut self,
        node: &Id,
        message: ProtocolMessage<InboundRequestId>,
    ) {
        self.notify(Event::OutboundResponse(node.into(), message))
            .await;
    }

    async fn notify(&mut self, event: Event<Context>) {
        if let Err(e) = self.ext_event_tx.send(event.clone()).await {
            tracing::debug!(%event, error=%e, "failed to notify due to an error");
        };
    }
}
