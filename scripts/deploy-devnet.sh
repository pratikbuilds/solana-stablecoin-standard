#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_FILE="$ROOT_DIR/tests/devnet/fixtures/program-ids.json"

cd "$ROOT_DIR"

anchor build
anchor deploy --provider.cluster devnet

STABLECOIN_PROGRAM_ID="$(solana address -k target/deploy/stablecoin-keypair.json)"
TRANSFER_HOOK_PROGRAM_ID="$(solana address -k target/deploy/transfer_hook-keypair.json)"

mkdir -p "$(dirname "$FIXTURE_FILE")"
printf '{\n  "cluster": "devnet",\n  "stablecoinProgramId": "%s",\n  "transferHookProgramId": "%s"\n}\n' \
  "$STABLECOIN_PROGRAM_ID" \
  "$TRANSFER_HOOK_PROGRAM_ID" >"$FIXTURE_FILE"

printf 'Wrote %s\n' "$FIXTURE_FILE"
