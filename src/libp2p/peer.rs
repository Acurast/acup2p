use core::fmt;

use libp2p::{identity, multiaddr, swarm::dial_opts::DialOpts, Multiaddr, PeerId};

use crate::types::node;

pub(crate) struct Peer {
    id: Id,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Id {
    Peer(PeerId),
    Addr(Multiaddr),
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Id::Peer(peer_id) => peer_id.fmt(f),
            Id::Addr(multiaddr) => multiaddr.fmt(f),
        }
    }
}

#[derive(Debug)]
pub(crate) enum ParseError {
    Peer(identity::ParseError),
    Addr(multiaddr::Error),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Peer(parse_error) => write!(f, "Could not parse peer: {parse_error}"),
            ParseError::Addr(error) => write!(f, "Could not parse address: {error}"),
        }
    }
}

impl std::error::Error for ParseError {}

impl TryFrom<&node::Id> for Id {
    type Error = ParseError;

    fn try_from(value: &node::Id) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            node::Id::Peer(peer_id) => Id::Peer(peer_id.parse().map_err(|e| ParseError::Peer(e))?),
            node::Id::Addr(addr) => Id::Addr(addr.parse().map_err(|e| ParseError::Addr(e))?),
        })
    }
}

impl From<&Id> for node::Id {
    fn from(value: &Id) -> Self {
        match value {
            Id::Peer(peer_id) => node::Id::Peer(peer_id.to_string()),
            Id::Addr(multiaddr) => node::Id::Addr(multiaddr.to_string()),
        }
    }
}

impl From<Id> for DialOpts {
    fn from(value: Id) -> Self {
        match value {
            Id::Peer(peer_id) => peer_id.into(),
            Id::Addr(multiaddr) => multiaddr.into(),
        }
    }
}