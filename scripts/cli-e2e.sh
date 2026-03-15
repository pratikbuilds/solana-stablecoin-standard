#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

PROGRAM_IDS_FILE="$ROOT_DIR/tests/devnet/fixtures/program-ids.json"
if [[ ! -f "$PROGRAM_IDS_FILE" ]]; then
  echo "Missing $PROGRAM_IDS_FILE. Deploy the programs first."
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required"
  exit 1
fi

SOLANA_RPC_URL="${SOLANA_RPC_URL:-https://api.devnet.solana.com}"
ADMIN_DB_URL="${TEST_DATABASE_ADMIN_URL:-postgres://localhost/postgres}"
API_BIND="${SSS_API_BIND:-127.0.0.1:18080}"
API_URL="http://$API_BIND"
ADMIN_KEYPAIR="${SSS_AUTHORITY_KEYPAIR:-$HOME/.config/solana/id.json}"
STABLECOIN_PROGRAM_ID="$(jq -r .stablecoinProgramId "$PROGRAM_IDS_FILE")"
TRANSFER_HOOK_PROGRAM_ID="$(jq -r .transferHookProgramId "$PROGRAM_IDS_FILE")"
TMP_DIR="$(mktemp -d)"
CONFIG_PATH="$TMP_DIR/config.toml"
ROLE_KEYPAIR="$TMP_DIR/role.json"
RECIPIENT_KEYPAIR="$TMP_DIR/recipient.json"
DB_NAME="sss_cli_e2e_${USER}_$$"
DATABASE_URL="${ADMIN_DB_URL%/*}/$DB_NAME"
API_LOG="$TMP_DIR/api.log"
INDEXER_LOG="$TMP_DIR/indexer.log"
API_PID=""
INDEXER_PID=""
CLI_BIN="$ROOT_DIR/target/debug/sss-token"

cleanup() {
  set +e
  if [[ -n "$INDEXER_PID" ]]; then
    kill "$INDEXER_PID" >/dev/null 2>&1 || true
    wait "$INDEXER_PID" 2>/dev/null || true
  fi
  if [[ -n "$API_PID" ]]; then
    kill "$API_PID" >/dev/null 2>&1 || true
    wait "$API_PID" 2>/dev/null || true
  fi
  psql "$ADMIN_DB_URL" -c "drop database if exists $DB_NAME" >/dev/null 2>&1 || true
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

run_admin() {
  SSS_CONFIG="$CONFIG_PATH" \
  SSS_AUTHORITY_KEYPAIR="$ADMIN_KEYPAIR" \
  "$CLI_BIN" --rpc-url "$SOLANA_RPC_URL" "$@"
}

run_role() {
  SSS_CONFIG="$CONFIG_PATH" \
  SSS_AUTHORITY_KEYPAIR="$ROLE_KEYPAIR" \
  "$CLI_BIN" --rpc-url "$SOLANA_RPC_URL" "$@"
}

extract_request_id() {
  awk -F': ' '/^request_id:/ {print $2; exit}'
}

wait_for_status() {
  local request_id="$1"
  local expected="$2"
  local attempt=0
  while (( attempt < 30 )); do
    local output
    output="$(run_admin operation get "$request_id")"
    if grep -q "^status: $expected$" <<<"$output"; then
      printf '%s\n' "$output"
      return 0
    fi
    sleep 2
    attempt=$((attempt + 1))
  done
  echo "Timed out waiting for request $request_id to reach $expected"
  run_admin operation get "$request_id" || true
  return 1
}

wait_for_audit_log() {
  local attempt=0
  while (( attempt < 120 )); do
    local output
    output="$(run_admin audit-log --limit 20)"
    if grep -q '^event_type:' <<<"$output"; then
      printf '%s\n' "$output"
      return 0
    fi
    sleep 2
    attempt=$((attempt + 1))
  done
  echo "Timed out waiting for audit log events"
  return 1
}

start_api() {
  local run_workers="$1"
  DATABASE_URL="$DATABASE_URL" \
  SOLANA_RPC_URL="$SOLANA_RPC_URL" \
  SSS_API_BIND="$API_BIND" \
  SSS_RUN_WORKERS="$run_workers" \
  SSS_AUTHORITY_KEYPAIR="$ADMIN_KEYPAIR" \
  SSS_STABLECOIN_PROGRAM_ID="$STABLECOIN_PROGRAM_ID" \
  cargo run -p sss-api >"$API_LOG" 2>&1 &
  API_PID=$!

  for _ in $(seq 1 30); do
    if curl -fsS "$API_URL/readyz" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  curl -fsS "$API_URL/readyz" >/dev/null
}

echo "==> Creating local test database $DB_NAME"
psql "$ADMIN_DB_URL" -c "drop database if exists $DB_NAME" >/dev/null
psql "$ADMIN_DB_URL" -c "create database $DB_NAME" >/dev/null

echo "==> Building CLI binary"
cargo build -p sss-admin-cli >/dev/null

echo "==> Starting API with workers on $API_URL"
start_api 1

CURRENT_SLOT="$(solana slot --url "$SOLANA_RPC_URL")"

echo "==> Generating role and recipient wallets"
solana-keygen new --no-bip39-passphrase -s -o "$ROLE_KEYPAIR" >/dev/null
solana-keygen new --no-bip39-passphrase -s -o "$RECIPIENT_KEYPAIR" >/dev/null
ROLE_WALLET="$(solana address -k "$ROLE_KEYPAIR")"
RECIPIENT_WALLET="$(solana address -k "$RECIPIENT_KEYPAIR")"
ADMIN_WALLET="$(solana address -k "$ADMIN_KEYPAIR")"

echo "==> Funding role wallet $ROLE_WALLET"
solana transfer --url "$SOLANA_RPC_URL" --allow-unfunded-recipient "$ROLE_WALLET" 0.1 >/dev/null

echo "==> Initializing SSS-2 mint from CLI"
run_admin init \
  --preset sss-2 \
  --api-url "$API_URL" \
  --authority-keypair "$ADMIN_KEYPAIR" \
  --yes

MINT="$(awk -F'"' '/^mint = / {print $2; exit}' "$CONFIG_PATH")"
if [[ -z "$MINT" ]]; then
  echo "Failed to read mint from $CONFIG_PATH"
  exit 1
fi

echo "==> Mint created: $MINT"
echo "==> Creating Token-2022 ATAs"
ADMIN_ATA="$(spl-token address --url "$SOLANA_RPC_URL" --program-2022 --owner "$ADMIN_WALLET" --token "$MINT" --fee-payer "$ADMIN_KEYPAIR" --verbose | awk -F': ' '/Associated token address:/ {print $2; exit}')"
RECIPIENT_ATA="$(spl-token address --url "$SOLANA_RPC_URL" --program-2022 --owner "$RECIPIENT_WALLET" --token "$MINT" --fee-payer "$ADMIN_KEYPAIR" --verbose | awk -F': ' '/Associated token address:/ {print $2; exit}')"
spl-token create-account --url "$SOLANA_RPC_URL" --program-2022 "$MINT" --owner "$ADMIN_WALLET" --fee-payer "$ADMIN_KEYPAIR" >/dev/null
spl-token create-account --url "$SOLANA_RPC_URL" --program-2022 "$MINT" --owner "$RECIPIENT_WALLET" --fee-payer "$ADMIN_KEYPAIR" >/dev/null

echo "==> Assigning CLI roles to $ROLE_WALLET"
run_admin roles set \
  --pauser "$ROLE_WALLET" \
  --blacklister "$ROLE_WALLET" \
  --seizer "$ROLE_WALLET" \
  --burner "$ROLE_WALLET" \
  --yes
run_admin roles get

echo "==> Thawing default-frozen ATAs before mint requests"
run_role thaw "$ADMIN_ATA" --yes
run_role thaw "$RECIPIENT_ATA" --yes

echo "==> Checking minter list"
run_admin minters list

echo "==> Testing minter add/remove"
run_admin minters add "$RECIPIENT_WALLET" --quota 500000 --yes
MINTERS_AFTER_ADD="$(run_admin minters list)"
if ! awk -v wallet="$RECIPIENT_WALLET" 'BEGIN { RS=""; FS="\n" } $0 ~ ("minter: " wallet) && $0 ~ /quota: 500000/ && $0 ~ /active: true/ { found = 1 } END { exit(found ? 0 : 1) }' <<<"$MINTERS_AFTER_ADD"; then
  echo "Recipient wallet was not added as minter"
  exit 1
fi
run_admin minters remove "$RECIPIENT_WALLET" --yes
MINTERS_AFTER_REMOVE="$(run_admin minters list)"
if ! awk -v wallet="$RECIPIENT_WALLET" 'BEGIN { RS=""; FS="\n" } $0 ~ ("minter: " wallet) && $0 ~ /quota: 0/ && $0 ~ /active: false/ { found = 1 } END { exit(found ? 0 : 1) }' <<<"$MINTERS_AFTER_REMOVE"; then
  echo "Recipient wallet was not deactivated as minter"
  exit 1
fi

echo "==> Creating mint request to recipient wallet"
run_admin mint "$RECIPIENT_WALLET" 1000000 --yes
MINT_REQ_ID="$(run_admin operation list --status requested --type mint --limit 1 | extract_request_id)"
echo "Mint request id: $MINT_REQ_ID"
run_admin operation approve "$MINT_REQ_ID" --approved-by cli-e2e
wait_for_status "$MINT_REQ_ID" finalized >/dev/null

echo "==> Creating treasury mint request so burn request can use admin ATA"
run_admin mint "$ADMIN_WALLET" 600000 --yes
TREASURY_MINT_REQ_ID="$(run_admin operation list --status requested --type mint --limit 1 | extract_request_id)"
echo "Treasury mint request id: $TREASURY_MINT_REQ_ID"
run_admin operation approve "$TREASURY_MINT_REQ_ID" --approved-by cli-e2e
wait_for_status "$TREASURY_MINT_REQ_ID" finalized >/dev/null

echo "==> Testing burn request from admin ATA"
run_admin burn 200000 --account "$ADMIN_ATA" --yes
BURN_REQ_ID="$(run_admin operation list --status requested --type burn --limit 1 | extract_request_id)"
echo "Burn request id: $BURN_REQ_ID"
run_admin operation approve "$BURN_REQ_ID" --approved-by cli-e2e
wait_for_status "$BURN_REQ_ID" finalized >/dev/null

echo "==> Testing status, supply, and holders"
run_admin status
run_admin supply
run_admin holders --min-balance 1 --limit 10

echo "==> Testing freeze/thaw with role wallet"
run_role freeze "$RECIPIENT_ATA" --yes
run_role thaw "$RECIPIENT_ATA" --yes

echo "==> Testing pause/unpause with role wallet"
run_role pause --yes
if ! grep -q '^paused: true$' <<<"$(run_admin status)"; then
  echo "Mint did not report paused after pause command"
  exit 1
fi
run_role unpause --yes
if ! grep -q '^paused: false$' <<<"$(run_admin status)"; then
  echo "Mint did not report unpaused after unpause command"
  exit 1
fi

echo "==> Testing blacklist add/remove with role wallet"
run_role blacklist add "$RECIPIENT_WALLET" --reason "cli-e2e blacklist" --yes
run_role blacklist remove "$RECIPIENT_WALLET" --yes

echo "==> Re-adding blacklist and freezing for seize"
run_role blacklist add "$RECIPIENT_WALLET" --reason "cli-e2e seize" --yes
run_role freeze "$RECIPIENT_ATA" --yes
run_role seize "$RECIPIENT_ATA" --to "$ADMIN_ATA" --amount 100000 --yes
run_role thaw "$RECIPIENT_ATA" --yes
run_role blacklist remove "$RECIPIENT_WALLET" --yes

echo "==> Starting indexer from slot $CURRENT_SLOT for audit log backfill"
if [[ -n "$API_PID" ]]; then
  kill "$API_PID" >/dev/null 2>&1 || true
  wait "$API_PID" 2>/dev/null || true
  API_PID=""
fi

echo "==> Restarting API without workers for audit log readback"
start_api 0

DATABASE_URL="$DATABASE_URL" \
SOLANA_RPC_URL="$SOLANA_RPC_URL" \
SSS_STABLECOIN_PROGRAM_ID="$STABLECOIN_PROGRAM_ID" \
SSS_TRANSFER_HOOK_PROGRAM_ID="$TRANSFER_HOOK_PROGRAM_ID" \
SSS_START_SLOT="$CURRENT_SLOT" \
SSS_DISABLE_BLOCK_SUBSCRIBE=1 \
cargo run -p sss-indexer >"$INDEXER_LOG" 2>&1 &
INDEXER_PID=$!

echo "==> Waiting for indexer to ingest events"
wait_for_audit_log

echo "==> CLI devnet E2E completed"
echo "Mint: $MINT"
echo "Recipient wallet: $RECIPIENT_WALLET"
echo "Recipient ATA: $RECIPIENT_ATA"
echo "Admin ATA: $ADMIN_ATA"
