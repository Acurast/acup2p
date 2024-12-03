use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum NodeId {
    Peer(String),
    Address(String),
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeId::Peer(peer_id) => write!(f, "Peer({peer_id})"),
            NodeId::Address(addr) => write!(f, "Address({addr})"),
        }
    }
}
