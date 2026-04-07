# Phase 2: Routing Engine & Directory Service

> **Status**: ✅ Complete (runtime-tested, 47 unit tests + integration test passing)
> **Estimated Duration**: 3-4 weeks
> **Dependencies**: Phase 1 (On-Chain Programs)

---

## Objective

Build the central payment switch (Routing Engine) and the Directory Service that together form the "brain" of SolUPG. The Routing Engine determines how to execute each payment, while the Directory Service manages **Verified Payment Identities**: OTP-verified email/phone-to-wallet bindings and merchant payment profiles (preferred token, fee split config). For `.sol` domain resolution, SolUPG integrates with Solana Name Service (SNS) rather than duplicating what already exists on-chain.

---

## Component 1: Routing Engine

### Purpose
The Routing Engine is the core off-chain service that:
- Receives payment intents from the API Gateway
- Determines the optimal execution path
- Constructs and submits Solana transactions
- Handles retries, confirmations, and failure recovery

### Architecture

```
Payment Intent (from API Gateway)
        │
        ▼
┌─────────────────────┐
│   Intent Parser     │ ← Validate & normalize request
├─────────────────────┤
│   Token Resolver    │ ← Determine source/dest tokens
├─────────────────────┤
│   Route Planner     │ ← Choose execution path
│   ├─ Direct Pay     │
│   ├─ Swap + Pay     │
│   ├─ Escrow         │
│   └─ Split Pay      │
├─────────────────────┤
│   TX Builder        │ ← Construct Solana transaction
├─────────────────────┤
│   TX Submitter      │ ← Submit, confirm, retry
└─────────────────────┘
        │
        ▼
  Solana Network
```

### Key Features
- **Intelligent Routing**: Automatically selects the cheapest/fastest execution path
- **Swap Optimization**: Queries multiple DEX routes via Jupiter for best price
- **Fee Calculation**: Pre-calculates all fees (network, platform, swap) before execution
- **Retry Logic**: Handles transaction failures with exponential backoff
- **Idempotency**: Payment intents have unique IDs to prevent double execution

### Technology
- **Language**: Rust
- **Framework**: Axum (HTTP server)
- **Message Queue**: Apache Kafka (async event processing)
- **Cache**: Redis (route caching, nonce management)
- **RPC**: Solana JSON-RPC + Geyser for confirmations

---

## Component 2: Directory Service

### Purpose
Stores **Verified Payment Identities** — the combination of a verified identifier (email or phone) and a payment profile (wallet, preferred token, fee config). This enables payments like "send 10 USDC to alice@example.com" and ensures the recipient receives it in their preferred token with the correct fee split applied.

For crypto-native identifiers (e.g. `alice.sol`), the Directory Service delegates to **Solana Name Service (SNS)** — SolUPG does not re-implement what SNS already solves on-chain.

### Data Model
```
┌─────────────────────────────────────────┐
│ Alias                                    │
│ ├─ type: email | phone | username        │
│ ├─ value: "alice@example.com"            │
│ ├─ wallet_address: "7xKXtg..."          │
│ ├─ verified: true                        │
│ └─ created_at: timestamp                 │
├─────────────────────────────────────────┤
│ Merchant                                 │
│ ├─ merchant_id: "merch_abc123"           │
│ ├─ name: "Coffee Shop"                   │
│ ├─ wallet_address: "9bYzt..."            │
│ ├─ preferred_token: USDC                 │
│ ├─ split_config: PDA address             │
│ ├─ webhook_url: "https://..."            │
│ └─ kyc_status: verified                  │
└─────────────────────────────────────────┘
```

### API Endpoints
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/aliases` | Register a new alias |
| GET | `/aliases/:alias` | Resolve alias to wallet |
| DELETE | `/aliases/:alias` | Remove alias binding |
| POST | `/merchants` | Register merchant |
| GET | `/merchants/:id` | Get merchant details |
| PUT | `/merchants/:id` | Update merchant config |

### Technology
- **Language**: Rust (Axum)
- **Database**: PostgreSQL (persistent storage)
- **Cache**: Redis (fast lookups)

---

## Deliverables Checklist

- [ ] Routing Engine with all route types (direct, swap, escrow, split)
- [ ] Directory Service with alias CRUD operations
- [ ] Kafka event pipeline for async processing
- [ ] Redis caching layer for performance
- [ ] Unit and integration tests
- [ ] API documentation (OpenAPI spec)
- [ ] Phase 2 completion documentation

---

## Next Phase

The Routing Engine and Directory Service will be exposed externally through the **API Gateway** (Phase 3).
