use crate::utils::bytes::FitIntoArr;
use crate::{base, types};

#[derive(uniffi::Enum, Debug, Clone)]
pub enum Identity {
    Random,
    Seed(Vec<u8>),
    Keypair(SecretKey),
}

#[derive(uniffi::Enum, Debug, Clone)]
pub enum SecretKey {
    Ed25519(Vec<u8>),
}

impl From<Identity> for base::types::Identity {
    fn from(value: Identity) -> Self {
        match value {
            Identity::Random => base::types::Identity::Random,
            Identity::Seed(vec) => base::types::Identity::Seed(vec.fit_into_arr()),
            Identity::Keypair(secret_key) => base::types::Identity::Keypair(secret_key.into()),
        }
    }
}

impl From<SecretKey> for base::types::SecretKey {
    fn from(value: SecretKey) -> Self {
        match value {
            SecretKey::Ed25519(vec) => base::types::SecretKey::Ed25519(vec.fit_into_arr()),
        }
    }
}

pub type Event = base::types::Event;
pub type NodeId = base::types::NodeId;
pub type ReconnectPolicy = types::ReconnectPolicy;
