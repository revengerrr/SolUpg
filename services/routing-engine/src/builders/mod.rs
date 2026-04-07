mod direct_pay;
mod escrow;
mod split_pay;
mod swap_pay;
mod anchor_ix;

use solana_sdk::transaction::Transaction;
use solupg_common::error::AppError;
use solupg_common::types::{PaymentRoute, RouteType};

/// Build an unsigned Solana transaction for a given route.
/// The transaction will need to be signed by the payer before submission.
pub fn build_transaction(route: &PaymentRoute) -> Result<Transaction, AppError> {
    match route.route_type {
        RouteType::DirectPay => direct_pay::build(route),
        RouteType::Escrow => escrow::build(route),
        RouteType::SplitPay => split_pay::build(route),
        RouteType::SwapPay => swap_pay::build(route),
    }
}
