use crate::utils::bytes::FitIntoArr;
use crate::{base, types};

#[derive(uniffi::Enum, Clone)]
pub enum Identity {
    Random,
    Seed(Vec<u8>),
}

impl From<Identity> for base::types::Identity {
    fn from(value: Identity) -> Self {
        match value {
            Identity::Random => base::types::Identity::Random,
            Identity::Seed(vec) => base::types::Identity::Seed(vec.fit_into_arr()),
        }
    }
}

pub type Event = base::types::Event;
pub type NodeId = base::types::NodeId;
pub type ReconnectPolicy = types::ReconnectPolicy;
