use std::fmt;

use libp2p::swarm::SwarmEvent;
use libp2p::{identify, mdns, relay, PeerId};
use libp2p_request_response::{self as request_response, InboundFailure, OutboundFailure};

use crate::libp2p::behaviour::BehaviourEvent;

use super::listen::EXPECTED_ADDRS_PER_LISTENER;
use super::{Id, NodeInner};

impl NodeInner {
    pub(crate) async fn on_swarm_event(&mut self, event: SwarmEvent<BehaviourEvent>) {
        match event {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                self.notify_connected(&peer_id).await;
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                self.maybe_reconnect_relay(peer_id).await;
                self.notify_disconnected(&peer_id).await;
            }
            SwarmEvent::OutgoingConnectionError { peer_id, .. } => {
                if let Some(peer_id) = peer_id {
                    self.maybe_reconnect_relay(peer_id).await;
                }
            }
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => {
                *self.listeners.entry(listener_id).or_insert(0) += 1;

                if self
                    .listeners
                    .values()
                    .all(|c| *c >= EXPECTED_ADDRS_PER_LISTENER)
                {
                    self.notify_listeners_ready().await;
                }

                self.notify_listening_on(&address).await;
            }
            SwarmEvent::Behaviour(event) => {
                self.on_behaviour_event(event).await;
            }
            _ => {}
        }
    }

    async fn maybe_reconnect_relay(&mut self, peer_id: PeerId) {
        if let Some(relay) = self.relays.get_mut(&peer_id) {
            relay.set_disconnected(&self.reconn_policy);
            if !relay.is_unreachable() {
                self.send_dial_intent(Id::Peer(peer_id), None).await
            }
        }
    }

    async fn on_behaviour_event(&mut self, event: BehaviourEvent) {
        match event {
            BehaviourEvent::Mdns(event) => self.on_mdns_event(event).await,
            BehaviourEvent::Relay(event) => self.on_relay_event(event).await,
            BehaviourEvent::Identify(event) => self.on_identify_event(event).await,
            BehaviourEvent::Messages(event) => self.on_messages_event(event).await,
            _ => {}
        }
    }

    async fn on_mdns_event(&mut self, event: mdns::Event) {
        match event {
            mdns::Event::Discovered(list) => {
                for (peer_id, addr) in list {
                    self.swarm.add_peer_address(peer_id, addr.clone());
                }
            }
            mdns::Event::Expired(list) => {
                for (_, addr) in list {
                    self.swarm.remove_external_address(&addr);
                }
            }
        }
    }

    async fn on_relay_event(&mut self, event: relay::client::Event) {
        match event {
            relay::client::Event::ReservationReqAccepted { relay_peer_id, .. } => {
                if let Some(relay) = self.relays.get_mut(&relay_peer_id) {
                    relay.set_relaying();
                }
            }
            _ => {}
        }
    }

    async fn on_identify_event(&mut self, event: identify::Event) {
        match event {
            identify::Event::Received { peer_id, .. } => {
                self.maybe_update_relay_on_identify(&peer_id, true, false)
                    .await;
            }
            identify::Event::Sent { peer_id, .. } => {
                self.maybe_update_relay_on_identify(&peer_id, false, true)
                    .await;
            }
            _ => {}
        }
    }

    async fn maybe_update_relay_on_identify(
        &mut self,
        peer_id: &PeerId,
        received: bool,
        sent: bool,
    ) {
        if let Some(relay) = self.relays.get_mut(&peer_id) {
            relay.update_connecting(sent, received);
            if relay.is_connected() {
                self.notify_relay_connected(peer_id.to_owned()).await;
            }
        }
    }

    async fn on_messages_event(
        &mut self,
        event: (String, request_response::Event<Vec<u8>, Vec<u8>>),
    ) {
        match event.1 {
            request_response::Event::Message {
                peer,
                message:
                    request_response::Message::Request {
                        request_id,
                        request,
                        channel,
                    },
            } => {
                self.response_channels
                    .insert((Id::Peer(peer), event.0.clone(), request_id), channel);
                self.notify_inbound_request(&peer, event.0, request, request_id)
                    .await;
            }
            request_response::Event::Message {
                peer,
                message:
                    request_response::Message::Response {
                        request_id,
                        response,
                    },
            } => {
                self.notify_inbound_response(&peer, event.0, response, request_id)
                    .await;
            }
            request_response::Event::InboundFailure { peer, error, .. } => {
                self.notify_error(Error::InboundMessageFailure(peer, error).to_string())
                    .await;
            }
            request_response::Event::OutboundFailure { peer, error, .. } => {
                self.notify_error(Error::OutboundMessageFailure(peer, error).to_string())
                    .await;
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
pub(crate) enum Error {
    InboundMessageFailure(PeerId, InboundFailure),
    OutboundMessageFailure(PeerId, OutboundFailure),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InboundMessageFailure(peer_id, inbound_failure) => write!(
                f,
                "Error while receiving a message from {peer_id}: {inbound_failure}"
            ),
            Error::OutboundMessageFailure(peer_id, outbound_failure) => write!(
                f,
                "Error while sending a message to {peer_id}: {outbound_failure}"
            ),
        }
    }
}
