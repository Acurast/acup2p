#[derive(Debug, Clone, Copy)]
pub enum Identity {
    Random,
    Seed([u8; 32]),
}
