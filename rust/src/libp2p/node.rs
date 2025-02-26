use core::fmt;

use libp2p::{identity, multiaddr, swarm, Multiaddr, PeerId};

use crate::base;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeId {
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
pub enum ParseError {
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

impl NodeId {
    fn try_from_base(base: &base::types::NodeId) -> Result<NodeId, ParseError> {
        Ok(match base {
            base::types::NodeId::Peer { peer_id } => {
                NodeId::Peer(peer_id.parse().map_err(|e| ParseError::Peer(e))?)
            }
            base::types::NodeId::Address { address } => {
                NodeId::Addr(address.parse().map_err(|e| ParseError::Addr(e))?)
            }
        })
    }

    fn try_from_base_pk(pk: &base::types::PublicKey) -> Result<NodeId, identity::DecodingError> {
        let pk: identity::PublicKey = match pk {
            base::types::PublicKey::Ed25519(pk) => libp2p::identity::ed25519::PublicKey::try_from_bytes(pk)?.into()
        };
    
        Ok(NodeId::from_pk(&pk))
    }

    fn from_pk(pk: &identity::PublicKey) -> NodeId {
        NodeId::Peer(pk.to_peer_id())
    }

    fn to_base(&self) -> base::types::NodeId {
        match self {
            NodeId::Peer(peer_id) => base::types::NodeId::Peer {
                peer_id: peer_id.to_string(),
            },
            NodeId::Addr(multiaddr) => base::types::NodeId::Address {
                address: multiaddr.to_string(),
            },
        }
    }

    fn to_dial_opts(self) -> swarm::dial_opts::DialOpts {
        match self {
            NodeId::Peer(peer_id) => peer_id.into(),
            NodeId::Addr(multiaddr) => multiaddr.into(),
        }
    }
}

impl TryFrom<base::types::NodeId> for NodeId {
    type Error = ParseError;

    fn try_from(value: base::types::NodeId) -> std::result::Result<Self, Self::Error> {
        NodeId::try_from_base(&value)
    }
}

impl TryFrom<&base::types::NodeId> for NodeId {
    type Error = ParseError;

    fn try_from(value: &base::types::NodeId) -> std::result::Result<Self, Self::Error> {
        NodeId::try_from_base(value)
    }
}

impl From<NodeId> for base::types::NodeId {
    fn from(value: NodeId) -> Self {
        value.to_base()
    }
}

impl From<&NodeId> for base::types::NodeId {
    fn from(value: &NodeId) -> Self {
        value.to_base()
    }
}

impl From<NodeId> for swarm::dial_opts::DialOpts {
    fn from(value: NodeId) -> Self {
        value.to_dial_opts()
    }
}

impl TryFrom<base::types::PublicKey> for NodeId {
    type Error = identity::DecodingError;

    fn try_from(value: base::types::PublicKey) -> Result<Self, Self::Error> {
        NodeId::try_from_base_pk(&value)
    }
}

impl TryFrom<&base::types::PublicKey> for NodeId {
    type Error = identity::DecodingError;

    fn try_from(value: &base::types::PublicKey) -> Result<Self, Self::Error> {
        NodeId::try_from_base_pk(value)
    }
}

impl From<identity::PublicKey> for NodeId {
    fn from(value: identity::PublicKey) -> Self {
        NodeId::from_pk(&value)
    }
}

impl From<&identity::PublicKey> for NodeId {
    fn from(value: &identity::PublicKey) -> Self {
        NodeId::from_pk(value)
    }
}
