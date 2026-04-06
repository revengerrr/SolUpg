use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};

declare_id!("CGHhZAS23gXemD87jh3CMuGUkvbbr94Rez25MH8dmVL6");

#[program]
pub mod solupg_escrow {
    use super::*;

    /// Create an escrow: lock tokens into a PDA vault.
    pub fn create_escrow(
        ctx: Context<CreateEscrow>,
        escrow_id: [u8; 16],
        amount: u64,
        release_condition: ReleaseCondition,
        expiry: i64,
    ) -> Result<()> {
        require!(amount > 0, EscrowError::ZeroAmount);

        let now = Clock::get()?.unix_timestamp;
        require!(expiry > now, EscrowError::ExpiryInPast);

        let escrow = &mut ctx.accounts.escrow_state;
        escrow.escrow_id = escrow_id;
        escrow.payer = ctx.accounts.payer.key();
        escrow.recipient = ctx.accounts.recipient.key();
        escrow.token_mint = ctx.accounts.token_mint.key();
        escrow.amount = amount;
        escrow.release_condition = release_condition;
        escrow.expiry = expiry;
        escrow.status = EscrowStatus::Active;
        escrow.created_at = now;
        escrow.bump = ctx.bumps.escrow_state;
        escrow.vault_bump = ctx.bumps.escrow_vault;

        // Transfer tokens from payer to escrow vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.payer_token_account.to_account_info(),
                to: ctx.accounts.escrow_vault.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        emit!(EscrowCreated {
            escrow_id,
            payer: escrow.payer,
            recipient: escrow.recipient,
            amount,
            token_mint: escrow.token_mint,
            expiry,
        });

        Ok(())
    }

    /// Release escrow funds to recipient.
    pub fn release_escrow(ctx: Context<ReleaseEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow_state;
        require!(
            escrow.status == EscrowStatus::Active,
            EscrowError::InvalidStatus
        );

        let now = Clock::get()?.unix_timestamp;

        // Validate release conditions
        match escrow.release_condition {
            ReleaseCondition::TimeBased => {
                require!(now >= escrow.expiry, EscrowError::NotYetReleasable);
            }
            ReleaseCondition::AuthorityApproval => {
                require!(
                    ctx.accounts.authority.key() == escrow.payer,
                    EscrowError::Unauthorized
                );
            }
            ReleaseCondition::MutualApproval => {
                // For mutual approval, authority must be the payer (recipient signs separately)
                require!(
                    ctx.accounts.authority.key() == escrow.payer,
                    EscrowError::Unauthorized
                );
            }
        }

        // Transfer from vault to recipient
        let escrow_id = escrow.escrow_id;
        let payer_key = escrow.payer;
        let seeds = &[
            b"escrow_vault",
            payer_key.as_ref(),
            &escrow_id,
            &[escrow.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.escrow_vault.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: ctx.accounts.escrow_vault.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, escrow.amount)?;

        escrow.status = EscrowStatus::Released;

        emit!(EscrowReleased {
            escrow_id,
            recipient: escrow.recipient,
            amount: escrow.amount,
        });

        Ok(())
    }

    /// Cancel escrow and return funds to payer (only after expiry or by mutual consent).
    pub fn cancel_escrow(ctx: Context<CancelEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow_state;
        require!(
            escrow.status == EscrowStatus::Active,
            EscrowError::InvalidStatus
        );

        let now = Clock::get()?.unix_timestamp;
        // Can cancel if expired, or if payer is cancelling before execution
        let is_expired = now > escrow.expiry;
        let is_payer = ctx.accounts.payer.key() == escrow.payer;
        require!(is_expired || is_payer, EscrowError::Unauthorized);

        // Return funds to payer
        let escrow_id = escrow.escrow_id;
        let payer_key = escrow.payer;
        let seeds = &[
            b"escrow_vault",
            payer_key.as_ref(),
            &escrow_id,
            &[escrow.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.escrow_vault.to_account_info(),
                to: ctx.accounts.payer_token_account.to_account_info(),
                authority: ctx.accounts.escrow_vault.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, escrow.amount)?;

        escrow.status = EscrowStatus::Cancelled;

        emit!(EscrowCancelled {
            escrow_id,
            payer: escrow.payer,
            amount: escrow.amount,
        });

        Ok(())
    }

    /// Flag an active escrow as disputed.
    pub fn dispute_escrow(ctx: Context<DisputeEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow_state;
        require!(
            escrow.status == EscrowStatus::Active,
            EscrowError::InvalidStatus
        );

        // Either payer or recipient can raise a dispute
        let caller = ctx.accounts.caller.key();
        require!(
            caller == escrow.payer || caller == escrow.recipient,
            EscrowError::Unauthorized
        );

        escrow.status = EscrowStatus::Disputed;

        emit!(EscrowDisputed {
            escrow_id: escrow.escrow_id,
            disputed_by: caller,
        });

        Ok(())
    }
}

// ── Accounts ────────────────────────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(escrow_id: [u8; 16])]
pub struct CreateEscrow<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: recipient pubkey stored in state
    pub recipient: UncheckedAccount<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        space = 8 + EscrowState::INIT_SPACE,
        seeds = [b"escrow", payer.key().as_ref(), &escrow_id],
        bump,
    )]
    pub escrow_state: Account<'info, EscrowState>,

    #[account(
        init,
        payer = payer,
        token::mint = token_mint,
        token::authority = escrow_vault,
        seeds = [b"escrow_vault", payer.key().as_ref(), &escrow_id],
        bump,
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = payer_token_account.owner == payer.key(),
        constraint = payer_token_account.mint == token_mint.key(),
    )]
    pub payer_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ReleaseEscrow<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = token_mint,
    )]
    pub escrow_state: Account<'info, EscrowState>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"escrow_vault", escrow_state.payer.as_ref(), &escrow_state.escrow_id],
        bump = escrow_state.vault_bump,
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = recipient_token_account.owner == escrow_state.recipient,
        constraint = recipient_token_account.mint == token_mint.key(),
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelEscrow<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        has_one = token_mint,
    )]
    pub escrow_state: Account<'info, EscrowState>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [b"escrow_vault", escrow_state.payer.as_ref(), &escrow_state.escrow_id],
        bump = escrow_state.vault_bump,
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = payer_token_account.owner == escrow_state.payer,
        constraint = payer_token_account.mint == token_mint.key(),
    )]
    pub payer_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct DisputeEscrow<'info> {
    pub caller: Signer<'info>,

    #[account(mut)]
    pub escrow_state: Account<'info, EscrowState>,
}

// ── State ───────────────────────────────────────────────────────────────────

#[account]
#[derive(InitSpace)]
pub struct EscrowState {
    pub escrow_id: [u8; 16],
    pub payer: Pubkey,
    pub recipient: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub release_condition: ReleaseCondition,
    pub expiry: i64,
    pub status: EscrowStatus,
    pub created_at: i64,
    pub bump: u8,
    pub vault_bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ReleaseCondition {
    TimeBased,
    AuthorityApproval,
    MutualApproval,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum EscrowStatus {
    Active,
    Released,
    Cancelled,
    Disputed,
}

// ── Events ──────────────────────────────────────────────────────────────────

#[event]
pub struct EscrowCreated {
    pub escrow_id: [u8; 16],
    pub payer: Pubkey,
    pub recipient: Pubkey,
    pub amount: u64,
    pub token_mint: Pubkey,
    pub expiry: i64,
}

#[event]
pub struct EscrowReleased {
    pub escrow_id: [u8; 16],
    pub recipient: Pubkey,
    pub amount: u64,
}

#[event]
pub struct EscrowCancelled {
    pub escrow_id: [u8; 16],
    pub payer: Pubkey,
    pub amount: u64,
}

#[event]
pub struct EscrowDisputed {
    pub escrow_id: [u8; 16],
    pub disputed_by: Pubkey,
}

// ── Errors ──────────────────────────────────────────────────────────────────

#[error_code]
pub enum EscrowError {
    #[msg("Escrow amount must be greater than zero")]
    ZeroAmount,
    #[msg("Expiry timestamp must be in the future")]
    ExpiryInPast,
    #[msg("Escrow is not in the correct status for this operation")]
    InvalidStatus,
    #[msg("Release conditions have not been met")]
    NotYetReleasable,
    #[msg("Caller is not authorized for this operation")]
    Unauthorized,
}
