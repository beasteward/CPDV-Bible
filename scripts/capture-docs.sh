#!/usr/bin/env bash
# Regenerate the Rust server's embedded Swagger UI assets from the Node
# service, so /docs stays byte-identical. Run after changing
# src/docs/openapi.ts or upgrading swagger-ui-express/swagger-ui-dist.
# Needs: npm ci && npm run build.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=crates/server/assets/docs
PORT=3299
export NO_PROXY=localhost,127.0.0.1
PORT=$PORT API_KEYS= CORS_ORIGINS= LOG_LEVEL=silent node dist/index.js >/dev/null 2>&1 &
PID=$!
trap 'kill $PID' EXIT
until curl -s -o /dev/null "http://127.0.0.1:$PORT/"; do sleep 0.1; done
mkdir -p "$OUT"
curl -sf "http://127.0.0.1:$PORT/docs/" -o "$OUT/index.html"
curl -sf "http://127.0.0.1:$PORT/docs/swagger-ui-init.js" -o "$OUT/swagger-ui-init.js"
for f in swagger-ui.css swagger-ui-bundle.js swagger-ui-standalone-preset.js favicon-16x16.png favicon-32x32.png; do
  cp "node_modules/swagger-ui-dist/$f" "$OUT/"
done
echo "captured into $OUT; rebuild the Rust server"
