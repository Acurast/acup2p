#[derive(Debug, Clone, Copy)]
pub enum Identity {
    Random,
    Seed([u8; 32]),
    Keypair(SecretKey),
}

#[derive(Debug, Clone, Copy)]
pub enum SecretKey {
    Ed25519([u8; 32]),
}

#[derive(Debug, Clone, Copy)]
pub enum PublicKey {
    Ed25519([u8; 32]),
}