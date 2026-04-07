use solupg_common::types::{FeeEstimate, PaymentRoute, RouteType};

/// Base Solana transaction fee (5000 lamports per signature).
const BASE_TX_FEE: u64 = 5000;

/// Platform fee in basis points (0.1% = 10 bps).
const PLATFORM_FEE_BPS: u64 = 10;

/// Estimate fees for a payment route.
pub fn estimate_fees(route: &PaymentRoute) -> FeeEstimate {
    let network_fee = match route.route_type {
        RouteType::DirectPay => BASE_TX_FEE,
        RouteType::SwapPay => BASE_TX_FEE * 2, // swap + transfer
        RouteType::Escrow => BASE_TX_FEE,
        RouteType::SplitPay => BASE_TX_FEE * 2, // might need more compute
    };

    let platform_fee = route.amount * PLATFORM_FEE_BPS / 10_000;

    let swap_cost = if route.route_type == RouteType::SwapPay {
        // Estimate swap cost based on slippage
        route.amount * route.slippage_bps as u64 / 10_000
    } else {
        0
    };

    FeeEstimate {
        network_fee,
        platform_fee,
        swap_cost,
        total: network_fee + platform_fee + swap_cost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;
    use uuid::Uuid;

    fn test_route(route_type: RouteType, amount: u64, slippage_bps: u16) -> PaymentRoute {
        PaymentRoute {
            intent_id: Uuid::new_v4(),
            route_type,
            payer: Pubkey::new_from_array([1u8; 32]),
            recipient_wallet: Pubkey::new_from_array([2u8; 32]),
            source_mint: Pubkey::new_from_array([0u8; 32]),
            destination_mint: Pubkey::new_from_array([0u8; 32]),
            amount,
            estimated_fee_lamports: 0,
            metadata: None,
            escrow_condition: None,
            escrow_expiry: None,
            split_config_pda: None,
            slippage_bps,
        }
    }

    #[test]
    fn direct_pay_fees() {
        let route = test_route(RouteType::DirectPay, 1_000_000, 100);
        let fees = estimate_fees(&route);
        assert_eq!(fees.network_fee, 5000);
        assert_eq!(fees.platform_fee, 1000); // 1M * 10/10000 = 1000
        assert_eq!(fees.swap_cost, 0);
        assert_eq!(fees.total, 6000);
    }

    #[test]
    fn swap_pay_fees_include_slippage() {
        let route = test_route(RouteType::SwapPay, 1_000_000, 100);
        let fees = estimate_fees(&route);
        assert_eq!(fees.network_fee, 10000); // 2x base
        assert_eq!(fees.platform_fee, 1000);
        assert_eq!(fees.swap_cost, 10000); // 1M * 100/10000 = 10000
        assert_eq!(fees.total, 21000);
    }

    #[test]
    fn escrow_fees() {
        let route = test_route(RouteType::Escrow, 500_000, 100);
        let fees = estimate_fees(&route);
        assert_eq!(fees.network_fee, 5000);
        assert_eq!(fees.platform_fee, 500); // 500K * 10/10000 = 500
        assert_eq!(fees.swap_cost, 0);
        assert_eq!(fees.total, 5500);
    }

    #[test]
    fn split_pay_fees() {
        let route = test_route(RouteType::SplitPay, 2_000_000, 100);
        let fees = estimate_fees(&route);
        assert_eq!(fees.network_fee, 10000); // 2x base
        assert_eq!(fees.platform_fee, 2000); // 2M * 10/10000 = 2000
        assert_eq!(fees.swap_cost, 0);
        assert_eq!(fees.total, 12000);
    }

    #[test]
    fn zero_amount_zero_platform_fee() {
        let route = test_route(RouteType::DirectPay, 0, 100);
        let fees = estimate_fees(&route);
        assert_eq!(fees.platform_fee, 0);
    }

    #[test]
    fn high_slippage_affects_swap_cost() {
        let route = test_route(RouteType::SwapPay, 1_000_000, 500); // 5%
        let fees = estimate_fees(&route);
        assert_eq!(fees.swap_cost, 50000); // 1M * 500/10000
    }
}
