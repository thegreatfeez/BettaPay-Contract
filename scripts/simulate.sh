#!/usr/bin/env bash
set -euo pipefail

if ! command -v soroban >/dev/null 2>&1; then
  echo "soroban CLI not found in PATH"
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

: "${SOROBAN_RPC_URL:=https://soroban-testnet.stellar.org}"
: "${SOROBAN_NETWORK_PASSPHRASE:=Test SDF Network ; September 2015}"
: "${SOROBAN_SOURCE:=bettapay-sim}"

if ! soroban keys address "$SOROBAN_SOURCE" >/dev/null 2>&1; then
  soroban keys generate "$SOROBAN_SOURCE" --fund >/dev/null
fi

SOROBAN_SOURCE_ADDRESS="$(soroban keys address "$SOROBAN_SOURCE")"

curl --silent --show-error "https://friendbot.stellar.org/?addr=${SOROBAN_SOURCE_ADDRESS}" >/dev/null || true

cargo build --target wasm32-unknown-unknown --release -p settlement_contract -p governance_contract

SETTLEMENT_WASM="$ROOT_DIR/target/wasm32-unknown-unknown/release/settlement_contract.wasm"
GOVERNANCE_WASM="$ROOT_DIR/target/wasm32-unknown-unknown/release/governance_contract.wasm"

soroban contract deploy \
  --wasm "$SETTLEMENT_WASM" \
  --source-account "$SOROBAN_SOURCE" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$SOROBAN_NETWORK_PASSPHRASE" \
  >/tmp/bettapay_settlement_id.txt

soroban contract deploy \
  --wasm "$GOVERNANCE_WASM" \
  --source-account "$SOROBAN_SOURCE" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$SOROBAN_NETWORK_PASSPHRASE" \
  >/tmp/bettapay_governance_id.txt

SETTLEMENT_ID="$(tr -d '\n' </tmp/bettapay_settlement_id.txt)"
GOVERNANCE_ID="$(tr -d '\n' </tmp/bettapay_governance_id.txt)"

soroban contract invoke \
  --id "$SETTLEMENT_ID" \
  --source-account "$SOROBAN_SOURCE" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$SOROBAN_NETWORK_PASSPHRASE" \
  -- \
  init --admin "$SOROBAN_SOURCE_ADDRESS"

soroban contract invoke \
  --id "$GOVERNANCE_ID" \
  --source-account "$SOROBAN_SOURCE" \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$SOROBAN_NETWORK_PASSPHRASE" \
  -- \
  init --admin "$SOROBAN_SOURCE_ADDRESS"

echo "Simulation bootstrap complete."
echo "Source identity: $SOROBAN_SOURCE"
echo "Source address: $SOROBAN_SOURCE_ADDRESS"
echo "Settlement contract ID: $SETTLEMENT_ID"
echo "Governance contract ID: $GOVERNANCE_ID"
