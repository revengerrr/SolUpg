<p align="center">
  <h1 align="center">SolUPG: Solana Universal Payment Gateway</h1>
  <p align="center">
    A decentralized, open-source payment gateway built on Solana blockchain.<br/>
    Enabling universal, instant, and cross-token payments globally.
  </p>
</p>

<p align="center">
  <a href="#architecture">Architecture</a> •
  <a href="#features">Features</a> •
  <a href="#roadmap">Roadmap</a> •
  <a href="#getting-started">Getting Started</a> •
  <a href="#contributing">Contributing</a> •
  <a href="#license">License</a>
</p>

---

## What is SolUPG?

**Solana Universal Payment Gateway (SolUPG)** is a decentralized payment infrastructure built on the [Solana](https://solana.com) blockchain. Inspired by national payment gateway systems like Indonesia's GPN and India's UPI, SolUPG brings the same concept to the global crypto ecosystem: a single, unified network that connects wallets, merchants, and payment providers with instant settlement.

### The Problem

- Crypto payments are fragmented across chains, tokens, and protocols.
- Merchants struggle to accept diverse tokens without complex integrations.
- No unified standard exists for crypto payment routing and interoperability.
- Traditional payment gateways (Visa, Mastercard) charge high fees and settle in days.

### The Solution

SolUPG provides:
- **One gateway** to route payments across any SPL token on Solana.
- **Instant settlement**: powered by Solana's sub-second finality.
- **Auto-swap**: payers use any token; merchants receive their preferred token.
- **Verified Payment Identity**: link email or phone number to a wallet with OTP verification, complete with payment preferences (preferred token, fee split config). For crypto-native users, integrates with existing `.sol` domains via Solana Name Service.
- **Open standard**: anyone can integrate, no permission needed.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    END USERS / MERCHANTS                │
│              (Wallet, dApp, POS, E-commerce)             │
└──────────────────────┬──────────────────────────────────┘
                       │ REST / gRPC / WebSocket
                       ▼
┌──────────────────────────────────────────────────────────┐
│  LAYER 1: API GATEWAY & SDK                              │
│  Auth, rate limiting, merchant SDK (TS/Python)           │
└──────────────────────┬───────────────────────────────────┘
                       ▼
┌──────────────────────────────────────────────────────────┐
│  LAYER 2: ROUTING ENGINE (Central Switch)                │
│  Transaction routing, token resolution, DEX aggregation  │
└──────────┬───────────────────┬───────────────────────────┘
           │                   │
           ▼                   ▼
┌─────────────────┐  ┌────────────────────────────────────┐
│  LAYER 3A:      │  │  LAYER 3B: ON-CHAIN PROGRAMS       │
│  DIRECTORY      │  │  (Solana / Anchor / Rust)           │
│  SERVICE        │  │  Escrow, Swap, Splitter, Dispute    │
└─────────────────┘  └────────────────────────────────────┘
           │                   │
           ▼                   ▼
┌──────────────────────────────────────────────────────────┐
│  LAYER 4: CLEARING & RECONCILIATION                      │
│  Off-chain ledger, batch reconciliation, analytics       │
└──────────────────────┬───────────────────────────────────┘
                       ▼
┌──────────────────────────────────────────────────────────┐
│  LAYER 5: COMPLIANCE & MONITORING                        │
│  Fraud detection, AML, audit trail, regulatory reporting │
└──────────────────────────────────────────────────────────┘
```

> For detailed architecture documentation, see [`docs/architecture/`](./docs/architecture/).

---

## Features

| Feature | Description | Status |
|---------|-------------|--------|
| **On-Chain Escrow** | Secure fund holding until conditions are met | ✅ Phase 1 |
| **Multi-Token Payments** | Accept any SPL token, receive your preferred one | ✅ Phase 1 |
| **Fee Splitting** | Automatic fee distribution to all parties | ✅ Phase 1 |
| **Auto-Swap** | Integrated DEX aggregation (Jupiter) | ✅ Phase 1 |
| **Payment Routing** | Intelligent transaction routing engine | ✅ Phase 2 |
| **Directory Service** | Verified Payment Identity: email/phone → wallet + payment profile (preferred token, fee config). Integrates with `.sol` domains for crypto-native users. | ✅ Phase 2 |
| **REST/gRPC API** | Merchant-facing API gateway | ✅ Phase 3 |
| **TypeScript SDK** | Easy integration for web/Node.js apps | ✅ Phase 3 |
| **Reconciliation** | Off-chain clearing and reporting | ✅ Phase 4 |
| **Fraud Detection** | Real-time transaction monitoring | ✅ Phase 5 |
| **Mainnet Deployment** | Production-ready release | 🔲 Phase 6 |

---

## Tech Stack

| Component | Technology |
|-----------|-----------|
| On-Chain Programs | Rust, Anchor Framework |
| Routing Engine | Rust (Axum) |
| API Gateway | Rust (Axum) |
| Directory Service | PostgreSQL, Redis |
| Message Queue | Apache Kafka |
| SDK | TypeScript (@solana/web3.js) |
| Monitoring | Prometheus, Grafana |
| Clearing/Recon | Rust |

---

## Roadmap

| Phase | Description | Duration | Status |
|-------|-------------|----------|--------|
| **Phase 1** | On-Chain Programs (Escrow, Payment, Splitter, Swap) | 4-6 weeks | ✅ Complete |
| **Phase 2** | Routing Engine + Directory Service | 3-4 weeks | ✅ Complete |
| **Phase 3** | API Gateway + Merchant SDK | 2-3 weeks | ✅ Complete |
| **Phase 4** | Clearing, Reconciliation & Dashboard | 3-4 weeks | ✅ Complete |
| **Phase 5** | Compliance & Monitoring | 2-3 weeks | ✅ Complete |
| **Phase 6** | Testing, Security Audit & Mainnet Deploy | 4-6 weeks | 🔲 Not Started |

> Detailed documentation for each phase is available in [`docs/`](./docs/).

---

## Project Structure

```
solupg/
├── programs/                  # Solana on-chain programs (Rust/Anchor)
│   ├── solupg-escrow/         # Escrow program
│   ├── solupg-payment/        # Core payment program
│   ├── solupg-splitter/       # Fee splitting program
│   └── solupg-swap/           # Token swap integration
├── services/                  # Off-chain backend services (Rust/Axum)
│   ├── routing-engine/        # Central payment switch (47 tests)
│   ├── directory-service/     # Alias + merchant + OTP verification
│   ├── solupg-common/         # Shared types, PDA helpers, config
│   ├── api-gateway/           # REST API gateway + auth + rate limiting (8 tests)
│   ├── clearing-engine/       # Transaction indexer, reconciliation, dashboard API (Phase 4)
│   ├── monitoring/            # Fraud detection, audit trail, metrics, alerting (Phase 5)
│   ├── migrations/            # Shared database migrations
│   └── docker-compose.yml     # PostgreSQL 16 + Redis 7
├── sdk/                       # Client SDKs
│   ├── typescript/            # @solupg/sdk TypeScript package
│   └── python/                # Python SDK (Phase 3+)
├── docs/                      # Documentation
│   ├── architecture/          # Architecture deep-dives
│   ├── phase-1-onchain-programs/
│   ├── phase-2-routing-engine/
│   ├── phase-3-api-gateway/
│   ├── phase-4-clearing-reconciliation/
│   ├── phase-5-compliance-monitoring/
│   └── phase-6-testing-deployment/
├── tests/                     # Integration tests
├── CONTRIBUTING.md            # Contribution guidelines
├── LICENSE                    # Apache 2.0 License
└── README.md                  # This file
```

---

## Getting Started

> **Prerequisites**: Rust, Solana CLI, Anchor Framework, Node.js 18+, Docker

```bash
# Clone the repository
git clone https://github.com/revengerrr/SolUpg.git
cd SolUpg

# Build on-chain programs (Phase 1)
anchor build
anchor test

# Start infrastructure (Phase 2)
cd services
docker compose up -d          # PostgreSQL + Redis

# Build and test off-chain services
cargo build
cargo test                    # 55 unit tests

# Run services
cargo run -p directory-service   # Port 3001
cargo run -p routing-engine      # Port 3000
cargo run -p api-gateway         # Port 3002

# Build TypeScript SDK
cd ../sdk/typescript && npm install && npm run build

# Integration test (requires solana-test-validator)
cargo test --test integration_test -- --ignored
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture Overview](./docs/architecture/overview.md) | System design and layer breakdown |
| [Development Status](./docs/development/CURRENT_STATUS.md) | Live development progress |
| [Changelog](./CHANGELOG.md) | Version history and changes |
| [Documentation Guide](./docs/DOCUMENTATION_GUIDE.md) | How to maintain docs |

### Phase Documentation

| Phase | Documentation |
|-------|---------------|
| Phase 1 | [On-Chain Programs](./docs/phase-1-onchain-programs/README.md) |
| Phase 2 | [Routing Engine & Directory](./docs/phase-2-routing-engine/README.md) |
| Phase 3 | [API Gateway & SDK](./docs/phase-3-api-gateway/README.md) |
| Phase 4 | [Clearing & Reconciliation](./docs/phase-4-clearing-reconciliation/README.md) |
| Phase 5 | [Compliance & Monitoring](./docs/phase-5-compliance-monitoring/README.md) |
| Phase 6 | [Testing & Deployment](./docs/phase-6-testing-deployment/README.md) |

---

## Contributing

We welcome contributions! Please read our [Contributing Guide](./CONTRIBUTING.md) for details on our code of conduct, development process, and how to submit pull requests.

> **Note**: Every contribution must include documentation updates. See [Documentation Guide](./docs/DOCUMENTATION_GUIDE.md).

---

## License

This project is licensed under the **Apache License 2.0**. See the [LICENSE](./LICENSE) file for details.

---

<p align="center">
  Built with ❤️ for the future of global payments.
</p>
