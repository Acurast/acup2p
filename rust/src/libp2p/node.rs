use core::fmt;

use libp2p::swarm::dial_opts::DialOpts;
use libp2p::{identity, multiaddr, Multiaddr, PeerId};

use crate::base;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum NodeId {
    Peer(PeerId),
    Addr(Multiaddr),
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeId::Peer(peer_id) => peer_id.fmt(f),
            NodeId::Addr(multiaddr) => multiaddr.fmt(f),
        }
    }
}

#[derive(Debug)]
pub(super) enum ParseError {
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

impl TryFrom<&base::types::NodeId> for NodeId {
    type Error = ParseError;

    fn try_from(value: &base::types::NodeId) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            base::types::NodeId::Peer(peer_id) => {
                NodeId::Peer(peer_id.parse().map_err(|e| ParseError::Peer(e))?)
            }
            base::types::NodeId::Address(addr) => {
                NodeId::Addr(addr.parse().map_err(|e| ParseError::Addr(e))?)
            }
        })
    }
}

impl From<&NodeId> for base::types::NodeId {
    fn from(value: &NodeId) -> Self {
        match value {
            NodeId::Peer(peer_id) => base::types::NodeId::Peer(peer_id.to_string()),
            NodeId::Addr(multiaddr) => base::types::NodeId::Address(multiaddr.to_string()),
        }
    }
}

impl From<NodeId> for DialOpts {
    fn from(value: NodeId) -> Self {
        match value {
            NodeId::Peer(peer_id) => peer_id.into(),
            NodeId::Addr(multiaddr) => multiaddr.into(),
        }
    }
}
