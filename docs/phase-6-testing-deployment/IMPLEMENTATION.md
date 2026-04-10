# Phase 6 — Implementation Notes

> **Companion to**: [`README.md`](./README.md) — Phase 6 overview & deliverables checklist
> **Status**: Tier 1 (Foundations) + Tier 2 (Testing Expansion) shipped
> **Remaining**: External-action items (third-party audit, mainnet deploy, production load test, public launch)

---

## What shipped in this pass

### Tier 1 — Foundations

| # | Area | Deliverable | Files |
|---|------|-------------|-------|
| 1 | Anchor config | Added `[programs.devnet]` and `[programs.mainnet]` sections | `Anchor.toml` |
| 2 | Security docs | Threat model, audit scope, incident runbook, this doc | `docs/security/*.md`, `docs/phase-6-testing-deployment/*.md` |
| 3 | Security tooling | `cargo-deny`, `cargo-audit`, Dependabot configs | `deny.toml`, `.cargo/audit.toml`, `.github/dependabot.yml` |
| 4 | Containerization | Shared multi-stage Dockerfile + .dockerignore | `services/Dockerfile`, `services/.dockerignore` |
| 5 | Compose stack | Full-stack `docker-compose.yml` with all 5 services | `services/docker-compose.yml` |
| 6 | Deploy scripts | Devnet / mainnet deploy, stack runner, DB seeding | `scripts/*.sh`, `scripts/README.md` |
| 7 | CI/CD | 5 GitHub Actions workflows (pinned toolchain) | `.github/workflows/*.yml` |

### Tier 2 — Testing Expansion

| # | Area | Deliverable | Files |
|---|------|-------------|-------|
| 8 | Coverage | `cargo-llvm-cov` in CI, Codecov config, 90% target | `codecov.yml`, rust.yml step |
| 9 | Unit tests | directory-service unit tests | `services/directory-service/tests/*.rs` |
| 10 | SDK tests | TypeScript SDK smoke tests (vitest) | `sdk/typescript/tests/*.ts`, `package.json` |
| 11 | E2E tests | New `integration-tests` crate with 4 flows | `services/integration-tests/**` |
| 12 | Load tests | k6 scenarios for API Gateway | `load-tests/**` |

### Tier 3 — Pre-launch Docs

- `README.md` Phase 6 row updated.
- `CHANGELOG.md` Unreleased section appended.
- Auto-memory (`project_phase6_progress.md`) updated.

---

## How to run each piece

### Local static validation (no compile)

```bash
# YAML syntax sanity
yamllint .github/workflows/*.yml services/docker-compose.yml

# Shell scripts
shellcheck scripts/*.sh

# Compose validation
cd services && docker compose config
```

### Local compose stack (requires Docker Desktop with Linux containers)

```bash
cd services
docker compose build
docker compose up -d
docker compose ps
# Health probes:
curl http://localhost:3000/health   # routing-engine
curl http://localhost:3001/health   # directory-service
curl http://localhost:3002/health   # api-gateway
curl http://localhost:3003/health   # clearing-engine
curl http://localhost:3004/health   # monitoring
```

### CI verification (push to GitHub)

Push a branch; GitHub Actions will run:

- `rust.yml` → fmt, clippy, build, test, llvm-cov upload.
- `anchor.yml` → anchor build + the 10 existing TS tests.
- `sdk.yml` → TS SDK build + vitest.
- `security.yml` → cargo-audit + cargo-deny + npm audit.
- `docker.yml` → matrix build all 5 service images.

### E2E integration tests (requires compose stack up)

```bash
cd services
docker compose up -d
cargo test -p integration-tests -- --ignored
```

### Load tests (k6)

```bash
# Install k6: https://k6.io/docs/get-started/installation/
cd load-tests

# Single scenario
k6 run scenarios/create-payment.js

# With overrides
BASE_URL=https://staging.api.solupg.io \
API_KEY=solupg_live_xxx \
k6 run scenarios/mixed-workflow.js
```

### Deploy to devnet (first-time setup)

```bash
# Ensure Solana CLI is configured and funded
solana config set --url devnet
solana airdrop 5

# From repo root
./scripts/deploy-devnet.sh
# Follow the prompts; update Anchor.toml [programs.devnet] with output IDs
```

### Deploy to mainnet-beta (external action, not run in CI)

```bash
# Requires: funded upgrade authority on hardware wallet
./scripts/deploy-mainnet-beta.sh
# Script prompts for confirmation before each program
```

---

## External-action items (NOT covered by this session)

The Phase 6 README deliverables checklist has items that require action outside of this repo:

| # | Item | Status | Who/what is needed |
|---|------|--------|---------------------|
| 1 | >90% test coverage proven | **CI will report** | Run CI after merge; address uncovered code |
| 2 | Load test meeting 1k TPS | **Framework shipped, target unproven** | Dedicated load runner (k6 cloud / self-hosted VM) + staging infra |
| 3 | Third-party security audit | **Scope doc shipped** | Engage auditor per `docs/security/audit-scope.md` |
| 4 | All audit findings resolved | — | Post-audit fix cycle |
| 5 | Mainnet-beta deployment | **Script shipped** | Funded keypair, security ceremony, run `deploy-mainnet-beta.sh` |
| 6 | Monitoring verified in prod | **Service + runbook shipped** | Connect monitoring to real alert channels, verify under load |
| 7 | Incident response runbook | **Shipped** | `docs/phase-6-testing-deployment/runbook.md` |
| 8 | Phase 6 completion documentation | **This file** | — |
| 9 | Public launch announcement | — | Marketing/comms work, out of repo scope |

---

## Known gaps & follow-ups

1. **Kubernetes manifests**: not shipped this session. Compose is sufficient for local/staging; k8s is a post-launch concern.
2. **Kafka**: not wired up in compose. Monitoring/clearing use direct DB for now; Kafka can be added later.
3. **Prometheus/Grafana dashboards**: monitoring service emits metrics already — dashboard JSON can follow once production Grafana exists.
4. **SBOM**: `cargo auditable` and `syft` not integrated; easy follow-up.
5. **mTLS between services**: noted in threat model as must-ship before mainnet soft-launch.
6. **Fuzz testing** (Trident/Honggfuzz): not yet set up; would strengthen pre-audit confidence.

---

## Windows dev notes

The known Windows GNU-linker bug (`DllEntryPoint` unresolved) blocks full workspace builds locally. All compile-heavy verification runs on Linux CI or in the Docker build. File authoring on Windows is unaffected.

If you need to run integration tests or coverage locally on Windows, use WSL2:

```bash
# From WSL2 Ubuntu
cd /mnt/d/SolUpg/services
cargo test                              # unit tests
docker compose up -d                    # requires Docker Desktop WSL2 integration
cargo test -p integration-tests -- --ignored
```

---

## Verification matrix

| Check | Local (Win) | Local (WSL/Linux) | CI |
|-------|-------------|-------------------|----|
| YAML lint | yes | yes | yes |
| Shell lint | git bash | yes | yes |
| `docker compose config` | yes | yes | yes |
| `docker compose build` | Desktop | yes | yes (docker.yml) |
| `cargo build` workspace | blocked | yes | yes (rust.yml) |
| `cargo test` unit | blocked | yes | yes (rust.yml) |
| `cargo llvm-cov` | blocked | yes | yes (rust.yml) |
| `cargo-audit` | yes | yes | yes (security.yml) |
| `cargo-deny check` | yes | yes | yes (security.yml) |
| SDK `npm test` | yes | yes | yes (sdk.yml) |
| `anchor build` | yes | yes | yes (anchor.yml) |
| `anchor test` | yes | yes | yes (anchor.yml) |
| E2E integration tests | blocked | yes | manual trigger |
| k6 load tests | installed | yes | not run (manual) |
