#!/usr/bin/env bash
# Run the parity harness against local Node (reference) and Rust servers in
# three configurations. Usage: scripts/parity/run-local.sh [--all-verses]
# Needs: npm ci && npm run build; cargo build --release.
set -euo pipefail
cd "$(dirname "$0")/../.."
export NO_PROXY=localhost,127.0.0.1 no_proxy=localhost,127.0.0.1
OUT=${OUT:-parity-reports}
mkdir -p "$OUT"
export CPDV_KEY="local-test-key-$RANDOM" CPDV_KEY2="local-test-key-two"
NODE_PORT=3201 RUST_PORT=3202
EXTRA=("$@")
rc=0

start() { # profile env...
  pkill -f "node dist/index.js" 2>/dev/null || true
  pkill -f "target/release/cpdv-server" 2>/dev/null || true
  sleep 0.5
  env "$@" PORT=$NODE_PORT LOG_LEVEL=silent node dist/index.js >"$OUT/node.log" 2>&1 &
  env "$@" PORT=$RUST_PORT LOG_LEVEL=warn ./target/release/cpdv-server >"$OUT/rust.log" 2>&1 &
  for _ in $(seq 50); do
    curl -s -o /dev/null "http://127.0.0.1:$NODE_PORT/" && curl -s -o /dev/null "http://127.0.0.1:$RUST_PORT/" && return
    sleep 0.2
  done
  echo "servers did not start"; exit 2
}

run() { # profile extra-args
  local p=$1; shift
  echo "=== profile $p"
  python3 -u scripts/parity/parity.py --a "http://127.0.0.1:$NODE_PORT" --b "http://127.0.0.1:$RUST_PORT" \
    --profile "$p" --report "$OUT/local-$p.json" "$@" || rc=1
}

# main: API keys + CORS allowlist (same shape as production), big rate limit
start API_KEYS="$CPDV_KEY,$CPDV_KEY2" CORS_ORIGINS="https://biblehodl.com,*.example.org" RATE_LIMIT_MAX=1000000
run main "${EXTRA[@]}"
# open: no keys, CORS '*'
start API_KEYS= CORS_ORIGINS="*" RATE_LIMIT_MAX=1000000
run open
# ratelimit: tiny limit
start API_KEYS= CORS_ORIGINS="https://biblehodl.com" RATE_LIMIT_MAX=3 RATE_LIMIT_WINDOW_MS=60000
run ratelimit
pkill -f "node dist/index.js" 2>/dev/null || true
pkill -f "target/release/cpdv-server" 2>/dev/null || true
exit $rc
