use std::net::Ipv4Addr;
use std::{fmt, io};

use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId, TransportError};

use super::NodeInner;

pub(crate) const EXPECTED_ADDRS_PER_LISTENER: u8 = 2; // loopback + local network

impl NodeInner {
    pub(crate) fn listen(&mut self) -> Result<(), Error> {
        if let Err(e) = self.listen_on_tcp() {
            tracing::debug!(error=%e, "TCP listener not established");
        }

        if let Err(e) = self.listen_on_quic() {
            tracing::debug!(error=%e, "QUIC listener not established");
        }

        if let Err(e) = self.listen_on_websocket() {
            tracing::debug!(error=%e, "WebSocket listener not established");
        }

        if self.listeners.is_empty() {
            return Err(Error::NoListeners);
        }

        Ok(())
    }

    fn listen_on_tcp(&mut self) -> Result<(), TransportError<io::Error>> {
        let addr_ipv4 = Multiaddr::from(Ipv4Addr::UNSPECIFIED).with(Protocol::Tcp(0));

        let listener = self.swarm.listen_on(addr_ipv4)?;
        self.listeners.insert(listener, 0);

        Ok(())
    }

    fn listen_on_quic(&mut self) -> Result<(), TransportError<io::Error>> {
        let addr_ipv4 = Multiaddr::from(Ipv4Addr::UNSPECIFIED)
            .with(Protocol::Udp(0))
            .with(Protocol::QuicV1);

        let listener = self.swarm.listen_on(addr_ipv4)?;
        self.listeners.insert(listener, 0);

        Ok(())
    }

    fn listen_on_websocket(&mut self) -> Result<(), TransportError<io::Error>> {
        let addr_ipv4 = Multiaddr::from(Ipv4Addr::UNSPECIFIED)
            .with(Protocol::Tcp(0))
            .with(Protocol::Ws("/".into()));

        let listener = self.swarm.listen_on(addr_ipv4)?;
        self.listeners.insert(listener, 0);

        Ok(())
    }

    pub(crate) fn listen_on_relay(&mut self, peer_id: &PeerId) -> Result<(), Error> {
        let relay = self
            .relays
            .get_mut(peer_id)
            .ok_or(Error::NoRelay(*peer_id))?;

        if !relay.is_connected() {
            return Err(Error::RelayDisconnected(*peer_id));
        }

        if let Err(e) = self
            .swarm
            .listen_on(relay.clone().with(Protocol::P2pCircuit))
        {
            let addr: &Multiaddr = &relay;
            return Err(Error::NoRelayListener(addr.clone(), e));
        };
        relay.set_pending_reservation();

        Ok(())
    }

    pub(crate) fn stop_listeners(&mut self) {
        for listener_id in self.listeners.keys() {
            let _ = self.swarm.remove_listener(listener_id.clone());
        }

        self.listeners.clear();
    }
}

#[derive(Debug)]
pub(crate) enum Error {
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
                    "Failed to establish circut relay at address {addr}: {err}"
                )
            }
        }
    }
}
