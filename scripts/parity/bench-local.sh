#!/usr/bin/env bash
# Local memory/throughput/startup comparison, Node vs Rust (same machine).
set -u
cd "$(dirname "$0")/../.."
export NO_PROXY=localhost,127.0.0.1 no_proxy=localhost,127.0.0.1
pkill -f "[n]ode dist/index.js"; pkill -f "[t]arget/release/cpdv-server"; sleep 0.5
rss() { echo "RSS=$(awk '/VmRSS/{print $2}' /proc/$1/status)kB peak=$(awk '/VmHWM/{print $2}' /proc/$1/status)kB"; }
PORT=3301 LOG_LEVEL=silent API_KEYS= RATE_LIMIT_MAX=10000000 node dist/index.js >/dev/null 2>&1 & NP=$!
PORT=3302 LOG_LEVEL=warn API_KEYS= RATE_LIMIT_MAX=10000000 ./target/release/cpdv-server >/dev/null 2>&1 & RP=$!
until curl -s -o /dev/null localhost:3301/ && curl -s -o /dev/null localhost:3302/; do sleep 0.1; done; sleep 1
echo "idle   node $(rss $NP)   rust $(rss $RP)"
python3 scripts/parity/load.py 3301 1000 node
python3 scripts/parity/load.py 3302 1000 rust
echo "loaded node $(rss $NP)   rust $(rss $RP)"
kill $NP $RP; sleep 0.5
t=$(date +%s%N); PORT=3302 LOG_LEVEL=warn ./target/release/cpdv-server >/dev/null 2>&1 & RP=$!
until curl -s -o /dev/null localhost:3302/; do sleep 0.01; done; echo "startup rust $(( ($(date +%s%N)-t)/1000000 ))ms"
t=$(date +%s%N); PORT=3301 LOG_LEVEL=silent node dist/index.js >/dev/null 2>&1 & NP=$!
until curl -s -o /dev/null localhost:3301/; do sleep 0.01; done; echo "startup node $(( ($(date +%s%N)-t)/1000000 ))ms"
kill $NP $RP
