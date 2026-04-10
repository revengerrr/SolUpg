# SolUPG External Security Audit Scope

> **Status**: Draft — pending auditor engagement
> **Target engagement window**: Phase 6 (pre-mainnet)
> **Owner**: SolUPG security working group

---

## Purpose

Scope document for a third-party security audit of SolUPG ahead of Solana mainnet-beta deployment. This document is intended to be shared with prospective auditors during the quote/RFP process.

## Recommended Auditors

- [OtterSec](https://osec.io/) — Anchor-native, extensive Solana experience.
- [Neodyme](https://neodyme.io/) — Solana program audits, fuzzing, formal methods.
- [Halborn](https://halborn.com/) — multi-chain, mature audit workflow.
- [Zellic](https://www.zellic.io/) — Solana + zk, strong economics/incentive review.

Recommend gathering at least two quotes.

## In-Scope Components

### 1. On-chain Anchor Programs (primary focus)

All four programs and their direct dependencies:

| Program | Path | LoC (approx) | Priority |
|---------|------|--------------|----------|
| `solupg-payment` | `programs/solupg-payment/` | TBD | Critical |
| `solupg-escrow` | `programs/solupg-escrow/` | TBD | Critical |
| `solupg-splitter` | `programs/solupg-splitter/` | TBD | High |
| `solupg-swap` | `programs/solupg-swap/` | TBD | High |

Areas of particular concern:
- Account constraint completeness (`#[account(...)]` attributes).
- PDA derivation correctness (`solupg-common/src/pda.rs`).
- Integer arithmetic (overflow/underflow, rounding in splitter).
- Authority checks on close/update/release paths.
- CPI into SPL Token and Jupiter — correct account passing and signer seeds.
- Slippage and min-output enforcement on swap paths.
- Escrow state machine transitions (create → release / cancel / dispute).
- Reentrancy considerations even though Solana prevents direct self-CPI.
- Economic attacks: fee splitter rounding, dust accumulation, MEV on swap.

### 2. Off-chain Services (secondary focus — critical paths only)

Auditors should spot-check these critical paths rather than full coverage:

- **`api-gateway`** — authentication, authorization, rate limiting, input validation.
  - `services/api-gateway/src/auth.rs`
  - `services/api-gateway/src/middleware.rs`
  - `services/api-gateway/src/routes/payments.rs`
  - `services/api-gateway/src/routes/escrows.rs`
- **`routing-engine`** — transaction building, signing key handling, RPC interaction.
  - `services/routing-engine/src/anchor_ix.rs`
  - `services/routing-engine/src/direct_pay.rs`, `escrow.rs`, `split_pay.rs`, `swap_pay.rs`
  - `services/routing-engine/src/fee_calculator.rs`
- **`directory-service`** — OTP verification, alias → wallet binding.
  - `services/directory-service/src/services/`
- **`monitoring`** — fraud rule engine, sanctions screening.
  - `services/monitoring/src/fraud.rs`
  - `services/monitoring/src/audit.rs`

### 3. Client SDK (tertiary)

- `sdk/typescript/src/client.ts` — request construction, API key handling, error surface.

## Out of Scope (for this engagement)

- Third-party dependencies (Anchor, Solana SDK, SPL programs, Jupiter) beyond integration correctness.
- Infrastructure (Kubernetes, CloudFlare, cloud provider). Covered by a separate cloud security review.
- Frontend dashboards (not yet implemented).
- Social engineering / physical security.
- Penetration testing (tracked separately; may be same vendor).

## Audit Deliverables Expected

1. **Written report** with:
   - Executive summary.
   - Methodology and tools used (manual review, fuzzing, symbolic execution, etc.).
   - Findings classified by severity (Critical / High / Medium / Low / Informational).
   - Reproduction steps and recommended remediations per finding.
2. **Fix review pass** — after we remediate, auditor confirms fixes.
3. **Public report** (redacted as needed) — published in `docs/security/audits/` after all critical findings resolved.

## Severity Definitions

| Severity | Definition | Target SLA to fix |
|----------|------------|-------------------|
| Critical | Direct loss of funds or full program takeover | Must fix before mainnet |
| High | Significant fund risk or privilege escalation under realistic conditions | Must fix before mainnet |
| Medium | Limited fund risk or DoS requiring privileged position | Fix before mainnet-beta GA |
| Low | Minor issue, defense-in-depth | Fix within 2 release cycles |
| Informational | Code quality, best practices | Best effort |

## Pre-Audit Checklist (SolUPG responsibilities)

- [ ] Freeze feature development on audited components.
- [ ] Tag the audit commit (e.g., `audit/2026-q2`).
- [ ] Supply auditor with:
  - Architecture overview (`docs/architecture/overview.md`).
  - Threat model (`docs/security/threat-model.md`).
  - This scope document.
  - Test coverage report (from CI).
  - Build instructions (README + scripts/).
  - Known issues / self-identified risks.
- [ ] Dedicated Slack channel or shared repo for Q&A during the engagement.
- [ ] Response SLA: 1 business day to auditor questions.

## Post-Audit Checklist

- [ ] Track every finding in a dedicated issue tracker project.
- [ ] Remediate all Critical + High findings.
- [ ] Write reproduction tests for each exploitable finding and add to CI.
- [ ] Fix review pass signed off by auditor.
- [ ] Publish public report.
- [ ] Update threat model with any new attack vectors discovered.

## Suggested Audit Commit Pin

A specific commit hash will be selected after feature-freeze. Candidate criteria:
- All Tier 1 + Tier 2 Phase 6 work merged.
- All CI jobs green.
- Coverage ≥ 90% on programs, ≥ 80% on critical service paths.
- No `TODO`, `FIXME`, or `unimplemented!()` in in-scope code.

## Budget Planning

Typical Solana program audits (4 programs, ~2–4k LoC total) range **$50k–$150k** depending on auditor, depth, and fix-review inclusion. Budget accordingly, and plan for a potential second audit pass if major changes are required post-audit.

## Contact

Audit coordination: TBD (to be assigned before outreach).
