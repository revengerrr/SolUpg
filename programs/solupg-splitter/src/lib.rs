use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};

declare_id!("5aLb2o44AyWRYKMvpKiYU7PHHBzugQswRPvmhHSTuYHP");

/// Maximum number of recipients in a split configuration.
pub const MAX_RECIPIENTS: usize = 10;

/// Basis points denominator (100.00%).
pub const BASIS_POINTS_TOTAL: u16 = 10_000;

#[program]
pub mod solupg_splitter {
    use super::*;

    /// Create a split configuration defining how payments are distributed.
    pub fn create_split_config(
        ctx: Context<CreateSplitConfig>,
        config_id: [u8; 16],
        recipients: Vec<Pubkey>,
        ratios: Vec<u16>,
    ) -> Result<()> {
        require!(
            !recipients.is_empty() && recipients.len() <= MAX_RECIPIENTS,
            SplitterError::InvalidRecipientCount
        );
        require!(
            recipients.len() == ratios.len(),
            SplitterError::MismatchedLengths
        );

        let total: u16 = ratios.iter().sum();
        require!(
            total == BASIS_POINTS_TOTAL,
            SplitterError::RatiosMustSumTo10000
        );

        let config = &mut ctx.accounts.split_config;
        config.config_id = config_id;
        config.authority = ctx.accounts.authority.key();
        config.token_mint = ctx.accounts.token_mint.key();
        config.recipients = recipients;
        config.ratios = ratios;
        config.bump = ctx.bumps.split_config;

        emit!(SplitConfigCreated {
            config_id,
            authority: config.authority,
            token_mint: config.token_mint,
            num_recipients: config.recipients.len() as u8,
        });

        Ok(())
    }

    /// Execute a split: transfer tokens from sender, distributing to all recipients.
    pub fn execute_split<'info>(
        ctx: Context<'_, '_, 'info, 'info, ExecuteSplit<'info>>,
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, SplitterError::ZeroAmount);

        let config = &ctx.accounts.split_config;
        let remaining_accounts = &ctx.remaining_accounts;

        // remaining_accounts should contain token accounts for each recipient
        require!(
            remaining_accounts.len() == config.recipients.len(),
            SplitterError::MismatchedRecipientAccounts
        );

        let mut total_distributed: u64 = 0;

        for (i, (recipient_key, ratio)) in config
            .recipients
            .iter()
            .zip(config.ratios.iter())
            .enumerate()
        {
            let share = if i == config.recipients.len() - 1 {
                // Last recipient gets the remainder to avoid rounding dust
                amount - total_distributed
            } else {
                amount
                    .checked_mul(*ratio as u64)
                    .ok_or(SplitterError::Overflow)?
                    / BASIS_POINTS_TOTAL as u64
            };

            if share == 0 {
                continue;
            }

            // Validate the recipient token account
            let recipient_token_info = &remaining_accounts[i];
            let recipient_token_account =
                Account::<TokenAccount>::try_from(recipient_token_info)?;
            require!(
                recipient_token_account.owner == *recipient_key,
                SplitterError::InvalidRecipientAccount
            );
            require!(
                recipient_token_account.mint == config.token_mint,
                SplitterError::InvalidRecipientAccount
            );

            let transfer_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.sender_token_account.to_account_info(),
                    to: recipient_token_info.clone(),
                    authority: ctx.accounts.sender.to_account_info(),
                },
            );
            token::transfer(transfer_ctx, share)?;

            total_distributed += share;
        }

        emit!(SplitExecuted {
            config_id: config.config_id,
            total_amount: amount,
            num_recipients: config.recipients.len() as u8,
        });

        Ok(())
    }

    /// Update split ratios (authority only).
    pub fn update_split_config(
        ctx: Context<UpdateSplitConfig>,
        new_recipients: Vec<Pubkey>,
        new_ratios: Vec<u16>,
    ) -> Result<()> {
        require!(
            !new_recipients.is_empty() && new_recipients.len() <= MAX_RECIPIENTS,
            SplitterError::InvalidRecipientCount
        );
        require!(
            new_recipients.len() == new_ratios.len(),
            SplitterError::MismatchedLengths
        );

        let total: u16 = new_ratios.iter().sum();
        require!(
            total == BASIS_POINTS_TOTAL,
            SplitterError::RatiosMustSumTo10000
        );

        let config = &mut ctx.accounts.split_config;
        config.recipients = new_recipients;
        config.ratios = new_ratios;

        emit!(SplitConfigUpdated {
            config_id: config.config_id,
            authority: config.authority,
            num_recipients: config.recipients.len() as u8,
        });

        Ok(())
    }
}

// ── Accounts ────────────────────────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(config_id: [u8; 16])]
pub struct CreateSplitConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        space = 8 + SplitConfig::INIT_SPACE,
        seeds = [b"split_config", authority.key().as_ref(), &config_id],
        bump,
    )]
    pub split_config: Account<'info, SplitConfig>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteSplit<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    pub split_config: Account<'info, SplitConfig>,

    #[account(
        mut,
        constraint = sender_token_account.owner == sender.key(),
        constraint = sender_token_account.mint == split_config.token_mint,
    )]
    pub sender_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UpdateSplitConfig<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority,
    )]
    pub split_config: Account<'info, SplitConfig>,
}

// ── State ───────────────────────────────────────────────────────────────────

#[account]
#[derive(InitSpace)]
pub struct SplitConfig {
    pub config_id: [u8; 16],
    pub authority: Pubkey,
    pub token_mint: Pubkey,
    #[max_len(10)]
    pub recipients: Vec<Pubkey>,
    #[max_len(10)]
    pub ratios: Vec<u16>,
    pub bump: u8,
}

// ── Events ──────────────────────────────────────────────────────────────────

#[event]
pub struct SplitConfigCreated {
    pub config_id: [u8; 16],
    pub authority: Pubkey,
    pub token_mint: Pubkey,
    pub num_recipients: u8,
}

#[event]
pub struct SplitExecuted {
    pub config_id: [u8; 16],
    pub total_amount: u64,
    pub num_recipients: u8,
}

#[event]
pub struct SplitConfigUpdated {
    pub config_id: [u8; 16],
    pub authority: Pubkey,
    pub num_recipients: u8,
}

// ── Errors ──────────────────────────────────────────────────────────────────

#[error_code]
pub enum SplitterError {
    #[msg("Number of recipients must be between 1 and 10")]
    InvalidRecipientCount,
    #[msg("Recipients and ratios arrays must have the same length")]
    MismatchedLengths,
    #[msg("Ratios must sum to 10000 (basis points)")]
    RatiosMustSumTo10000,
    #[msg("Payment amount must be greater than zero")]
    ZeroAmount,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Number of remaining accounts must match number of recipients")]
    MismatchedRecipientAccounts,
    #[msg("Recipient token account owner or mint mismatch")]
    InvalidRecipientAccount,
}
