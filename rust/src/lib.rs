uniffi::setup_scaffolding!();

pub mod ffi;

pub mod base;
pub mod types;
pub mod utils;

#[cfg(feature = "libp2p")]
pub mod libp2p;

pub use crate::base::*;
