use solana_sdk::pubkey::Pubkey;
use crate::program_ids;

/// Derive the PaymentState PDA.
/// Seeds: [b"payment", payer, payment_id]
pub fn payment_state_pda(payer: &Pubkey, payment_id: &[u8; 16]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"payment", payer.as_ref(), payment_id.as_ref()],
        &program_ids::payment_program_id(),
    )
}

/// Derive the EscrowState PDA.
/// Seeds: [b"escrow", payer, escrow_id]
pub fn escrow_state_pda(payer: &Pubkey, escrow_id: &[u8; 16]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"escrow", payer.as_ref(), escrow_id.as_ref()],
        &program_ids::escrow_program_id(),
    )
}

/// Derive the EscrowVault PDA (token account).
/// Seeds: [b"escrow_vault", payer, escrow_id]
pub fn escrow_vault_pda(payer: &Pubkey, escrow_id: &[u8; 16]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"escrow_vault", payer.as_ref(), escrow_id.as_ref()],
        &program_ids::escrow_program_id(),
    )
}

/// Derive the SplitConfig PDA.
/// Seeds: [b"split_config", authority, config_id]
pub fn split_config_pda(authority: &Pubkey, config_id: &[u8; 16]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"split_config", authority.as_ref(), config_id.as_ref()],
        &program_ids::splitter_program_id(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    fn test_payer() -> Pubkey {
        "11111111111111111111111111111112".parse().unwrap()
    }

    fn test_id() -> [u8; 16] {
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
    }

    #[test]
    fn payment_pda_is_deterministic() {
        let (pda1, bump1) = payment_state_pda(&test_payer(), &test_id());
        let (pda2, bump2) = payment_state_pda(&test_payer(), &test_id());
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
        // PDA should not be on the ed25519 curve
        assert_ne!(pda1, test_payer());
    }

    #[test]
    fn escrow_pdas_are_different() {
        let id = test_id();
        let payer = test_payer();
        let (state, _) = escrow_state_pda(&payer, &id);
        let (vault, _) = escrow_vault_pda(&payer, &id);
        assert_ne!(state, vault, "escrow state and vault PDAs should differ");
    }

    #[test]
    fn different_ids_produce_different_pdas() {
        let payer = test_payer();
        let id1 = [1u8; 16];
        let id2 = [2u8; 16];
        let (pda1, _) = payment_state_pda(&payer, &id1);
        let (pda2, _) = payment_state_pda(&payer, &id2);
        assert_ne!(pda1, pda2);
    }

    #[test]
    fn different_payers_produce_different_pdas() {
        let id = test_id();
        let payer1: Pubkey = "11111111111111111111111111111112".parse().unwrap();
        let payer2: Pubkey = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse().unwrap();
        let (pda1, _) = payment_state_pda(&payer1, &id);
        let (pda2, _) = payment_state_pda(&payer2, &id);
        assert_ne!(pda1, pda2);
    }

    #[test]
    fn split_config_pda_deterministic() {
        let authority = test_payer();
        let id = test_id();
        let (pda1, b1) = split_config_pda(&authority, &id);
        let (pda2, b2) = split_config_pda(&authority, &id);
        assert_eq!(pda1, pda2);
        assert_eq!(b1, b2);
    }
}
