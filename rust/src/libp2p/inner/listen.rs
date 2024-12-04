use std::net::Ipv4Addr;
use std::{fmt, io};

use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId, TransportError};

use super::NodeInner;

#[derive(Debug, Hash, PartialEq, Eq)]
pub(super) enum ListenerType {
    TCP,
    QUIC,
    WebSocket,
    CircuitRelay(PeerId),
}

impl NodeInner {
    pub(super) fn listen(&mut self) -> Result<(), Error> {
        self.required_listeners.insert(ListenerType::TCP);
        self.required_listeners.insert(ListenerType::QUIC);
        self.required_listeners.insert(ListenerType::WebSocket);

        if let Err(e) = self.listen_on_tcp() {
            tracing::debug!(error=%e, "TCP listener not established");
            self.required_listeners.remove(&ListenerType::TCP);
        }

        if let Err(e) = self.listen_on_quic() {
            tracing::debug!(error=%e, "QUIC listener not established");
            self.required_listeners.remove(&ListenerType::QUIC);
        }

        if let Err(e) = self.listen_on_websocket() {
            tracing::debug!(error=%e, "WebSocket listener not established");
            self.required_listeners.remove(&ListenerType::WebSocket);
        }

        if self.tracked_listeners.is_empty() {
            return Err(Error::NoListeners);
        }

        Ok(())
    }

    fn listen_on_tcp(&mut self) -> Result<(), TransportError<io::Error>> {
        let addr_ipv4 = Multiaddr::from(Ipv4Addr::UNSPECIFIED).with(Protocol::Tcp(0));

        let listener = self.swarm.listen_on(addr_ipv4)?;
        self.tracked_listeners.insert(listener, ListenerType::TCP);

        Ok(())
    }

    fn listen_on_quic(&mut self) -> Result<(), TransportError<io::Error>> {
        let addr_ipv4 = Multiaddr::from(Ipv4Addr::UNSPECIFIED)
            .with(Protocol::Udp(0))
            .with(Protocol::QuicV1);

        let listener = self.swarm.listen_on(addr_ipv4)?;
        self.tracked_listeners.insert(listener, ListenerType::QUIC);

        Ok(())
    }

    fn listen_on_websocket(&mut self) -> Result<(), TransportError<io::Error>> {
        let addr_ipv4 = Multiaddr::from(Ipv4Addr::UNSPECIFIED)
            .with(Protocol::Tcp(0))
            .with(Protocol::Ws("/".into()));

        let listener = self.swarm.listen_on(addr_ipv4)?;
        self.tracked_listeners
            .insert(listener, ListenerType::WebSocket);

        Ok(())
    }

    pub(super) fn listen_on_relay(&mut self, peer_id: &PeerId) -> Result<(), Error> {
        let relay = self
            .relays
            .get_mut(peer_id)
            .ok_or(Error::NoRelay(*peer_id))?;

        if !relay.is_connected() {
            return Err(Error::RelayDisconnected(*peer_id));
        }

        match self
            .swarm
            .listen_on(relay.clone().with(Protocol::P2pCircuit))
        {
            Ok(listener_id) => {
                self.tracked_listeners
                    .insert(listener_id, ListenerType::CircuitRelay(peer_id.clone()));
            }
            Err(e) => {
                let addr: &Multiaddr = &relay;
                return Err(Error::NoRelayListener(addr.clone(), e));
            }
        }

        relay.set_pending_reservation();

        Ok(())
    }

    pub(super) fn stop_listeners(&mut self) {
        for listener_id in self.listeners.iter() {
            let _ = self.swarm.remove_listener(listener_id.clone());
        }

        self.listeners.clear();
    }
}

#[derive(Debug)]
pub(super) enum Error {
    NoListeners,

    NoRelay(PeerId),
    RelayDisconnected(PeerId),
    NoRelayListener(Multiaddr, TransportError<std::io::Error>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoListeners => write!(f, "Failed to establish any listener"),
            Error::NoRelay(peer_id) => write!(f, "No relay address found for peer {peer_id}"),
            Error::RelayDisconnected(peer_id) => write!(f, "Relay {peer_id} is disconnected"),
            Error::NoRelayListener(addr, err) => {
                write!(
                    f,
                    "Failed to establish circuit relay at address {addr}: {err}"
                )
            }
        }
    }
}
