#[derive(Debug, Clone, Copy)]
pub enum ReconnectPolicy {
    Never,
    Attempts(u8),
    Always,
}
