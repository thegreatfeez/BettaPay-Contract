#!/usr/bin/env bash
# BettaPay — Stellar Testnet Deployment Script
# Run from inside BettaPay-Contract/
set -euo pipefail

if ! command -v soroban >/dev/null 2>&1; then
  echo "soroban CLI not found in PATH"
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

: "${SOROBAN_RPC_URL:=https://soroban-testnet.stellar.org}"
: "${SOROBAN_NETWORK_PASSPHRASE:=Test SDF Network ; September 2015}"
: "${BETTAPAY_IDENTITY:=bettapay-admin}"

if ! soroban keys address "$BETTAPAY_IDENTITY" >/dev/null 2>&1; then
  soroban keys generate "$BETTAPAY_IDENTITY" --fund >/dev/null
fi

ADMIN_ADDRESS="$(soroban keys address "$BETTAPAY_IDENTITY")"

curl --silent --show-error "https://friendbot.stellar.org/?addr=${ADMIN_ADDRESS}" >/dev/null || true

# Build both WASM binaries
cargo build --target wasm32-unknown-unknown --release \
  -p settlement_contract \
  -p governance_contract

SETTLEMENT_WASM="$ROOT_DIR/target/wasm32-unknown-unknown/release/settlement_contract.wasm"
GOVERNANCE_WASM="$ROOT_DIR/target/wasm32-unknown-unknown/release/governance_contract.wasm"

SETTLEMENT_ID="$(
  soroban contract deploy \
    --wasm "$SETTLEMENT_WASM" \
    --source-account "$BETTAPAY_IDENTITY" \
    --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$SOROBAN_NETWORK_PASSPHRASE"
)"

GOVERNANCE_ID="$(
  soroban contract deploy \
    --wasm "$GOVERNANCE_WASM" \
    --source-account "$BETTAPAY_IDENTITY" \
    --rpc-url "$SOROBAN_RPC_URL" \
    --network-passphrase "$SOROBAN_NETWORK_PASSPHRASE"
)"

# Initialize both contracts with admin
soroban contract invoke \
  --id "$SETTLEMENT_ID" \
  --source-account "$BETTAPAY_IDENTITY" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$SOROBAN_NETWORK_PASSPHRASE" \
  -- \
  init --admin "$ADMIN_ADDRESS"

soroban contract invoke \
  --id "$GOVERNANCE_ID" \
  --source-account "$BETTAPAY_IDENTITY" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$SOROBAN_NETWORK_PASSPHRASE" \
  -- \
  init --admin "$ADMIN_ADDRESS"

echo ""
echo "========================================"
echo "  BettaPay Testnet Deployment Complete"
echo "========================================"
echo "  Identity:             $BETTAPAY_IDENTITY"
echo "  Admin address:        $ADMIN_ADDRESS"
echo "  Settlement contract:  $SETTLEMENT_ID"
echo "  Governance contract:  $GOVERNANCE_ID"
echo "========================================"
echo ""
echo "Next: update SETTLEMENT_CONTRACT_ID and GOVERNANCE_CONTRACT_ID"
echo "in BettaPay-Frontend and BettaPay-Backend .env files."
