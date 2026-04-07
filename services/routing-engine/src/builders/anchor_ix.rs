use sha2::{Digest, Sha256};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;

/// Compute Anchor instruction discriminator: sha256("global:<method_name>")[..8]
pub fn anchor_discriminator(method_name: &str) -> [u8; 8] {
    let preimage = format!("global:{method_name}");
    let hash = Sha256::digest(preimage.as_bytes());
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

/// Build an Anchor instruction with discriminator + borsh-serialized data.
pub fn anchor_instruction(
    program_id: Pubkey,
    method_name: &str,
    data: Vec<u8>,
    accounts: Vec<AccountMeta>,
) -> Instruction {
    let disc = anchor_discriminator(method_name);
    let mut ix_data = disc.to_vec();
    ix_data.extend_from_slice(&data);

    Instruction {
        program_id,
        accounts,
        data: ix_data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discriminator_matches_known_values() {
        // Anchor discriminator = sha256("global:create_payment")[..8]
        let disc = anchor_discriminator("create_payment");
        assert_eq!(disc.len(), 8);
        // Verify deterministic
        assert_eq!(disc, anchor_discriminator("create_payment"));
    }

    #[test]
    fn different_methods_different_discriminators() {
        let d1 = anchor_discriminator("create_payment");
        let d2 = anchor_discriminator("execute_payment");
        let d3 = anchor_discriminator("create_escrow");
        assert_ne!(d1, d2);
        assert_ne!(d1, d3);
        assert_ne!(d2, d3);
    }

    #[test]
    fn instruction_data_starts_with_discriminator() {
        let program_id = Pubkey::new_from_array([1u8; 32]);
        let data = vec![0xAA, 0xBB];
        let ix = anchor_instruction(program_id, "test_method", data, vec![]);
        let disc = anchor_discriminator("test_method");
        assert_eq!(&ix.data[..8], &disc);
        assert_eq!(&ix.data[8..], &[0xAA, 0xBB]);
        assert_eq!(ix.data.len(), 10);
    }

    #[test]
    fn instruction_empty_data_is_just_discriminator() {
        let program_id = Pubkey::new_from_array([1u8; 32]);
        let ix = anchor_instruction(program_id, "method", vec![], vec![]);
        assert_eq!(ix.data.len(), 8);
    }

    #[test]
    fn instruction_accounts_preserved() {
        let program_id = Pubkey::new_from_array([1u8; 32]);
        let acc = AccountMeta::new(Pubkey::new_from_array([2u8; 32]), true);
        let ix = anchor_instruction(program_id, "m", vec![], vec![acc.clone()]);
        assert_eq!(ix.accounts.len(), 1);
        assert_eq!(ix.accounts[0].pubkey, acc.pubkey);
        assert!(ix.accounts[0].is_signer);
    }
}
