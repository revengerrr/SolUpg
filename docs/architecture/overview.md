# UPG Architecture Overview

## System Design Principles

1. **Decentralization First** — Core payment logic lives on-chain (Solana programs). Off-chain services handle routing, caching, and UX optimization only.
2. **Token Agnostic** — Any SPL token can be used for payments. Auto-swap ensures merchants receive their preferred token.
3. **Instant Settlement** — Solana's ~400ms block time means payments settle in under a second.
4. **Permissionless** — Anyone can integrate as a merchant or payment provider via the open SDK/API.
5. **Modular** — Each layer is independently deployable and replaceable.

---

## Layer Breakdown

### Layer 1: API Gateway & SDK

**Purpose**: Single entry point for all external interactions.

**Responsibilities**:
- Authentication & authorization (API keys, JWT)
- Rate limiting & DDoS protection
- Request validation & transformation
- WebSocket connections for real-time payment status

**Technology**: Rust (Axum), Redis (rate limiting cache)

**Interfaces**:
- REST API (HTTPS) for standard merchant integration
- gRPC for high-performance service-to-service calls
- WebSocket for real-time payment notifications

---

### Layer 2: Routing Engine (Central Switch)

**Purpose**: Determines the optimal execution path for each transaction.

**Responsibilities**:
- Parse payment intent (source token, destination token, amount)
- Query Directory Service for wallet resolution
- Determine if swap is needed; if so, find optimal route via DEX aggregator
- Calculate fees (platform fee, network fee, swap slippage)
- Construct and submit Solana transaction

**Technology**: Rust (Axum), Apache Kafka (event streaming)

**Decision Flow**:
```
Payment Request
    │
    ├── Same token? ──→ Direct transfer via upg-payment program
    │
    ├── Different token? ──→ Route through upg-swap (Jupiter aggregator)
    │
    ├── Escrow needed? ──→ Route through upg-escrow program
    │
    └── Fee split needed? ──→ Include upg-splitter in transaction
```

---

### Layer 3A: Directory Service

**Purpose**: Human-readable alias resolution to Solana wallet addresses.

**Data Model**:
```
Alias (email/phone/username) → Wallet Address (Solana pubkey)
Merchant ID → { wallet, preferred_token, fee_config, metadata }
```

**Technology**: PostgreSQL (persistent store), Redis (lookup cache)

**Security**: Aliases are verified via email/SMS OTP before linking.

---

### Layer 3B: On-Chain Programs (Solana / Anchor)

**Purpose**: Trustless execution of payment logic on Solana blockchain.

**Programs**:

| Program | Description |
|---------|-------------|
| `upg-payment` | Core payment instruction: transfer SPL tokens with metadata |
| `upg-escrow` | Lock funds in escrow with release conditions (time, approval) |
| `upg-swap` | Cross-token payment via Jupiter/Raydium integration |
| `upg-splitter` | Distribute payment across multiple recipients (fee sharing) |

**Key Design Decisions**:
- All programs are upgradeable (via Anchor upgrade authority) during development, frozen on mainnet.
- Programs emit events via Solana logs for off-chain indexing.
- PDA (Program Derived Addresses) used for escrow vaults and state accounts.

---

### Layer 4: Clearing & Reconciliation

**Purpose**: Off-chain bookkeeping, analytics, and reporting.

**Responsibilities**:
- Index on-chain transactions via Solana RPC / Geyser plugin
- Maintain off-chain ledger for fast querying
- Generate merchant settlement reports
- Dashboard for transaction analytics

**Technology**: Rust (indexer), PostgreSQL (ledger), Grafana (dashboard)

---

### Layer 5: Compliance & Monitoring

**Purpose**: Regulatory compliance and fraud prevention.

**Responsibilities**:
- Real-time transaction monitoring (velocity checks, amount limits)
- AML/CFT screening against sanction lists
- Immutable audit trail (on-chain tx + off-chain enriched logs)
- Jurisdiction-specific reporting modules

**Technology**: Rust (rules engine), Prometheus + Grafana (metrics), PostgreSQL (audit logs)

---

## Data Flow: Complete Payment Lifecycle

```
1. Merchant creates payment request via API
2. API Gateway validates & forwards to Routing Engine
3. Routing Engine queries Directory Service for wallet resolution
4. Routing Engine determines execution path (direct/swap/escrow)
5. Routing Engine constructs Solana transaction with appropriate programs
6. Transaction submitted to Solana network
7. On-chain program executes (transfer/swap/escrow)
8. Transaction confirmed (~400ms)
9. Clearing Engine indexes the transaction
10. Merchant receives webhook notification
11. Compliance Engine logs and monitors the transaction
```

---

## Security Model

- **On-chain**: All fund movements require cryptographic signatures. No admin keys can move user funds.
- **API Gateway**: mTLS between services, API key + JWT for external clients.
- **Directory Service**: Encrypted PII storage, verified alias binding.
- **Key Management**: HSM for signing keys in production environment.
- **Audit**: All state changes logged on-chain (immutable) and off-chain (searchable).
