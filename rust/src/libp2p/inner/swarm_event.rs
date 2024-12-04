use std::fmt;
use std::time::Duration;

use libp2p::identify::Info;
use libp2p::swarm::SwarmEvent;
use libp2p::{identify, mdns, relay, PeerId};
use libp2p_request_response::{self as request_response, InboundFailure, OutboundFailure};

use crate::libp2p::relay::ConnectionUpdate;

use super::super::behaviour::BehaviourEvent;
use super::super::node::NodeId;
use super::listen::ListenerType;
use super::NodeInner;

impl NodeInner {
    pub(super) async fn on_swarm_event(&mut self, event: SwarmEvent<BehaviourEvent>) {
        match event {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                self.notify_connected(&peer_id).await;
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                if let Some(e) = cause {
                    tracing::info!(error=%e, "relay connection closed unexpectedly");
                    self.maybe_reconnect_relay(peer_id, None).await;
                }
                self.notify_disconnected(&peer_id).await;
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                if let Some(peer_id) = peer_id {
                    tracing::info!(%error, "relay connection failed");
                    self.maybe_reconnect_relay(peer_id, None).await;
                }
            }
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => {
                let was_required = match self.tracked_listeners.remove(&listener_id) {
                    Some(listener_type) => self.required_listeners.remove(&listener_type),
                    None => false,
                };

                self.listeners.insert(listener_id);
                self.notify_listening_on(&address).await;

                if was_required && self.required_listeners.is_empty() {
                    self.notify_listeners_ready().await;
                }
            }
            SwarmEvent::ListenerClosed { listener_id, reason, .. } => {
                if let Some(ListenerType::CircuitRelay(peer_id)) = self.tracked_listeners.remove(&listener_id) {
                    if let Err(e) = reason {
                        tracing::info!(error=%e, "circuit relay closed unexpectedly");
                        self.maybe_reconnect_relay(peer_id, Some(Duration::from_secs(15))).await;
                    }
                }
            }
            SwarmEvent::Behaviour(event) => {
                self.on_behaviour_event(event).await;
            }
            _ => {}
        }
    }

    async fn maybe_reconnect_relay(&mut self, peer_id: PeerId, delay: Option<Duration>) {
        if let Some(relay) = self.relays.get_mut(&peer_id) {
            relay.set_disconnected(&self.reconn_policy);
            if !relay.is_unreachable() {
                self.send_dial_intent(NodeId::Peer(peer_id), delay).await
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
            identify::Event::Received {
                peer_id,
                info: Info { observed_addr, .. },
                ..
            } => {
                self.maybe_update_relay_on_identify(
                    &peer_id,
                    ConnectionUpdate::LearntObservedAddr(observed_addr),
                )
                .await;
            }
            identify::Event::Sent { peer_id, .. } => {
                self.maybe_update_relay_on_identify(&peer_id, ConnectionUpdate::SentObservedAddr)
                    .await;
            }
            _ => {}
        }
    }

    async fn maybe_update_relay_on_identify(&mut self, peer_id: &PeerId, update: ConnectionUpdate) {
        if let Some(relay) = self.relays.get_mut(&peer_id) {
            relay.update_connecting(update);
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
                let request_id = request_id.to_string();
                self.response_channels.insert(
                    (NodeId::Peer(peer), event.0.clone(), request_id.clone()),
                    channel,
                );
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
                let request_id = request_id.to_string();
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
pub(super) enum Error {
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
