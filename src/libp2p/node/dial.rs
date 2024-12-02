use std::{fmt, time::Duration};

use libp2p::swarm::DialError;
use libp2p::PeerId;

use crate::types::result::Result;
use crate::types::transform::MaybeFrom;

use super::{Id, NodeInner};

impl NodeInner {
    pub(crate) async fn dial_node(&mut self, node: Id) -> Result<(), Error> {
        if let Id::Peer(peer_id) = node {
            if self.relays.contains_key(&peer_id) {
                self.dial_relay(&peer_id).await;
            } else {
                self.dial(Id::Peer(peer_id)).await?;
            }
        } else {
            self.dial(node).await?;
        }

        Ok(())
    }

    async fn dial(&mut self, node: Id) -> Result<(), Error> {
        match self.swarm.dial(node.clone()) {
            Ok(_) => {},
            Err(DialError::Aborted)
            | Err(DialError::Denied { .. })
            | Err(DialError::Transport(_)) => {
                let retry_delay = Duration::from_secs(1);
                match self.reconn_policy {
                    crate::connection::ReconnectPolicy::Never => {},
                    crate::connection::ReconnectPolicy::Attempts(_) => { /* TODO: implement when required */ },
                    crate::connection::ReconnectPolicy::Always => self.send_dial_intent(node, Some(retry_delay)).await,
                }
            }
            Err(e) => {
                return Err(Error::NodeUnreachable(node, e))
            }
        }

        Ok(())
    }

    pub(crate) async fn dial_relays(&mut self) {
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
            self.send_dial_intent(Id::Peer(peer_id.to_owned()), None).await;
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
                        self.send_dial_intent(Id::Peer(peer_id.to_owned()), Some(Duration::from_secs(1)))
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

    pub(crate) fn disconnect(&mut self, node: Id) {
        match node {
            Id::Peer(peer_id) => {
                self.disconnect_peer(peer_id);
            }
            Id::Addr(addr) => {
                if let Some(peer_id) = PeerId::maybe_from(addr.clone()) {
                    self.disconnect_peer(peer_id);
                } else {
                    tracing::debug!(%addr, "cannot disconnect from address, unknown peer id");
                }
            }
        }
    }

    pub(crate) fn disconnect_peer(&mut self, peer_id: PeerId) {
        let _ = self.swarm.disconnect_peer_id(peer_id);
    }

    pub(crate) fn disconnect_all(&mut self) {
        for peer_id in self.swarm.connected_peers().cloned().collect::<Vec<_>>() {
            self.disconnect_peer(peer_id);
        }
    }
}

#[derive(Debug)]
pub(crate) enum Error {
    NodeUnreachable(Id, DialError)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NodeUnreachable(node, dial_error) => write!(f, "Peer {node} cannot be dialed: {dial_error}"),
        }
    }
}