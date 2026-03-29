# Phase 1: On-Chain Programs

> **Status**: 🔲 Not Started
> **Estimated Duration**: 4-6 weeks
> **Dependencies**: None (foundational layer)

---

## Objective

Build the core Solana smart contracts (programs) that handle all on-chain payment logic. These programs form the trustless foundation of SolUPG. All fund movements, escrow, swaps, and fee distributions happen here.

---

## Programs to Build

### 1. `solupg-payment` (Core Payment Program)

**Purpose**: Execute direct SPL token transfers with payment metadata.

**Instructions**:
| Instruction | Description |
|-------------|-------------|
| `create_payment` | Initialize a payment with amount, token, recipient, and metadata |
| `execute_payment` | Transfer tokens from payer to recipient |
| `cancel_payment` | Cancel a pending payment (before execution) |

**Accounts**:
- `PaymentState` PDA: stores payment details (amount, token_mint, payer, recipient, status, metadata)

**Events**:
- `PaymentCreated { payment_id, payer, recipient, amount, token_mint }`
- `PaymentExecuted { payment_id, tx_signature }`
- `PaymentCancelled { payment_id }`

---

### 2. `solupg-escrow` (Escrow Program)

**Purpose**: Hold funds in a trustless escrow until conditions are met.

**Instructions**:
| Instruction | Description |
|-------------|-------------|
| `create_escrow` | Lock tokens in escrow PDA vault |
| `release_escrow` | Release funds to recipient (by condition or authority) |
| `cancel_escrow` | Return funds to payer (timeout or mutual cancel) |
| `dispute_escrow` | Flag escrow for dispute resolution |

**Accounts**:
- `EscrowState` PDA: { payer, recipient, amount, token_mint, release_condition, expiry, status }
- `EscrowVault` PDA: token account holding escrowed funds

**Release Conditions**:
- `TimeBasedRelease`: auto-release after timestamp
- `AuthorityApproval`: release by designated authority
- `MutualApproval`: both parties must approve

---

### 3. `solupg-splitter` (Fee Splitting Program)

**Purpose**: Distribute a payment across multiple recipients with configurable ratios.

**Instructions**:
| Instruction | Description |
|-------------|-------------|
| `create_split_config` | Define split ratios (e.g., 97% merchant, 2% platform, 1% referrer) |
| `execute_split` | Transfer and split tokens according to config |
| `update_split_config` | Modify split ratios (admin only) |

**Accounts**:
- `SplitConfig` PDA: { recipients[], ratios[], token_mint, authority }

**Constraints**:
- Ratios must sum to 10000 (basis points, 100.00%)
- Maximum 10 recipients per split config

---

### 4. `solupg-swap` (Token Swap Integration)

**Purpose**: Enable cross-token payments by integrating with DEX aggregators.

**Instructions**:
| Instruction | Description |
|-------------|-------------|
| `swap_and_pay` | Swap source token to destination token, then transfer to recipient |

**Integration**:
- Jupiter Aggregator v6 for optimal swap routing
- Slippage protection (configurable, default 1%)
- Fallback routes if primary swap fails

**Flow**:
```
Payer (Token A) → Jupiter Swap → Token B → Recipient
                                        → Fee Split (if configured)
```

---

## Technical Specifications

### Framework & Tooling
- **Anchor Framework** v0.30+ (Rust-based Solana development framework)
- **Solana Program Library (SPL)** for token operations
- **Solana CLI** for deployment and testing

### Testing Strategy
- Unit tests: Per-instruction logic (Rust #[test])
- Integration tests: Full payment flows (Anchor test suite, TypeScript)
- Fuzz testing: Random inputs to find edge cases
- Security: Basic audit checklist before Phase 6 formal audit

### Deployment Plan
1. **Local validator** (solana-test-validator) during development
2. **Devnet** for integration testing
3. **Mainnet-beta** in Phase 6 after security audit

---

## Deliverables Checklist

- [ ] `solupg-payment` program with full test coverage
- [ ] `solupg-escrow` program with full test coverage
- [ ] `solupg-splitter` program with full test coverage
- [ ] `solupg-swap` program with Jupiter integration
- [ ] IDL (Interface Description Language) files for all programs
- [ ] TypeScript client bindings auto-generated from IDL
- [ ] Devnet deployment of all programs
- [ ] Phase 1 completion documentation

---

## Next Phase

After Phase 1 is complete, the on-chain programs will be consumed by the **Routing Engine** (Phase 2), which orchestrates transaction construction and submission.
