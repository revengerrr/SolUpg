#!/usr/bin/env bash
# deploy-devnet.sh — build and deploy SolUPG on-chain programs to Solana devnet.
#
# Prerequisites:
#   - solana CLI configured with a funded devnet keypair
#       solana config set --url devnet
#       solana airdrop 5
#   - anchor CLI on PATH (0.29.0)
#
# What it does:
#   1. Runs `anchor build` for all 4 programs.
#   2. Runs `anchor deploy --provider.cluster devnet` for each program.
#   3. Prints the resulting program IDs so you can update Anchor.toml.
#
# What it does NOT do:
#   - Rewrite Anchor.toml (do that manually after verifying IDs).
#   - Write program keypair files (Anchor handles that under target/deploy/).

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

echo "==> SolUPG devnet deployment"
echo "    Repo: ${REPO_ROOT}"
echo "    Programs: ${PROGRAMS[*]}"
echo

# ---------------------------------------------------------------------------
# Sanity checks
# ---------------------------------------------------------------------------
command -v solana >/dev/null || { echo "ERROR: solana CLI not found"; exit 1; }
command -v anchor >/dev/null || { echo "ERROR: anchor CLI not found"; exit 1; }

CURRENT_CLUSTER="$(solana config get | awk '/RPC URL/ {print $3}')"
echo "    Current RPC: ${CURRENT_CLUSTER}"
if [[ "${CURRENT_CLUSTER}" != *"devnet"* ]]; then
  echo "    WARNING: current RPC is not devnet. Continue? [y/N]"
  read -r ans
  [[ "${ans}" == "y" ]] || exit 1
fi

BALANCE="$(solana balance | awk '{print $1}')"
echo "    Wallet balance: ${BALANCE} SOL"
if (( $(echo "${BALANCE} < 2" | bc -l) )); then
  echo "    WARNING: low balance; run 'solana airdrop 5' first."
fi
echo

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
echo "==> anchor build"
anchor build

# ---------------------------------------------------------------------------
# Deploy each program
# ---------------------------------------------------------------------------
declare -A DEPLOYED_IDS

for prog in "${PROGRAMS[@]}"; do
  echo
  echo "==> Deploying ${prog}"

  SO_PATH="target/deploy/${prog}.so"
  if [[ ! -f "${SO_PATH}" ]]; then
    echo "ERROR: ${SO_PATH} not found after build"
    exit 1
  fi

  KEYPAIR_PATH="target/deploy/${prog}-keypair.json"
  if [[ ! -f "${KEYPAIR_PATH}" ]]; then
    echo "ERROR: ${KEYPAIR_PATH} not found"
    exit 1
  fi

  PROGRAM_ID="$(solana-keygen pubkey "${KEYPAIR_PATH}")"
  DEPLOYED_IDS["${prog}"]="${PROGRAM_ID}"

  anchor deploy \
    --provider.cluster devnet \
    --program-name "${prog}" \
    --program-keypair "${KEYPAIR_PATH}"

  echo "    ${prog} -> ${PROGRAM_ID}"
done

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo
echo "==> Devnet deployment complete"
echo
echo "Update Anchor.toml [programs.devnet] with:"
echo
for prog in "${PROGRAMS[@]}"; do
  printf '  %s = "%s"\n' "${prog}" "${DEPLOYED_IDS[${prog}]}"
done
echo
echo "Then commit Anchor.toml and push."
