# Changelog

All notable changes to SolUPG will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added — Phase 6 (Tier 1 + Tier 2)
- **CI/CD**: GitHub Actions workflows pinned to Rust 1.79.0, Solana 2.2.1,
  Anchor 0.29.0, Node 20:
  - `.github/workflows/rust.yml` — fmt + clippy + test + `cargo llvm-cov` + e2e
  - `.github/workflows/anchor.yml` — anchor build/test against test-validator
  - `.github/workflows/sdk.yml` — TypeScript SDK build + vitest
  - `.github/workflows/security.yml` — `cargo-audit`, `cargo-deny`, `npm audit`
  - `.github/workflows/docker.yml` — matrix build of 5 services, push to GHCR
- **Dependabot**: `.github/dependabot.yml` covering cargo, npm, github-actions,
  and docker ecosystems with grouped updates.
- **Security tooling**:
  - `deny.toml` — license allowlist, advisory DB, source restrictions
  - `.cargo/audit.toml` — cargo-audit scaffolding
- **Containerization**:
  - `services/Dockerfile` — shared multi-stage build (cargo-chef) with
    `SERVICE` build arg; single image per service
  - `services/.dockerignore` — excludes target/, node_modules/, test-ledger/
  - `services/docker-compose.yml` — full stack (postgres, redis, and all 5
    Rust services on ports 3000–3004) with healthchecks + `solupg-net`
- **Deployment & ops scripts** (`scripts/`):
  - `deploy-devnet.sh`, `deploy-mainnet-beta.sh` (multi-confirmation guard)
  - `run-all-services.sh` — compose lifecycle wrapper
  - `seed-db.sh` — migrations + fixtures
- **Security & ops documentation** (`docs/`):
  - `security/threat-model.md` — STRIDE per trust boundary, attacker profiles,
    residual risks
  - `security/audit-scope.md` — external audit scope, recommended auditors
    (OtterSec / Neodyme / Halborn / Zellic), budget, pre/post checklists
  - `phase-6-testing-deployment/runbook.md` — SEV tiers + 6 incident playbooks
  - `phase-6-testing-deployment/IMPLEMENTATION.md` — Phase 6 narrative,
    external-action items, verification matrix
- **Testing expansion**:
  - `codecov.yml` — 90/80/70 coverage targets per component
  - `services/directory-service/tests/handlers_test.rs` + `db_test.rs`
  - `services/directory-service/src/lib.rs` + `[lib]` target for test imports
  - `sdk/typescript/tests/client.test.ts` — fetch-mocked smoke tests (11 cases)
  - `sdk/typescript/tests/types.test.ts` — contract tests for SDK types
  - `sdk/typescript/vitest.config.ts` — v8 coverage, 10s test timeout
  - `services/integration-tests/` — new workspace member with `#[ignore]`'d
    E2E flows: `payment_flow`, `escrow_flow`, `splitter_flow`, `swap_flow`
  - `load-tests/` — k6 scenarios: `create-payment`, `get-status`,
    `rate-limit`, `mixed-workflow` + shared `lib/auth.js` + `lib/fixtures.js`
- **Anchor cluster config**: `Anchor.toml` now has `[programs.devnet]` and
  `[programs.mainnet]` sections alongside the existing `[programs.localnet]`.

### Notes
- External actions still required (explicitly out of scope this phase):
  third-party audit engagement, real mainnet program deploy, 1k TPS proof on
  prod-like infra, public launch. See
  `docs/phase-6-testing-deployment/IMPLEMENTATION.md`.
- Windows GNU linker bug blocks full `cargo build` of the services workspace
  on Win11 hosts. All compile-heavy verification runs on Linux CI.

### Added
- Initial project structure and documentation
- Phase 1-6 development roadmap documentation
- Architecture overview documentation
- Apache 2.0 license
- Contributing guidelines
- **Documentation System**
  - `CHANGELOG.md`: Version tracking
  - `docs/DOCUMENTATION_GUIDE.md`: Documentation maintenance guide
  - `docs/development/CURRENT_STATUS.md`: Live development status
  - `.github/PULL_REQUEST_TEMPLATE.md`: PR template with doc checklist
  - `.github/ISSUE_TEMPLATE/bug_report.md`: Bug report template
  - `.github/ISSUE_TEMPLATE/feature_request.md`: Feature request template
- Updated `CONTRIBUTING.md` with mandatory documentation requirements
- Updated `README.md` with documentation links

### Changed
- Renamed project from UPG to SolUPG (Solana Universal Payment Gateway)

---

## [0.1.0] - 2026-03-29

### Added
- **Project Setup**
  - Initial repository structure
  - README with project overview, architecture diagram, and roadmap
  - Documentation structure for all 6 development phases
  - Folder structure for programs, services, SDK, and tests

- **Documentation**
  - `docs/architecture/overview.md`: System architecture deep-dive
  - `docs/phase-1-onchain-programs/`: On-chain program specifications
  - `docs/phase-2-routing-engine/`: Routing engine & directory service specs
  - `docs/phase-3-api-gateway/`: API gateway & SDK specs
  - `docs/phase-4-clearing-reconciliation/`: Clearing & reconciliation specs
  - `docs/phase-5-compliance-monitoring/`: Compliance & monitoring specs
  - `docs/phase-6-testing-deployment/`: Testing & deployment specs

- **Open Source**
  - Apache 2.0 License
  - CONTRIBUTING.md with code standards and PR process
  - .gitignore for Rust, Node.js, and Anchor projects

---

## Version History

| Version | Date | Description |
|---------|------|-------------|
| 0.1.0 | 2026-03-29 | Initial project setup with documentation |

---

## Upcoming

### Phase 1 (Next)
- [ ] `solupg-payment`: Core payment program
- [ ] `solupg-escrow`: Escrow program
- [ ] `solupg-splitter`: Fee splitting program
- [ ] `solupg-swap`: Token swap integration

See [Phase 1 Documentation](./docs/phase-1-onchain-programs/README.md) for details.
