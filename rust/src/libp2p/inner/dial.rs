use std::fmt;
use std::time::Duration;

use libp2p::swarm::DialError;
use libp2p::PeerId;

use crate::types::{MaybeFrom, ReconnectPolicy, Result};

use super::super::node::NodeId;
use super::NodeInner;

impl NodeInner {
    pub(super) async fn dial_node(&mut self, node: NodeId) -> Result<(), Error> {
        if let NodeId::Peer(peer_id) = node {
            if self.relays.contains_key(&peer_id) {
                self.dial_relay(&peer_id).await;
            } else {
                self.dial(NodeId::Peer(peer_id)).await?;
            }
        } else {
            self.dial(node).await?;
        }

        Ok(())
    }

    async fn dial(&mut self, node: NodeId) -> Result<(), Error> {
        match self.swarm.dial(node.clone()) {
            Ok(_) => {}
            Err(e) => {
                tracing::info!(peer=%node, %e, "dial peer failed");
                match e {
                    DialError::Aborted
                    | DialError::Denied { .. }
                    | DialError::Transport(_) => {
                        let retry_delay = Duration::from_secs(1);
                        match self.reconn_policy {
                            ReconnectPolicy::Never => return Err(Error::NodeUnreachable(node, e)),
                            ReconnectPolicy::Attempts(_) => { /* TODO: implement when required */ }
                            ReconnectPolicy::Always => {
                                tracing::info!(peer=%node, "retry dial peer after {retry_delay:?}");
                                self.send_dial_intent(node, Some(retry_delay)).await
                            },
                        }
                    }
                    _ => return Err(Error::NodeUnreachable(node, e)),
                }
            }
        }

        Ok(())
    }

    pub(super) async fn dial_relays(&mut self) {
        if self.relays.len() <= 0 {
            return;
        }

        let peer_ids = self
            .relays
            .iter()
            .filter_map(|(peer_id, relay)| {
                if relay.is_unreachable() {
                    None
                } else {
                    Some(peer_id)
                }
            })
            .cloned()
            .collect::<Vec<_>>();

        for peer_id in peer_ids {
            self.send_dial_intent(NodeId::Peer(peer_id.to_owned()), None)
                .await;
        }
    }

    async fn dial_relay(&mut self, peer_id: &PeerId) {
        if let Some(relay) = self.relays.get_mut(peer_id) {
            if relay.is_unreachable() {
                tracing::debug!(%peer_id, "skipping dialing peer, tried already and was unreachable");
            }

            match self.swarm.dial(relay.to_owned()) {
                Ok(_) => {
                    relay.set_connecting();
                }
                Err(DialError::Aborted)
                | Err(DialError::Denied { .. })
                | Err(DialError::Transport(_)) => {
                    relay.set_disconnected(&self.reconn_policy);

                    if !relay.is_unreachable() {
                        self.send_dial_intent(
                            NodeId::Peer(peer_id.to_owned()),
                            Some(Duration::from_secs(1)),
                        )
                        .await;
                    }
                }
                _ => {
                    relay.set_unreachable();
                    tracing::info!(%peer_id, "unreachable");
                }
            }
        }
    }

    pub(super) fn disconnect(&mut self, node: NodeId) {
        match node {
            NodeId::Peer(peer_id) => {
                self.disconnect_peer(peer_id);
            }
            NodeId::Addr(addr) => {
                if let Some(peer_id) = PeerId::maybe_from(addr.clone()) {
                    self.disconnect_peer(peer_id);
                } else {
                    tracing::debug!(%addr, "cannot disconnect from address, unknown peer id");
                }
            }
        }
    }

    pub(super) fn disconnect_peer(&mut self, peer_id: PeerId) {
        let _ = self.swarm.disconnect_peer_id(peer_id);
    }

    pub(super) fn disconnect_all(&mut self) {
        for peer_id in self.swarm.connected_peers().cloned().collect::<Vec<_>>() {
            self.disconnect_peer(peer_id);
        }
    }
}

#[derive(Debug)]
pub(super) enum Error {
    NodeUnreachable(NodeId, DialError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NodeUnreachable(node, dial_error) => {
                write!(f, "Peer {node} cannot be dialed: {dial_error}")
            }
        }
    }
}
