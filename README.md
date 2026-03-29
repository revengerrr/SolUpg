<p align="center">
  <h1 align="center">UPG — Universal Payment Gateway</h1>
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

## What is UPG?

**Universal Payment Gateway (UPG)** is a decentralized payment infrastructure built on the [Solana](https://solana.com) blockchain. Inspired by national payment gateway systems like Indonesia's GPN and India's UPI, UPG brings the same concept to the global crypto ecosystem — a single, unified network that connects wallets, merchants, and payment providers with instant settlement.

### The Problem

- Crypto payments are fragmented across chains, tokens, and protocols.
- Merchants struggle to accept diverse tokens without complex integrations.
- No unified standard exists for crypto payment routing and interoperability.
- Traditional payment gateways (Visa, Mastercard) charge high fees and settle in days.

### The Solution

UPG provides:
- **One gateway** to route payments across any SPL token on Solana.
- **Instant settlement** — powered by Solana's sub-second finality.
- **Auto-swap** — payers use any token; merchants receive their preferred token.
- **Universal addressing** — send payments via alias (email, username, phone) instead of wallet addresses.
- **Open standard** — anyone can integrate, no permission needed.

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
| **On-Chain Escrow** | Secure fund holding until conditions are met | 🔲 Phase 1 |
| **Multi-Token Payments** | Accept any SPL token, receive your preferred one | 🔲 Phase 1 |
| **Fee Splitting** | Automatic fee distribution to all parties | 🔲 Phase 1 |
| **Auto-Swap** | Integrated DEX aggregation (Jupiter) | 🔲 Phase 1 |
| **Payment Routing** | Intelligent transaction routing engine | 🔲 Phase 2 |
| **Directory Service** | Alias-to-wallet address resolution | 🔲 Phase 2 |
| **REST/gRPC API** | Merchant-facing API gateway | 🔲 Phase 3 |
| **TypeScript SDK** | Easy integration for web/Node.js apps | 🔲 Phase 3 |
| **Reconciliation** | Off-chain clearing and reporting | 🔲 Phase 4 |
| **Fraud Detection** | Real-time transaction monitoring | 🔲 Phase 5 |
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
| **Phase 1** | On-Chain Programs (Escrow, Payment, Splitter, Swap) | 4-6 weeks | 🔲 Not Started |
| **Phase 2** | Routing Engine + Directory Service | 3-4 weeks | 🔲 Not Started |
| **Phase 3** | API Gateway + Merchant SDK | 2-3 weeks | 🔲 Not Started |
| **Phase 4** | Clearing, Reconciliation & Dashboard | 3-4 weeks | 🔲 Not Started |
| **Phase 5** | Compliance & Monitoring | 2-3 weeks | 🔲 Not Started |
| **Phase 6** | Testing, Security Audit & Mainnet Deploy | 4-6 weeks | 🔲 Not Started |

> Detailed documentation for each phase is available in [`docs/`](./docs/).

---

## Project Structure

```
upg/
├── programs/                  # Solana on-chain programs (Rust/Anchor)
│   ├── upg-escrow/            # Escrow program
│   ├── upg-payment/           # Core payment program
│   ├── upg-splitter/          # Fee splitting program
│   └── upg-swap/              # Token swap integration
├── services/                  # Off-chain backend services
│   ├── routing-engine/        # Central payment switch
│   ├── api-gateway/           # REST/gRPC API
│   ├── directory-service/     # Alias resolution service
│   ├── clearing-engine/       # Reconciliation & reporting
│   └── monitoring/            # Fraud detection & metrics
├── sdk/                       # Client SDKs
│   ├── typescript/            # TypeScript/JavaScript SDK
│   └── python/                # Python SDK
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

> **Prerequisites**: Rust, Solana CLI, Anchor Framework, Node.js 18+

```bash
# Clone the repository
git clone https://github.com/revengerrr/upg.git
cd upg

# More setup instructions coming in Phase 1...
```

---

## Contributing

We welcome contributions! Please read our [Contributing Guide](./CONTRIBUTING.md) for details on our code of conduct, development process, and how to submit pull requests.

---

## License

This project is licensed under the **Apache License 2.0** — see the [LICENSE](./LICENSE) file for details.

---

<p align="center">
  Built with ❤️ for the future of global payments.
</p>
