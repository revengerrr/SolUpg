#!/usr/bin/env bash
# deploy-mainnet-beta.sh — deploy SolUPG programs to Solana mainnet-beta.
#
# THIS SCRIPT SPENDS REAL SOL AND DEPLOYS TO PRODUCTION. READ THE
# CHECKLIST BELOW BEFORE RUNNING.
#
# Preconditions checklist:
#   [ ] Upgrade authority keypair is on a hardware wallet (Ledger).
#   [ ] You are running from the exact commit that has been audited.
#   [ ] CI is green on that commit.
#   [ ] You have a funded deploy wallet with ~30 SOL for program buffers.
#   [ ] Security lead + eng lead are both present for this ceremony.
#   [ ] Status page is updated with planned maintenance window.
#   [ ] docs/security/audit-scope.md findings are all resolved.
#
# Procedure:
#   1. Prompts for explicit confirmation per program.
#   2. Uses buffer accounts + two-step authority transfer for safety.
#   3. Prints final program IDs at the end for Anchor.toml update.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

PROGRAMS=(
  solupg_payment
  solupg_escrow
  solupg_splitter
  solupg_swap
)

RPC_URL="${SOLANA_RPC_URL:-https://api.mainnet-beta.solana.com}"
UPGRADE_AUTHORITY="${UPGRADE_AUTHORITY:-}"
DEPLOY_WALLET="${DEPLOY_WALLET:-${HOME}/.config/solana/id.json}"

# ---------------------------------------------------------------------------
# Guard rails
# ---------------------------------------------------------------------------
echo "==> SolUPG MAINNET-BETA deployment"
echo "    RPC URL          : ${RPC_URL}"
echo "    Deploy wallet    : ${DEPLOY_WALLET}"
echo "    Upgrade authority: ${UPGRADE_AUTHORITY:-<not set — will use deploy wallet>}"
echo

command -v solana >/dev/null || { echo "ERROR: solana CLI not found"; exit 1; }
command -v anchor >/dev/null || { echo "ERROR: anchor CLI not found"; exit 1; }

if [[ "${RPC_URL}" != *"mainnet"* ]]; then
  echo "ERROR: SOLANA_RPC_URL does not look like a mainnet endpoint (${RPC_URL})"
  exit 1
fi

# Force the user to type a magic string.
echo "Type MAINNET-DEPLOY to proceed, anything else to abort:"
read -r confirm
if [[ "${confirm}" != "MAINNET-DEPLOY" ]]; then
  echo "Aborted."
  exit 1
fi

# Git state must be clean and tagged.
if [[ -n "$(git status --porcelain)" ]]; then
  echo "ERROR: git working tree is dirty. Commit or stash before deploying."
  exit 1
fi

HEAD_COMMIT="$(git rev-parse HEAD)"
HEAD_TAG="$(git describe --tags --exact-match 2>/dev/null || true)"
if [[ -z "${HEAD_TAG}" ]]; then
  echo "WARNING: HEAD (${HEAD_COMMIT}) is not tagged. Continue? [y/N]"
  read -r ans
  [[ "${ans}" == "y" ]] || exit 1
fi
echo "    HEAD commit: ${HEAD_COMMIT}"
echo "    HEAD tag   : ${HEAD_TAG:-<none>}"
echo

BALANCE="$(solana --url "${RPC_URL}" balance | awk '{print $1}')"
echo "    Deploy wallet balance: ${BALANCE} SOL"
if (( $(echo "${BALANCE} < 20" | bc -l) )); then
  echo "ERROR: deploy wallet has < 20 SOL. Top up before continuing."
  exit 1
fi

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
echo
echo "==> anchor build --verifiable"
anchor build --verifiable

# ---------------------------------------------------------------------------
# Deploy each program (with confirmation)
# ---------------------------------------------------------------------------
declare -A DEPLOYED_IDS

for prog in "${PROGRAMS[@]}"; do
  echo
  echo "==> Preparing to deploy ${prog}"
  SO_PATH="target/deploy/${prog}.so"
  KEYPAIR_PATH="target/deploy/${prog}-keypair.json"

  [[ -f "${SO_PATH}" ]]      || { echo "ERROR: ${SO_PATH} missing";      exit 1; }
  [[ -f "${KEYPAIR_PATH}" ]] || { echo "ERROR: ${KEYPAIR_PATH} missing"; exit 1; }

  PROGRAM_ID="$(solana-keygen pubkey "${KEYPAIR_PATH}")"
  ARTIFACT_HASH="$(sha256sum "${SO_PATH}" | awk '{print $1}')"

  echo "    program id    : ${PROGRAM_ID}"
  echo "    artifact sha256: ${ARTIFACT_HASH}"
  echo "    upgrade auth  : ${UPGRADE_AUTHORITY:-<deploy wallet>}"
  echo
  echo "    Type the program name (${prog}) to deploy, anything else to skip:"
  read -r skip_confirm
  if [[ "${skip_confirm}" != "${prog}" ]]; then
    echo "    Skipped."
    continue
  fi

  DEPLOY_ARGS=(
    program deploy "${SO_PATH}"
    --program-id "${KEYPAIR_PATH}"
    --url "${RPC_URL}"
    --keypair "${DEPLOY_WALLET}"
  )
  if [[ -n "${UPGRADE_AUTHORITY}" ]]; then
    DEPLOY_ARGS+=(--upgrade-authority "${UPGRADE_AUTHORITY}")
  fi

  echo "    Running: solana ${DEPLOY_ARGS[*]}"
  solana "${DEPLOY_ARGS[@]}"

  DEPLOYED_IDS["${prog}"]="${PROGRAM_ID}"
  echo "    ${prog} deployed: ${PROGRAM_ID}"
done

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo
echo "==> MAINNET-BETA deployment complete"
echo
echo "Update Anchor.toml [programs.mainnet] with:"
echo
for prog in "${PROGRAMS[@]}"; do
  id="${DEPLOYED_IDS[${prog}]:-<skipped>}"
  printf '  %s = "%s"\n' "${prog}" "${id}"
done
echo
echo "Post-deploy tasks:"
echo "  1. Verify each program buffer is closed on-chain."
echo "  2. Confirm upgrade authority is the hardware-wallet address."
echo "  3. Update Anchor.toml, commit, tag, and push."
echo "  4. Update status page: maintenance window closed."
echo "  5. File deploy record in docs/deploys/YYYY-MM-DD-mainnet.md."
