pub(super) mod peer_id {
    use libp2p::multiaddr::Protocol;
    use libp2p::{Multiaddr, PeerId};

    use crate::types::transform::MaybeFrom;

    impl MaybeFrom<Multiaddr> for PeerId {
        fn maybe_from(mut value: Multiaddr) -> Option<Self> {
            while let Some(proto) = value.pop() {
                if let Protocol::P2p(peer_id) = proto {
                    return Some(peer_id);
                }
            }

            None
        }
    }
}

pub(super) mod ed25519 {
    use crate::types::result::Result;
    use libp2p::identity;

    pub fn generate(seed: [u8; 32]) -> Result<identity::Keypair> {
        Ok(identity::Keypair::ed25519_from_bytes(seed)?)
    }
}
