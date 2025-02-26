use crate::utils::bytes::FitIntoArr;
use crate::{base, types};

macro_rules! impl_from_key {
    ($key_type:tt) => {
        impl From<$key_type> for base::types::$key_type {
            fn from(value: $key_type) -> Self {
                match value {
                    $key_type::Ed25519(vec) => base::types::$key_type::Ed25519(vec.fit_into_arr()),
                }
            }
        }
    };
}

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

#[derive(uniffi::Enum, Debug, Clone)]
pub enum PublicKey {
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

impl_from_key!(SecretKey);
impl_from_key!(PublicKey);

pub type Event = base::types::Event;
pub type NodeId = base::types::NodeId;
pub type ReconnectPolicy = types::ReconnectPolicy;
