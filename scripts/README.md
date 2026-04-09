# SolUPG Scripts

Operational helpers for SolUPG development, deployment, and local stack management. Scripts are written for `bash` (Linux, macOS, WSL, Git Bash).

## Inventory

| Script | Purpose | Safety |
|--------|---------|--------|
| `deploy-devnet.sh` | Build + deploy all 4 on-chain programs to Solana devnet | Non-destructive; uses your devnet wallet |
| `deploy-mainnet-beta.sh` | Deploy to Solana mainnet-beta with multi-step confirmation | **Destructive: spends real SOL; do not run casually** |
| `run-all-services.sh` | Wrapper around `docker compose` for the full stack | Non-destructive except `nuke` |
| `seed-db.sh` | Apply migrations + insert fixtures in local Postgres | `--reset` drops the DB |

## Usage

```bash
# Local stack
scripts/run-all-services.sh up
scripts/run-all-services.sh status
scripts/run-all-services.sh logs api-gateway
scripts/run-all-services.sh down

# Database
scripts/seed-db.sh
scripts/seed-db.sh --reset      # wipes and reapplies

# Devnet deploy (requires funded devnet wallet)
scripts/deploy-devnet.sh

# Mainnet-beta deploy (requires audit sign-off, hardware wallet, MAINNET-DEPLOY confirmation)
UPGRADE_AUTHORITY=<pubkey> ./scripts/deploy-mainnet-beta.sh
```

## Making scripts executable

On Linux / macOS / WSL after cloning:

```bash
chmod +x scripts/*.sh
```

On Windows (Git Bash or PowerShell with Git):

```powershell
git update-index --chmod=+x scripts/deploy-devnet.sh
git update-index --chmod=+x scripts/deploy-mainnet-beta.sh
git update-index --chmod=+x scripts/run-all-services.sh
git update-index --chmod=+x scripts/seed-db.sh
```

## CI usage

The scripts are safe to invoke from GitHub Actions runners. See:

- `.github/workflows/anchor.yml` — calls `anchor` commands directly (not these scripts).
- `.github/workflows/rust.yml` — uses `docker compose` via `run-all-services.sh up`.

## Related docs

- [`docs/phase-6-testing-deployment/IMPLEMENTATION.md`](../docs/phase-6-testing-deployment/IMPLEMENTATION.md)
- [`docs/phase-6-testing-deployment/runbook.md`](../docs/phase-6-testing-deployment/runbook.md)
- [`docs/security/audit-scope.md`](../docs/security/audit-scope.md)
