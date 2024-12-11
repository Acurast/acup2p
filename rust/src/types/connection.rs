#[cfg_attr(
    any(target_os = "android", target_os = "ios"),
    derive(uniffi::Enum, Debug, Clone, Copy)
)]
#[cfg_attr(
    not(any(target_os = "android", target_os = "ios")),
    derive(Debug, Clone, Copy)
)]
pub enum ReconnectPolicy {
    Never,
    Attempts(u8),
    Always,
}
