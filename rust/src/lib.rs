#[cfg(any(target_os = "android", target_os = "ios"))]
uniffi::setup_scaffolding!();

#[cfg(any(target_os = "android", target_os = "ios"))]
pub mod ffi;

pub mod base;
pub mod types;
pub mod utils;

#[cfg(feature = "libp2p")]
pub mod libp2p;

pub use crate::base::*;
