#!/usr/bin/env bash
# Full DevNet E2E: deploy (optional), create preset + credentials, run API mint flow test.
# Requires: DATABASE_URL, SOLANA_RPC_URL (or default devnet).
# Optional: TEST_DATABASE_ADMIN_URL to create a temporary DB.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
FIXTURES="$ROOT_DIR/tests/devnet/fixtures"
CREDS="$FIXTURES/e2e-credentials.json"
PROGRAM_IDS="$FIXTURES/program-ids.json"

export SOLANA_RPC_URL="${SOLANA_RPC_URL:-https://api.devnet.solana.com}"

if [[ ! -f "$PROGRAM_IDS" ]]; then
  echo "Deploying programs to DevNet..."
  yarn test:devnet:deploy
fi

echo "Creating SSS-1 preset and E2E credentials..."
npx tsx tests/devnet/create-e2e-credentials.ts

if [[ ! -f "$CREDS" ]]; then
  echo "Missing $CREDS"
  exit 1
fi

MINT=$(jq -r .mint "$CREDS")
TARGET_ATA=$(jq -r .targetAta "$CREDS")
AUTHORITY_KEYPAIR=$(jq -r .authorityKeypairPath "$CREDS")
STABLECOIN_PROGRAM_ID=$(jq -r .stablecoinProgramId "$PROGRAM_IDS")

export SSS_DEVNET_E2E=1
export SSS_DEVNET_MINT="$MINT"
export SSS_DEVNET_TARGET_ATA="$TARGET_ATA"
export SSS_AUTHORITY_KEYPAIR="$AUTHORITY_KEYPAIR"
export SSS_STABLECOIN_PROGRAM_ID="$STABLECOIN_PROGRAM_ID"

if [[ -z "${DATABASE_URL:-}" ]]; then
  if [[ -n "${TEST_DATABASE_ADMIN_URL:-}" ]]; then
    DB_NAME="sss_e2e_$$"
    psql "$TEST_DATABASE_ADMIN_URL" -c "create database $DB_NAME" 2>/dev/null || true
    export DATABASE_URL="${TEST_DATABASE_ADMIN_URL%/*}/$DB_NAME"
    echo "Using temporary database $DB_NAME"
  else
    echo "No DATABASE_URL or TEST_DATABASE_ADMIN_URL: test will start ephemeral Postgres."
  fi
fi

echo "Running devnet E2E test (mint request → approve → execute → worker submits tx)..."
cargo test -p sss-api devnet_e2e_mint_execution --test api_integration -- --ignored --nocapture
