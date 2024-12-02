use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Id {
    Peer(String),
    Addr(String),
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Id::Peer(peer_id) => write!(f, "Peer({peer_id})"),
            Id::Addr(addr) => write!(f, "Addr({addr})"),
        }
    }
}
