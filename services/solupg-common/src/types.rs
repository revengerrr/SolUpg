use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use uuid::Uuid;

/// Identifies a payment recipient by various methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum RecipientIdentifier {
    /// Direct Solana wallet address
    Wallet(String),
    /// Email alias (resolved via Directory Service)
    Email(String),
    /// Phone alias (resolved via Directory Service)
    Phone(String),
    /// SNS .sol domain (resolved on-chain)
    SolDomain(String),
    /// Merchant ID (resolved via Directory Service)
    Merchant(String),
}

/// The type of route to execute a payment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteType {
    /// Same token, direct transfer via solupg-payment
    DirectPay,
    /// Different tokens, swap via solupg-swap then pay
    SwapPay,
    /// Funds locked in escrow via solupg-escrow
    Escrow,
    /// Payment split across multiple recipients via solupg-splitter
    SplitPay,
}

/// Escrow release condition (mirrors on-chain enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReleaseCondition {
    TimeBased,
    AuthorityApproval,
    MutualApproval,
}

/// A payment intent submitted by a client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntent {
    /// Unique idempotency key
    pub intent_id: Uuid,
    /// Payer wallet address (base58)
    pub payer: String,
    /// Who to pay
    pub recipient: RecipientIdentifier,
    /// Source token mint (base58). None = SOL
    pub source_token: Option<String>,
    /// Destination token mint (base58). None = same as source
    pub destination_token: Option<String>,
    /// Amount in smallest token units (lamports / base units)
    pub amount: u64,
    /// Optional metadata string (max 256 chars)
    pub metadata: Option<String>,
    /// Force a specific route type (otherwise auto-detected)
    pub route_type: Option<RouteType>,
    /// Escrow-specific: release condition
    pub escrow_condition: Option<ReleaseCondition>,
    /// Escrow-specific: expiry as unix timestamp
    pub escrow_expiry: Option<i64>,
    /// Split-specific: existing split config PDA (base58)
    pub split_config: Option<String>,
    /// Max slippage in basis points (for swaps)
    pub slippage_bps: Option<u16>,
}

/// The planned route for a payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRoute {
    pub intent_id: Uuid,
    pub route_type: RouteType,
    pub payer: Pubkey,
    pub recipient_wallet: Pubkey,
    pub source_mint: Pubkey,
    pub destination_mint: Pubkey,
    pub amount: u64,
    pub estimated_fee_lamports: u64,
    pub metadata: Option<String>,
    /// Escrow fields
    pub escrow_condition: Option<ReleaseCondition>,
    pub escrow_expiry: Option<i64>,
    /// Split config PDA
    pub split_config_pda: Option<Pubkey>,
    /// Slippage
    pub slippage_bps: u16,
    /// Additional accounts for split recipients (ATAs resolved at build time)
    #[serde(default)]
    pub remaining_accounts: Vec<Pubkey>,
}

/// Status of a payment intent in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum IntentStatus {
    Pending,
    Processing,
    Submitted,
    Confirmed,
    Failed,
    Cancelled,
}

/// Fee breakdown for a payment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeEstimate {
    /// Solana network fee (lamports)
    pub network_fee: u64,
    /// Platform fee (token base units)
    pub platform_fee: u64,
    /// Estimated swap fee / slippage cost
    pub swap_cost: u64,
    /// Total estimated cost
    pub total: u64,
}
