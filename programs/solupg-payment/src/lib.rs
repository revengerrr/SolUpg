use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("J2ZU7W5ee6X2vV58HM4EVSwBzJRmHtvxn8cLVzzvhwxK");

#[program]
pub mod solupg_payment {
    use super::*;

    /// Create a new payment intent with metadata.
    pub fn create_payment(
        ctx: Context<CreatePayment>,
        payment_id: [u8; 16],
        amount: u64,
        metadata: String,
    ) -> Result<()> {
        require!(amount > 0, PaymentError::ZeroAmount);
        require!(metadata.len() <= 256, PaymentError::MetadataTooLong);

        let payment = &mut ctx.accounts.payment_state;
        payment.payment_id = payment_id;
        payment.payer = ctx.accounts.payer.key();
        payment.recipient = ctx.accounts.recipient.key();
        payment.token_mint = ctx.accounts.token_mint.key();
        payment.amount = amount;
        payment.status = PaymentStatus::Pending;
        payment.metadata = metadata;
        payment.created_at = Clock::get()?.unix_timestamp;
        payment.bump = ctx.bumps.payment_state;

        emit!(PaymentCreated {
            payment_id,
            payer: payment.payer,
            recipient: payment.recipient,
            amount,
            token_mint: payment.token_mint,
        });

        Ok(())
    }

    /// Execute a pending payment: transfer tokens from payer to recipient.
    pub fn execute_payment(ctx: Context<ExecutePayment>) -> Result<()> {
        let payment = &mut ctx.accounts.payment_state;
        require!(
            payment.status == PaymentStatus::Pending,
            PaymentError::InvalidStatus
        );

        // Transfer SPL tokens from payer's token account to recipient's token account
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.payer_token_account.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, payment.amount)?;

        payment.status = PaymentStatus::Executed;
        payment.executed_at = Some(Clock::get()?.unix_timestamp);

        emit!(PaymentExecuted {
            payment_id: payment.payment_id,
            payer: payment.payer,
            recipient: payment.recipient,
            amount: payment.amount,
        });

        Ok(())
    }

    /// Cancel a pending payment (only the payer can cancel).
    pub fn cancel_payment(ctx: Context<CancelPayment>) -> Result<()> {
        let payment = &mut ctx.accounts.payment_state;
        require!(
            payment.status == PaymentStatus::Pending,
            PaymentError::InvalidStatus
        );

        payment.status = PaymentStatus::Cancelled;

        emit!(PaymentCancelled {
            payment_id: payment.payment_id,
        });

        Ok(())
    }
}

// ── Accounts ────────────────────────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(payment_id: [u8; 16])]
pub struct CreatePayment<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: recipient is just a pubkey stored in state, no signature required at creation
    pub recipient: UncheckedAccount<'info>,

    /// The SPL token mint for this payment
    /// CHECK: validated by token program during execution
    pub token_mint: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + PaymentState::INIT_SPACE,
        seeds = [b"payment", payer.key().as_ref(), &payment_id],
        bump,
    )]
    pub payment_state: Account<'info, PaymentState>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecutePayment<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        has_one = payer,
        has_one = token_mint,
    )]
    pub payment_state: Account<'info, PaymentState>,

    pub token_mint: Account<'info, anchor_spl::token::Mint>,

    #[account(
        mut,
        constraint = payer_token_account.owner == payer.key(),
        constraint = payer_token_account.mint == token_mint.key(),
    )]
    pub payer_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = recipient_token_account.owner == payment_state.recipient,
        constraint = recipient_token_account.mint == token_mint.key(),
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelPayment<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        has_one = payer,
        close = payer,
    )]
    pub payment_state: Account<'info, PaymentState>,
}

// ── State ───────────────────────────────────────────────────────────────────

#[account]
#[derive(InitSpace)]
pub struct PaymentState {
    pub payment_id: [u8; 16],
    pub payer: Pubkey,
    pub recipient: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub status: PaymentStatus,
    #[max_len(256)]
    pub metadata: String,
    pub created_at: i64,
    pub executed_at: Option<i64>,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum PaymentStatus {
    Pending,
    Executed,
    Cancelled,
}

// ── Events ──────────────────────────────────────────────────────────────────

#[event]
pub struct PaymentCreated {
    pub payment_id: [u8; 16],
    pub payer: Pubkey,
    pub recipient: Pubkey,
    pub amount: u64,
    pub token_mint: Pubkey,
}

#[event]
pub struct PaymentExecuted {
    pub payment_id: [u8; 16],
    pub payer: Pubkey,
    pub recipient: Pubkey,
    pub amount: u64,
}

#[event]
pub struct PaymentCancelled {
    pub payment_id: [u8; 16],
}

// ── Errors ──────────────────────────────────────────────────────────────────

#[error_code]
pub enum PaymentError {
    #[msg("Payment amount must be greater than zero")]
    ZeroAmount,
    #[msg("Metadata exceeds maximum length of 256 bytes")]
    MetadataTooLong,
    #[msg("Payment is not in the correct status for this operation")]
    InvalidStatus,
}
