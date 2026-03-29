# Phase 4: Clearing, Reconciliation & Dashboard

> **Status**: 🔲 Not Started
> **Estimated Duration**: 3-4 weeks
> **Dependencies**: Phase 3 (API Gateway)

---

## Objective

Build the off-chain clearing engine that indexes on-chain transactions, maintains a queryable ledger, generates merchant reports, and powers the analytics dashboard.

---

## Components

### 1. Transaction Indexer

**Purpose**: Listen to Solana blockchain events emitted by SolUPG programs and store them in a queryable database.

**Data Sources**:
- Solana RPC WebSocket subscriptions (program log events)
- Geyser plugin (for high-throughput production indexing)

**Indexed Data**:
| Field | Description |
|-------|-------------|
| `tx_signature` | Solana transaction signature |
| `payment_id` | SolUPG payment identifier |
| `payer` | Payer wallet address |
| `recipient` | Recipient wallet address |
| `amount` | Payment amount (raw + human-readable) |
| `token_mint` | SPL token mint address |
| `fee_amount` | Platform fee collected |
| `swap_details` | Source token, rate, slippage (if swapped) |
| `status` | confirmed, finalized, failed |
| `block_time` | On-chain timestamp |

### 2. Reconciliation Engine

**Purpose**: Cross-reference on-chain data with off-chain payment intents to ensure consistency.

**Reconciliation Checks**:
- Every payment intent has a matching on-chain transaction
- Amounts match (accounting for fees and swap slippage)
- No orphaned transactions (on-chain tx without payment intent)
- Settlement totals balance per merchant per period

**Schedule**: Real-time streaming + daily batch reconciliation

### 3. Merchant Dashboard

**Purpose**: Web dashboard for merchants to view transactions, analytics, and generate reports.

**Features**:
- Real-time transaction feed
- Revenue analytics (daily, weekly, monthly)
- Token breakdown (which tokens are used most)
- Settlement reports (downloadable CSV/PDF)
- Webhook configuration and delivery logs

---

## Deliverables Checklist

- [ ] Transaction indexer (Solana event listener + DB writer)
- [ ] Reconciliation engine with mismatch alerting
- [ ] Merchant dashboard (web UI)
- [ ] Report generation (CSV, PDF)
- [ ] API endpoints for dashboard data
- [ ] Phase 4 completion documentation

---

## Next Phase

With clearing and reporting in place, **Phase 5** adds compliance and monitoring capabilities.
