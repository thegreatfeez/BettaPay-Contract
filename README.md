# BettaPay Contracts

Soroban smart contracts for the BettaPay payment infrastructure on Stellar.


## Structure

```
BettaPay-Contract/
├── Cargo.toml                  # Rust workspace root (both contracts)
├── settlement_contract/        # Merchant registration, fee splits, payment references
│   ├── Cargo.toml
│   └── src/lib.rs
├── governance_contract/        # Fee config, anchor registry, system params
│   ├── Cargo.toml
│   └── src/lib.rs
└── scripts/
    ├── deploy_testnet.sh       # Build + deploy both contracts + init admin
    └── simulate.sh             # Simulate contract calls locally
```

## Deployed Addresses (Testnet)

| Contract     | Address                                                  |
|-------------|----------------------------------------------------------|
| Settlement  | `CBGBGKJSUY7XYB6HWW4CVAU6MW2KD25FSF45E5KCP53TKUK374MBZNFB` |
| Governance  | `CDPFWUTIXF5BC6BKNDLSQOZSDQCXAJNZFCZWHBE2RRHANRN25T3ILPZ7` |
| Admin       | `GCCHHKNI7GRA5QWC7RCTT3OHO7SKAUMKQA6IBWEQEO2SXI3GF376UHDD` |

Network: `Test SDF Network ; September 2015`

## Quick Start

```bash
# Run all tests
cargo test

# Build WASM release binaries
cargo build --target wasm32-unknown-unknown --release

# Deploy to testnet (requires soroban CLI)
bash scripts/deploy_testnet.sh
```

## CLI Usage Examples

All examples below assume you have the [Soroban CLI](https://soroban.stellar.org/docs) installed and a funded testnet identity.

### Build

```bash
# Build all contracts (release WASM)
cargo build --target wasm32-unknown-unknown --release

# Build a specific contract
cargo build --target wasm32-unknown-unknown --release -p settlement_contract
cargo build --target wasm32-unknown-unknown --release -p governance_contract
```

### Test

```bash
# Run all tests
cargo test

# Run tests for a specific contract
cargo test -p settlement_contract
cargo test -p governance_contract

# Run a specific test by name
cargo test registers_merchant_and_persists_flag -p settlement_contract
```

### Deploy to Testnet

```bash
# One-command deployment (builds + deploys + initializes both contracts)
bash scripts/deploy_testnet.sh

# Or deploy step-by-step:
# 1. Generate and fund a key
soroban keys generate bettapay-admin --fund

# 2. Build WASM
cargo build --target wasm32-unknown-unknown --release

# 3. Deploy settlement contract
SETTLEMENT_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/settlement_contract.wasm \
  --source-account bettapay-admin \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015")

# 4. Deploy governance contract
GOVERNANCE_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/governance_contract.wasm \
  --source-account bettapay-admin \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015")

# 5. Initialize both contracts
ADMIN=$(soroban keys address bettapay-admin)

soroban contract invoke \
  --id "$SETTLEMENT_ID" \
  --source-account bettapay-admin \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- \
  init --admin "$ADMIN"

soroban contract invoke \
  --id "$GOVERNANCE_ID" \
  --source-account bettapay-admin \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" \
  -- \
  init --admin "$ADMIN"
```

### Invoke Settlement Contract

```bash
# Set vars (adjust IDs as needed)
SETTLEMENT_ID=CBGBGKJSUY7XYB6HWW4CVAU6MW2KD25FSF45E5KCP53TKUK374MBZNFB
ADMIN=GCCHHKNI7GRA5QWC7RCTT3OHO7SKAUMKQA6IBWEQEO2SXI3GF376UHDD
MERCHANT=GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
RPC=https://soroban-testnet.stellar.org
PASS="Test SDF Network ; September 2015"

# Check admin
soroban contract invoke --id "$SETTLEMENT_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  get_admin

# Register a merchant
soroban contract invoke --id "$SETTLEMENT_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  register_merchant --merchant "$MERCHANT"

# Check if merchant is registered
soroban contract invoke --id "$SETTLEMENT_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  is_merchant_registered --merchant "$MERCHANT"

# Set a settlement rule (250 bps platform, 50 bps network)
soroban contract invoke --id "$SETTLEMENT_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  set_settlement_rule \
  --merchant "$MERCHANT" \
  --rule '{"platform_fee_bps": 250, "network_fee_bps": 50, "settlement_delay_ledger": 0, "auto_settle": false}'

# Calculate fee split without storing
soroban contract invoke --id "$SETTLEMENT_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  calculate_fee_split --merchant "$MERCHANT" --amount 10000

# Store a payment reference (32-byte hex hash)
soroban contract invoke --id "$SETTLEMENT_ID" --source-account "$MERCHANT" \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  store_payment_reference \
  --merchant "$MERCHANT" \
  --reference "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890" \
  --amount 10000

# Fetch a stored payment
soroban contract invoke --id "$SETTLEMENT_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  get_payment_reference \
  --reference "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"

# Set a global default rule
soroban contract invoke --id "$SETTLEMENT_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  set_default_rule \
  --new_rule '{"platform_fee_bps": 100, "network_fee_bps": 0, "settlement_delay_ledger": 0, "auto_settle": false}'

# Pause/unpause
soroban contract invoke --id "$SETTLEMENT_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  pause

soroban contract invoke --id "$SETTLEMENT_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  unpause
```

### Invoke Governance Contract

```bash
GOVERNANCE_ID=CDPFWUTIXF5BC6BKNDLSQOZSDQCXAJNZFCZWHBE2RRHANRN25T3ILPZ7
ASSET=GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
ANCHOR=GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

# Get admin
soroban contract invoke --id "$GOVERNANCE_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  get_admin

# Set fee config (platform 120 bps, network 35 bps)
soroban contract invoke --id "$GOVERNANCE_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  set_fee_config \
  --caller "$ADMIN" \
  --config '{"platform_fee_bps": 120, "network_fee_bps": 35}'

# Read fee config
soroban contract invoke --id "$GOVERNANCE_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  get_fee_config

# Update a system parameter
soroban contract invoke --id "$GOVERNANCE_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  update_system_param --caller "$ADMIN" --key '"max_settle"' --value 1440

# Read a system parameter
soroban contract invoke --id "$GOVERNANCE_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  get_system_param --key '"max_settle"'

# Register an anchor for an asset
soroban contract invoke --id "$GOVERNANCE_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  upsert_anchor --caller "$ADMIN" --asset "$ASSET" --anchor "$ANCHOR"

# Read an anchor
soroban contract invoke --id "$GOVERNANCE_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  get_anchor --asset "$ASSET"

# Remove an anchor
soroban contract invoke --id "$GOVERNANCE_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  remove_anchor --caller "$ADMIN" --asset "$ASSET"

# Transfer admin to a new address
soroban contract invoke --id "$GOVERNANCE_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  transfer_admin --caller "$ADMIN" --new_admin "GYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY"
```

### Local Simulation and Testing

For local development and testing without deploying to testnet, use the `simulate.sh` script:

```bash
# Run complete local simulation (builds, deploys, and initializes both contracts)
bash scripts/simulate.sh

# This outputs contract IDs and source identity:
# Source identity: bettapay-sim
# Source address: GYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY
# Settlement contract ID: CYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY
# Governance contract ID: CYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY
```

After running `simulate.sh`, use the printed contract IDs for local testing:

```bash
# Example: Query settlement contract locally
SETTLEMENT_ID=CYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY
GOVERNANCE_ID=CYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYYY
SOURCE=bettapay-sim

# Get settlement contract admin
soroban contract invoke --id "$SETTLEMENT_ID" --source-account "$SOURCE" \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" -- \
  get_admin

# Register a test merchant
TEST_MERCHANT=GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

soroban contract invoke --id "$SETTLEMENT_ID" --source-account "$SOURCE" \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015" -- \
  register_merchant --merchant "$TEST_MERCHANT"
```

### Environment Variable Setup

For repetitive testing, create a local `.env` file or set these variables:

```bash
# Testnet Configuration
export SOROBAN_RPC_URL="https://soroban-testnet.stellar.org"
export SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
export SOROBAN_ACCOUNT="bettapay-admin"

# Contract Addresses (update after deployment)
export SETTLEMENT_CONTRACT_ID="CBGBGKJSUY7XYB6HWW4CVAU6MW2KD25FSF45E5KCP53TKUK374MBZNFB"
export GOVERNANCE_CONTRACT_ID="CDPFWUTIXF5BC6BKNDLSQOZSDQCXAJNZFCZWHBE2RRHANRN25T3ILPZ7"
export ADMIN_ADDRESS="GCCHHKNI7GRA5QWC7RCTT3OHO7SKAUMKQA6IBWEQEO2SXI3GF376UHDD"

# Use variables in commands:
soroban contract invoke --id "$SETTLEMENT_CONTRACT_ID" --source-account "$SOROBAN_ACCOUNT" \
  --rpc-url "$SOROBAN_RPC_URL" --network-passphrase "$SOROBAN_NETWORK_PASSPHRASE" -- \
  get_admin
```

### Common Tasks

#### Set up a new identity for testing

```bash
# Generate a new key
soroban keys generate test-merchant

# Fund it (Friendbot for testnet)
soroban keys fund test-merchant

# Get its address
MERCHANT_ADDR=$(soroban keys address test-merchant)
echo "Merchant address: $MERCHANT_ADDR"
```

#### Register and configure a merchant

```bash
SETTLEMENT_ID=CBGBGKJSUY7XYB6HWW4CVAU6MW2KD25FSF45E5KCP53TKUK374MBZNFB
ADMIN_ACCOUNT="bettapay-admin"
MERCHANT_ADDR=$(soroban keys address test-merchant)
RPC="https://soroban-testnet.stellar.org"
PASS="Test SDF Network ; September 2015"

# Register merchant
soroban contract invoke --id "$SETTLEMENT_ID" --source-account "$ADMIN_ACCOUNT" \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  register_merchant --merchant "$MERCHANT_ADDR"

# Set settlement rule (e.g., 250 bps platform fee, 50 bps network fee, immediate settlement)
soroban contract invoke --id "$SETTLEMENT_ID" --source-account "$ADMIN_ACCOUNT" \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  set_settlement_rule \
  --merchant "$MERCHANT_ADDR" \
  --rule '{"platform_fee_bps": 250, "network_fee_bps": 50, "settlement_delay_ledger": 0, "auto_settle": false}'

# Verify merchant is registered
soroban contract invoke --id "$SETTLEMENT_ID" --source-account "$ADMIN_ACCOUNT" \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  is_merchant_registered --merchant "$MERCHANT_ADDR"
```

#### Test settlement calculations

```bash
SETTLEMENT_ID=CBGBGKJSUY7XYB6HWW4CVAU6MW2KD25FSF45E5KCP53TKUK374MBZNFB
MERCHANT_ADDR=$(soroban keys address test-merchant)

# Calculate fees for a 10,000 stroops payment
soroban contract invoke --id "$SETTLEMENT_ID" --source-account bettapay-admin \
  --rpc-url "$RPC" --network-passphrase "$PASS" -- \
  calculate_fee_split --merchant "$MERCHANT_ADDR" --amount 10000

# Example output: {platform: 2500, network: 500, merchant: 7000}
```


### settlement_contract

Handles the on-chain settlement layer:
- `init(admin)` — one-time initialization, sets admin
- `register_merchant(merchant)` — admin registers a merchant address
- `set_settlement_rule(merchant, rule)` — admin sets fee BPS and settlement config
- `store_payment_reference(merchant, reference, amount)` — merchant anchors a payment hash on-chain, emits events, calculates fee split
- `calculate_fee_split(merchant, amount)` — read-only fee split calculation
- `get_payment_reference(reference)` — fetch stored payment record
- `is_merchant_registered(merchant)` — boolean check

### governance_contract

Handles protocol-level configuration:
- `init(admin)` — one-time initialization
- `set_fee_config(config)` — admin sets platform + network fee BPS
- `get_fee_config()` — read current fee config
- `update_system_param(key, value)` — generic key/value system config
- `get_system_param(key)` — read system param
- `upsert_anchor(asset, anchor)` — register/update anchor for asset
- `remove_anchor(asset)` — remove anchor
- `get_anchor(asset)` — read anchor for asset

## Soroban SDK Version

`soroban-sdk = "21.7.7"`

## Dependencies

No cross-contract calls. Both contracts are independently deployable and stateless across each other. The backend services call them via Stellar RPC.
