#!/usr/bin/env bash
# DevNet API smoke test: events + lifecycle flow via curl.
# Prereqs: API running (cargo run -p sss-api), indexer run (SSS_RUN_INDEXER=1 ./scripts/devnet-e2e.sh or manually).
# Usage: ./scripts/devnet-api-smoke.sh [API_BASE_URL]
#   API_BASE_URL defaults to http://127.0.0.1:8080
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CREDS="$ROOT_DIR/tests/devnet/fixtures/e2e-credentials.json"
BASE="${1:-http://127.0.0.1:8080}"

if [[ ! -f "$CREDS" ]]; then
  echo "Run ./scripts/devnet-e2e.sh first to create credentials."
  exit 1
fi

MINT=$(jq -r .mint "$CREDS")
TARGET_ATA=$(jq -r .targetAta "$CREDS")
TARGET_WALLET=$(jq -r .targetWallet "$CREDS")

echo "=== Health ==="
curl -s "$BASE/healthz" | jq .
curl -s "$BASE/readyz" | jq .

echo ""
echo "=== Events for mint $MINT ==="
curl -s "$BASE/v1/mints/$MINT/events?limit=5" | jq '.events | length, .total'

echo ""
echo "=== Create mint request ==="
MINT_RESP=$(curl -s -X POST "$BASE/v1/mint-requests" \
  -H "Content-Type: application/json" \
  -d "{
    \"mint\": \"$MINT\",
    \"recipient\": \"$TARGET_WALLET\",
    \"token_account\": \"$TARGET_ATA\",
    \"amount\": 1000000,
    \"requested_by\": \"smoke-test\"
  }")
echo "$MINT_RESP" | jq .
REQ_ID=$(echo "$MINT_RESP" | jq -r '.id')
if [[ -z "$REQ_ID" || "$REQ_ID" == "null" ]]; then
  echo "Failed to create mint request"
  exit 1
fi

echo ""
echo "=== Get operation $REQ_ID ==="
curl -s "$BASE/v1/operations/$REQ_ID" | jq .

echo ""
echo "=== Approve ==="
curl -s -X POST "$BASE/v1/operations/$REQ_ID/approve" \
  -H "Content-Type: application/json" \
  -d '{"approved_by":"smoke-test"}' | jq .

echo ""
echo "=== Execute (signals worker) ==="
curl -s -X POST "$BASE/v1/operations/$REQ_ID/execute" | jq .

echo ""
echo "=== Get operation (check status) ==="
curl -s "$BASE/v1/operations/$REQ_ID" | jq '.request.status, .request.tx_signature'

echo ""
echo "Done. If SSS_RUN_WORKERS=1, worker will submit tx. Check status again in a few seconds."
