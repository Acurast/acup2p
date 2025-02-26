use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::task::{Context, Poll};
use std::{error, io};

use libp2p::core::transport::PortUse;
use libp2p::core::Endpoint;
use libp2p::identity::Keypair;
use libp2p::swarm::handler::multi::MultiHandler;
use libp2p::swarm::{
    ConnectionDenied, ConnectionId, FromSwarm, InvalidProtocol, NetworkBehaviour, THandler,
    THandlerInEvent, THandlerOutEvent, ToSwarm,
};
use libp2p::{dcutr, identify, mdns, ping, relay, Multiaddr, PeerId, StreamProtocol};
use rand::seq::SliceRandom;
use rand::rng;
use {libp2p_request_response as request_response, libp2p_stream as stream};

use crate::libp2p::message;

const IDENTIFY_PROTOCOL: &str = "/ipfs/id/1.0.0";

#[derive(Debug)]
pub(super) enum Error {
    Mdns(io::Error),
    Messages((String, InvalidProtocol)),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Mdns(e) => write!(f, "Failed to configure the MDNS behaviour: {e}"),
            Error::Messages(e) => write!(
                f,
                "Failed to configure a Message behaviour ({}): {}",
                e.0, e.1
            ),
        }
    }
}

impl error::Error for Error {}

#[derive(NetworkBehaviour)]
pub(super) struct Behaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub relay: relay::client::Behaviour,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub messages: MultiBehaviour<String, message::Behaviour>,
    pub stream: stream::Behaviour,
}

impl Behaviour {
    pub fn new(
        key: &Keypair,
        relay_behaviour: relay::client::Behaviour,
        msg_protocols: &[&str],
    ) -> Result<Self, Error> {
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())
            .map_err(Error::Mdns)?;

        let identify = identify::Behaviour::new(identify::Config::new(
            IDENTIFY_PROTOCOL.into(),
            key.public(),
        ));
        let ping = ping::Behaviour::default();
        let dcutr = dcutr::Behaviour::new(key.public().to_peer_id());
        let messages = MultiBehaviour::new(msg_protocols, |p| {
            Ok(message::Behaviour::new(
                [(
                    StreamProtocol::try_from_owned(p.clone())
                        .map_err(|e| Error::Messages((p.clone(), e)))?,
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            ))
        })?;
        let stream = stream::Behaviour::new();

        Ok(Behaviour {
            mdns,
            relay: relay_behaviour,
            identify,
            ping,
            dcutr,
            messages,
            stream,
        })
    }
}

pub(super) struct MultiBehaviour<K, B>
where
    B: NetworkBehaviour,
{
    behaviours: HashMap<K, B>,
}

impl<K, B> MultiBehaviour<K, B>
where
    K: Clone + fmt::Debug + Hash + Eq + Send + 'static,
    B: NetworkBehaviour,
{
    pub fn new<P, F, E>(protocols: &[&P], builder: F) -> Result<Self, E>
    where
        P: ToOwned<Owned = K> + ?Sized,
        F: Fn(&K) -> Result<B, E>,
    {
        let behaviours = protocols
            .iter()
            .map(|&p| {
                let p = p.to_owned();
                let b = builder(&p)?;

                Ok((p, b))
            })
            .collect::<Result<_, E>>()?;

        Ok(MultiBehaviour { behaviours })
    }

    pub fn get_mut(&mut self, protocol: &K) -> Option<&mut B> {
        self.behaviours.get_mut(protocol)
    }
}

impl<K, B> NetworkBehaviour for MultiBehaviour<K, B>
where
    K: Clone + Debug + Hash + Eq + Send + 'static,
    B: NetworkBehaviour,
{
    type ConnectionHandler = MultiHandler<K, B::ConnectionHandler>;
    type ToSwarm = (K, B::ToSwarm);

    fn handle_pending_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        local_addr: &Multiaddr,
        remote_addr: &Multiaddr,
    ) -> Result<(), ConnectionDenied> {
        self.behaviours
            .iter_mut()
            .map(|(_, behaviour)| {
                behaviour.handle_pending_inbound_connection(
                    connection_id,
                    local_addr,
                    remote_addr,
                )?;

                Ok(())
            })
            .collect::<Result<(), ConnectionDenied>>()?;

        Ok(())
    }

    fn handle_established_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        local_addr: &Multiaddr,
        remote_addr: &Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        let handlers = self
            .behaviours
            .iter_mut()
            .map(|(protocol, behaviour)| {
                let handler = behaviour.handle_established_inbound_connection(
                    connection_id,
                    peer,
                    local_addr,
                    remote_addr,
                )?;

                Ok((protocol.clone(), handler))
            })
            .collect::<Result<Vec<_>, ConnectionDenied>>()?;

        Ok(MultiHandler::try_from_iter(handlers.into_iter())
            .expect("handlers already have unique keys"))
    }

    fn handle_pending_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        maybe_peer: Option<PeerId>,
        addresses: &[Multiaddr],
        effective_role: Endpoint,
    ) -> Result<Vec<Multiaddr>, ConnectionDenied> {
        let addrs = self
            .behaviours
            .iter_mut()
            .map(|(_, behaviour)| {
                let addrs = behaviour.handle_pending_outbound_connection(
                    connection_id,
                    maybe_peer,
                    addresses,
                    effective_role,
                )?;

                Ok(addrs)
            })
            .collect::<Result<Vec<Vec<Multiaddr>>, ConnectionDenied>>()?
            .into_iter()
            .flatten()
            .collect();

        Ok(addrs)
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        addr: &Multiaddr,
        role_override: Endpoint,
        port_use: PortUse,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        let handlers = self
            .behaviours
            .iter_mut()
            .map(|(protocol, behaviour)| {
                let handler = behaviour.handle_established_outbound_connection(
                    connection_id,
                    peer,
                    addr,
                    role_override,
                    port_use,
                )?;

                Ok((protocol.clone(), handler))
            })
            .collect::<Result<Vec<_>, ConnectionDenied>>()?;

        Ok(MultiHandler::try_from_iter(handlers.into_iter())
            .expect("handlers already have unique keys"))
    }

    fn on_swarm_event(&mut self, event: FromSwarm) {
        self.behaviours.iter_mut().for_each(|(_, behaviour)| {
            behaviour.on_swarm_event(event);
        });
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        match self.behaviours.get_mut(&event.0) {
            Some(behaviour) => {
                behaviour.on_connection_handler_event(peer_id, connection_id, event.1)
            }
            None => {}
        }
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        let mut behaviours: Vec<_> = self.behaviours.iter_mut().collect::<Vec<_>>();
        behaviours.shuffle(&mut rng());

        for (protocol, behaviour) in behaviours {
            match behaviour.poll(cx).map(|e| match e {
                ToSwarm::GenerateEvent(e) => ToSwarm::GenerateEvent((protocol.clone(), e)),
                ToSwarm::Dial { opts } => ToSwarm::Dial { opts },
                ToSwarm::ListenOn { opts } => ToSwarm::ListenOn { opts },
                ToSwarm::RemoveListener { id } => ToSwarm::RemoveListener { id },
                ToSwarm::NotifyHandler {
                    peer_id,
                    handler,
                    event,
                } => ToSwarm::NotifyHandler {
                    peer_id,
                    handler,
                    event: (protocol.clone(), event),
                },
                ToSwarm::NewExternalAddrCandidate(multiaddr) => {
                    ToSwarm::NewExternalAddrCandidate(multiaddr)
                }
                ToSwarm::ExternalAddrConfirmed(multiaddr) => {
                    ToSwarm::ExternalAddrConfirmed(multiaddr)
                }
                ToSwarm::ExternalAddrExpired(multiaddr) => ToSwarm::ExternalAddrExpired(multiaddr),
                ToSwarm::CloseConnection {
                    peer_id,
                    connection,
                } => ToSwarm::CloseConnection {
                    peer_id,
                    connection,
                },
                ToSwarm::NewExternalAddrOfPeer { peer_id, address } => {
                    ToSwarm::NewExternalAddrOfPeer { peer_id, address }
                }
                _ => panic!("unknown ToSwarm value"),
            }) {
                Poll::Ready(e) => return Poll::Ready(e),
                Poll::Pending => {}
            }
        }

        Poll::Pending
    }
}
