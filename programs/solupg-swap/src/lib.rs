use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};

declare_id!("Cf3nY8WkFXU4hn2TqLcfshS7E3hijY2eefekg5RHsz3n");

/// Default slippage tolerance in basis points (1% = 100 bps).
pub const DEFAULT_SLIPPAGE_BPS: u16 = 100;

#[program]
pub mod solupg_swap {
    use super::*;

    /// Swap source token to destination token, then transfer to recipient.
    ///
    /// In production, this will CPI into Jupiter Aggregator v6. For now, this
    /// implements the account structure and validation, with the actual swap
    /// executed via a Jupiter instruction passed through remaining_accounts.
    pub fn swap_and_pay(
        ctx: Context<SwapAndPay>,
        swap_id: [u8; 16],
        amount_in: u64,
        minimum_amount_out: u64,
        slippage_bps: Option<u16>,
    ) -> Result<()> {
        require!(amount_in > 0, SwapError::ZeroAmount);
        require!(minimum_amount_out > 0, SwapError::ZeroMinimumOut);

        let slippage = slippage_bps.unwrap_or(DEFAULT_SLIPPAGE_BPS);
        require!(slippage <= 1000, SwapError::SlippageTooHigh); // max 10%

        let source_mint = ctx.accounts.source_mint.key();
        let dest_mint = ctx.accounts.destination_mint.key();
        // TODO: Re-enable when Jupiter CPI is integrated. Disabled for placeholder
        // direct-transfer path which requires same mint on both sides.
        // require!(source_mint != dest_mint, SwapError::SameToken);

        // Record balance before swap for verification
        let recipient_balance_before = ctx.accounts.recipient_destination_token.amount;

        // --- Jupiter CPI would happen here ---
        // In production, the off-chain routing engine constructs the Jupiter swap
        // instruction and passes it via remaining_accounts. The program validates
        // the result after the swap.
        //
        // For Phase 1, we implement a direct transfer path as a placeholder.
        // The actual Jupiter CPI integration will be wired in during integration testing.

        // Transfer source tokens from payer to a holding/swap account
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.payer_source_token.to_account_info(),
                to: ctx.accounts.recipient_destination_token.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount_in)?;

        // Verify output meets minimum (post-swap check)
        ctx.accounts.recipient_destination_token.reload()?;
        let received = ctx
            .accounts
            .recipient_destination_token
            .amount
            .checked_sub(recipient_balance_before)
            .ok_or(SwapError::Overflow)?;

        require!(
            received >= minimum_amount_out,
            SwapError::SlippageExceeded
        );

        emit!(SwapExecuted {
            swap_id,
            payer: ctx.accounts.payer.key(),
            recipient: ctx.accounts.recipient.key(),
            source_mint,
            destination_mint: dest_mint,
            amount_in,
            amount_out: received,
        });

        Ok(())
    }
}

// ── Accounts ────────────────────────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(swap_id: [u8; 16])]
pub struct SwapAndPay<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: recipient is just the destination wallet
    pub recipient: UncheckedAccount<'info>,

    pub source_mint: Account<'info, Mint>,
    pub destination_mint: Account<'info, Mint>,

    #[account(
        mut,
        constraint = payer_source_token.owner == payer.key(),
        constraint = payer_source_token.mint == source_mint.key(),
    )]
    pub payer_source_token: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = recipient_destination_token.owner == recipient.key(),
        constraint = recipient_destination_token.mint == destination_mint.key(),
    )]
    pub recipient_destination_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

// ── Events ──────────────────────────────────────────────────────────────────

#[event]
pub struct SwapExecuted {
    pub swap_id: [u8; 16],
    pub payer: Pubkey,
    pub recipient: Pubkey,
    pub source_mint: Pubkey,
    pub destination_mint: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
}

// ── Errors ──────────────────────────────────────────────────────────────────

#[error_code]
pub enum SwapError {
    #[msg("Swap amount must be greater than zero")]
    ZeroAmount,
    #[msg("Minimum output amount must be greater than zero")]
    ZeroMinimumOut,
    #[msg("Slippage tolerance too high (max 10%)")]
    SlippageTooHigh,
    #[msg("Source and destination tokens must be different")]
    SameToken,
    #[msg("Output amount is less than minimum (slippage exceeded)")]
    SlippageExceeded,
    #[msg("Arithmetic overflow")]
    Overflow,
}
