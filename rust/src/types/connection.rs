#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum ReconnectPolicy {
    Never,
    Attempts(u8),
    Always,
}
