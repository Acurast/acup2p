use std::fmt;

#[cfg_attr(
    any(target_os = "android", target_os = "ios"),
    derive(uniffi::Enum, Debug, Clone, PartialEq, Eq, Hash)
)]
#[cfg_attr(
    not(any(target_os = "android", target_os = "ios")),
    derive(Debug, Clone, PartialEq, Eq, Hash)
)]
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
