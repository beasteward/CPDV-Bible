# Cutover: Node -> Rust (CPDV Bible API)

Status (2026-11-11): Rust build on branch `rust`, parity-tested locally.
**Production still runs the Node build. Nothing below has been executed.**
Cutover needs Peter's approval.

## What runs where

| | Node (prod) | Rust (candidate) |
|---|---|---|
| Railway service | `CPDV-Bible` (3c53105b-9f2c-4451-8ca4-5f3299b4b99b) | `CPDV-Bible-rs` (new, same project `focused-youthfulness`) |
| URL | https://cpdv-bible-production.up.railway.app | own `*.up.railway.app` domain (see memory / deploy notes) |
| Source | GitHub `beasteward/CPDV-Bible` branch `main`, root `Dockerfile` | same repo, branch `rust`, `RAILWAY_DOCKERFILE_PATH=Dockerfile.rust` |
| Deploy trigger | Railway GitHub app, auto-deploys every push to `main` (railway-app deployments for 672a501 and 88989fa) | push to `rust` |
| Env | `API_KEYS`, `CORS_ORIGINS`, `RATE_LIMIT_*`, `LOG_LEVEL`, `NODE_ENV`, `PORT` | same values (same API key, so callers switch by URL only) + `RAILWAY_DOCKERFILE_PATH` |

Pushing to `rust` never touches the Node service. Merging `rust` into `main`
by itself is also safe: the root `Dockerfile` (Node) is unchanged and Railway
keeps building it until `RAILWAY_DOCKERFILE_PATH` is set on `CPDV-Bible`.

## Callers

* **BibleHodl** (biblehodl.com, VPS 93.188.163.15, `/opt/biblehodl`):
  server-side only, `app/lib/cpdv.ts`, env `CPDV_API_URL` + `CPDV_API_KEY`.
  It treats any non-2xx as 502 except 404, and requires `status: "ok"`.
* **Emmaus Road** `apps/web` `/api/bible/*` proxy (coming): same contract.

## Option A (recommended): switch callers by URL

1. Run the live parity check (below) the same day; require 0 unexplained diffs.
2. Emmaus Road first (not yet in production): point its CPDV base URL var at
   the Rust URL, deploy, smoke-test the reader routes.
3. BibleHodl: on the VPS set `CPDV_API_URL=<rust url>` in the app env (key
   unchanged), restart the app (PM2), then check the Bible pages (books list,
   a chapter, search, verse of the day). Needs Peter's OK: it is the prod server.
4. Keep the Node service running unchanged for 1-2 weeks as the rollback.
5. Then either delete `CPDV-Bible` and move the custom URL/domain, or do
   Option B to get the old URL back on Rust.

Rollback: set `CPDV_API_URL` back to `https://cpdv-bible-production.up.railway.app`
and restart. Seconds, no rebuild; the Node service never stopped.

## Option B: swap the build inside the existing service (keeps the URL)

1. Merge `rust` into `main` (Node still builds; harmless).
2. On `CPDV-Bible` set `RAILWAY_DOCKERFILE_PATH=Dockerfile.rust` -> Railway
   redeploys the same service with the Rust image. URL unchanged, no caller change.
3. Rollback: in Railway, redeploy the previous (Node) deployment, or delete
   the variable and redeploy. A minute or two, and the URL briefly serves
   whatever finished deploying last.

B touches the production service; A doesn't. Prefer A until Rust has run
in front of real traffic.

## What to watch after switching

* 5xx rate and the `{"status":"error"}` bodies (Railway logs; Rust logs one
  line per request at `LOG_LEVEL=info`).
* 401/403 counts: a non-zero jump means the key wasn't copied correctly.
* 429s. Note: in both builds the limiter keys on the TCP peer address. On
  Railway that is the edge proxy, not the client, and prod showed
  `RateLimit-Remaining: 99` on three back-to-back requests, so the per-client
  limit is effectively not enforced today. Rust keeps that behavior by
  default; `TRUST_PROXY=1` (Rust only) keys on the first `X-Forwarded-For`
  address instead. Decide separately, after cutover; it changes behavior.
* Memory and latency in the Railway metrics (expected: much lower, see below).
* Verse of the day: both builds use the container's local date (UTC unless
  `TZ` is set). Keep `TZ` identical on both services or they pick different verses.
* Rate-limit counters are in memory and restart from zero on any deploy (same as Node).

## Differences you should know about

See `docs/PARITY.md`: E1 reduced `/docs` static serving (only the 5 files the
Swagger page loads); E2 no `Content-Length: 0` on 204 preflights. Neither
affects `/api/v1`.

## Results (2026-11-11)

Local (same machine, identical env, `scripts/parity/run-local.sh --all-verses`):

| profile | requests | unexplained diffs | explained |
|---|---|---|---|
| main (keys + CORS allowlist) | 40,374 (every verse) | 0 | E1 |
| open (no keys, CORS `*`) | 5,855 | 0 | E1 |
| ratelimit (max 3) | 7 | 0 | - |

Local performance (`scripts/parity/bench-local.sh`, 16 keep-alive clients,
mixed routes, same 2-vCPU host):

| | Node 24 | Rust |
|---|---|---|
| RSS idle | 175 MB | 31 MB |
| RSS after 16k requests | 197 MB | 32 MB |
| throughput | 938 req/s | 3,217 req/s |
| p50 / p99 under load | 14.9 / 47.9 ms | 4.2 / 15.1 ms |
| startup to first response | 2.0 s | 0.48 s |
| image | node:22-alpine + node_modules | debian-slim + 4.8 MB binary |

Live (old prod URL vs new Rust URL): pending, see the final report /
memory notes; run with:

```bash
CPDV_KEY=... python3 scripts/parity/parity.py --profile live \
  --a https://cpdv-bible-production.up.railway.app --b https://<rust url> \
  --pace 0.05 --min-remaining 20 --report parity-reports/live.json
```
