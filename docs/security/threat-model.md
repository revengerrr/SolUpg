# SolUPG Threat Model

> **Status**: Initial draft, Phase 6
> **Scope**: On-chain programs + off-chain services + client SDK
> **Methodology**: STRIDE per trust boundary
> **Owner**: Security working group (TBD)

---

## 1. System Overview

SolUPG is a Solana-based payment gateway composed of:

- **4 on-chain programs** (Anchor): `solupg-payment`, `solupg-escrow`, `solupg-splitter`, `solupg-swap`.
- **5 off-chain Rust services**: routing-engine (3000), directory-service (3001), api-gateway (3002), clearing-engine (3003), monitoring (3004).
- **TypeScript SDK** for merchant integration.
- **PostgreSQL + Redis** as persistence/cache.
- **Kafka** (planned) for async event fan-out.

## 2. Trust Boundaries

```
[Internet] ──► [API Gateway] ──► [Routing Engine] ──► [Solana RPC]
                   │                   │                    │
                   ├──► [Directory]    ├──► [Clearing]      ├──► [On-chain programs]
                   │                   │
                   └──► [Auth DB]      └──► [Monitoring]
```

| # | Boundary | Inside | Outside |
|---|----------|--------|---------|
| B1 | Public → API Gateway | API Gateway process | Internet clients (merchants, dApps, wallets) |
| B2 | API Gateway → Routing Engine | Internal service mesh | API Gateway request handlers |
| B3 | Routing Engine → Solana RPC | Solana cluster | Routing engine process |
| B4 | Services → PostgreSQL | DB process | All off-chain services |
| B5 | Services → Redis | Cache process | All off-chain services |
| B6 | Admin → Services | Operator laptop / CI | Production services |
| B7 | On-chain program → Token program | SPL Token program | SolUPG programs |

## 3. Assets

| Asset | Sensitivity | Where it lives |
|-------|-------------|----------------|
| Merchant funds (SPL tokens) | Critical | Escrow PDAs, merchant wallets |
| Merchant API keys | High | `merchants` table (hashed), client env |
| Merchant JWTs | High | Client memory, client env |
| Payment intents (PII linkage) | Medium | `payment_intents` table |
| Directory aliases (email, phone → wallet) | Medium | `aliases` table |
| On-chain upgrade authority | Critical | Hardware wallet (prod), dev keypair (test) |
| Database credentials | High | Env vars / secret manager |
| Webhook signing secrets | High | `webhooks` table, merchant servers |
| Audit logs | Medium | `audit_log` table (immutable) |
| Fraud rules & thresholds | Medium | `fraud_rules` table |

## 4. STRIDE Threats per Boundary

### B1: Public → API Gateway

| Threat | STRIDE | Mitigation | Residual Risk |
|--------|--------|------------|---------------|
| Merchant API key leak via logs/errors | I (Info Disclosure) | Keys hashed at rest (argon2/sha256); redaction middleware; error responses strip headers | Low — depends on merchant hygiene |
| Credential stuffing on login | S (Spoofing) | Rate limiting (Redis-backed), JWT short expiry, merchant_id+wallet binding | Low |
| Replay of signed requests | T (Tampering) | Timestamp + nonce on signed requests (future), TLS 1.3 required | Medium — replay window exists |
| DoS via flooding `POST /v1/payments` | D (DoS) | Per-API-key rate limits, global throttle, CloudFlare in front | Medium — sophisticated botnets |
| Privilege escalation across merchants | E (EoP) | Merchant scoping in every query; tests in `api-gateway/tests/auth_tests.rs` | Low — covered by tests |
| Missing authz on webhook endpoints | E | All `/v1/webhooks/*` gated by API key middleware | Low |
| Sensitive data in query params | I | POST-only for sensitive ops, no tokens in URL | Low |
| Repudiation of payment creation | R | Immutable `audit_log` table (Phase 5) | Low |

### B2: API Gateway → Routing Engine

| Threat | STRIDE | Mitigation | Residual Risk |
|--------|--------|------------|---------------|
| MITM on internal network | T, I | mTLS between services (planned); service mesh (Istio/Linkerd) in prod | Medium — currently plaintext in dev |
| Forged internal requests | S | Shared secret header + allowlisted source IPs; production: service account tokens | Medium |
| Routing engine accepting malformed intents | T | `serde` strict deserialization, field validation, bounded amounts | Low |

### B3: Routing Engine → Solana RPC

| Threat | STRIDE | Mitigation | Residual Risk |
|--------|--------|------------|---------------|
| Malicious/misconfigured RPC returning fake state | T, I | Use trusted RPC providers (Helius, Triton, QuickNode); multi-RPC quorum (future) | Medium |
| RPC rate limit exhaustion | D | Multiple RPC endpoints, exponential backoff, circuit breaker | Low |
| Transaction malleability | T | Recent blockhash enforced, signature verification by cluster | Low |
| Front-running (MEV) on swap | T | Slippage caps (`slippage_bps`), Jupiter route freshness window | Medium — inherent to DEX |
| Priority fee drain | D | Bounded `compute_unit_price`, dynamic fee oracle | Low |

### B4: Services → PostgreSQL

| Threat | STRIDE | Mitigation | Residual Risk |
|--------|--------|------------|---------------|
| SQL injection | T | `sqlx` compile-time prepared statements exclusively | Low |
| Unauthorized DB access | S, I | Network-isolated DB, rotated credentials, secret manager | Low |
| Data exfil via backup | I | Encrypted backups, access logging | Medium |
| Audit log tampering | T, R | Append-only trigger, no UPDATE/DELETE permission on `audit_log` | Low — enforce via role |

### B5: Services → Redis

| Threat | STRIDE | Mitigation | Residual Risk |
|--------|--------|------------|---------------|
| Cache poisoning | T | Internal network only; ACL users; no untrusted writes | Low |
| Rate limit bypass | E | TTL + atomic INCR; distributed lock for critical ops | Low |

### B6: Admin → Services

| Threat | STRIDE | Mitigation | Residual Risk |
|--------|--------|------------|---------------|
| Compromised deploy pipeline | T, E | GitHub OIDC to cloud; branch protection; required reviews; signed tags | Medium |
| Stolen operator laptop | S, I | Hardware keys for upgrade authority; no long-lived cloud creds | Medium |
| Insider misuse of admin endpoints | E, R | Admin actions logged in `audit_log`; 2-person rule for sensitive ops | Medium |

### B7: On-chain SolUPG Programs

| Threat | STRIDE | Mitigation | Residual Risk |
|--------|--------|------------|---------------|
| Account confusion (wrong program_id) | S | Anchor account constraint checks; discriminator verification | Low |
| Integer overflow in fee/amount math | T | `checked_add`/`checked_sub`/`checked_mul` throughout | Low — covered by tests |
| Reentrancy via CPI | T | Solana's no-reentrancy model; no self-CPI | Low |
| Unauthorized close of PDAs | E | `has_one` / `close = authority` constraints | Low |
| Incorrect PDA derivation | T | Helper `solupg-common/pda.rs` with unit tests | Low |
| Fee splitter rounding exploit | T | Remainder always credited to canonical recipient; sum invariant checked | Low |
| Escrow authority override | E | `authority` field immutable after init | Low |
| Swap slippage bypass | T | Minimum output enforced on-chain before transfer | Low |
| Upgrade authority theft | E | Hardware wallet + multisig for prod; optional freeze post-launch | Medium |

## 5. Attacker Profiles

| # | Profile | Capability | Motivation | Primary threats |
|---|---------|-----------|------------|-----------------|
| A1 | Opportunistic bot | Scripted HTTP | Vulnerability scanning | B1 DoS, credential stuffing |
| A2 | Malicious merchant | Valid API key | Steal other merchant funds | B1 EoP, B4 cross-merchant leaks |
| A3 | Compromised merchant server | Stolen API key | Drain merchant's own balance | B1 replay, webhook spoofing |
| A4 | Solana MEV searcher | Mempool observation | Front-run swaps | B3 MEV |
| A5 | Insider | Deploy access | Backdoor or halt | B6 pipeline, B7 upgrade |
| A6 | Nation-state | Supply chain, 0-days | Systemic compromise | All boundaries |

## 6. Assumptions

- The Solana network's consensus and cryptographic guarantees hold.
- Solana RPC providers used in production are trusted or verified via quorum.
- Hardware wallets for upgrade authority are not compromised.
- TLS certificates are valid and rotated.
- Host OS on service nodes is patched and hardened.

## 7. Known Gaps (Phase 6 Open Items)

1. **mTLS between internal services** — currently plaintext in dev; must ship before mainnet soft-launch.
2. **Multi-RPC quorum** — single RPC is a soft SPOF today.
3. **Hardware-backed upgrade authority** — needs documented ceremony before mainnet.
4. **Third-party audit** — scheduled for Phase 6 per `docs/security/audit-scope.md`.
5. **Formal key management policy** — see runbook.
6. **SBOM generation** — `cargo auditable` + `syft` to be added to CI after Phase 6 basics land.

## 8. Review Cadence

- Revisit this document before every mainnet release.
- Re-run STRIDE per boundary if a new service/endpoint is added.
- Document changes in `CHANGELOG.md` under a "Security" sub-heading.

## References

- [`docs/security/audit-scope.md`](./audit-scope.md) — external audit scope
- [`docs/phase-6-testing-deployment/runbook.md`](../phase-6-testing-deployment/runbook.md) — incident response
- [`docs/architecture/overview.md`](../architecture/overview.md) — system architecture
- [OWASP ASVS v4.0](https://owasp.org/www-project-application-security-verification-standard/)
- [Solana Security Best Practices](https://book.anchor-lang.com/anchor_in_depth/security.html)
