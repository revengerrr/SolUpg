use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// On-chain program IDs (from Anchor.toml localnet).
/// These must match the deployed program addresses.
pub fn payment_program_id() -> Pubkey {
    Pubkey::from_str("J2ZU7W5ee6X2vV58HM4EVSwBzJRmHtvxn8cLVzzvhwxK").unwrap()
}

pub fn escrow_program_id() -> Pubkey {
    Pubkey::from_str("CGHhZAS23gXemD87jh3CMuGUkvbbr94Rez25MH8dmVL6").unwrap()
}

pub fn splitter_program_id() -> Pubkey {
    Pubkey::from_str("5aLb2o44AyWRYKMvpKiYU7PHHBzugQswRPvmhHSTuYHP").unwrap()
}

pub fn swap_program_id() -> Pubkey {
    Pubkey::from_str("Cf3nY8WkFXU4hn2TqLcfshS7E3hijY2eefekg5RHsz3n").unwrap()
}
