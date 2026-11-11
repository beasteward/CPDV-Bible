# Cutover: Node -> Rust (CPDV Bible API)

Status (2026-11-11 ~12:55 MST): **CUT OVER by URL (Option A)**, approved by
Peter at 12:18 ("switch over whenever it's ready"). Both callers now use the
Rust service. The Node service `CPDV-Bible` is unchanged and still running as
the rollback; keep it at least until 2026-11-18. Don't delete it, scale it
down or redeploy it before then.

## What runs where

| | Node (prod) | Rust (candidate) |
|---|---|---|
| Railway service | `CPDV-Bible` (3c53105b-9f2c-4451-8ca4-5f3299b4b99b) | `CPDV-Bible-rs` (new, same project `focused-youthfulness`) |
| URL | https://cpdv-bible-production.up.railway.app | https://cpdv-bible-rs-production.up.railway.app (service id 01fd49ff-c0b1-4341-b39d-0a6326726951) |
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

Live, 2026-11-11 12:43-12:50, old prod vs new Rust URL (`--profile live`,
paced, aborts if prod `RateLimit-Remaining` < 40): **513 requests, 0
unexplained diffs** (E1 only). The run was split in two (440 + 73 with
`--start-at 440`) because the guard tripped once. It covers every book (meta +
chapter list + last chapter), aliases, psalms, prologues, bad input, ~70 search
queries, auth (missing/empty/wrong key -> 401/403), CORS + preflights, HEAD/304,
/docs and verse of the day (both: Genesis 39:17). Latency through the egress
proxy (both sides include ~200 ms of tunnel overhead): Node p50 400 ms vs Rust
p50 225 ms. Railway memory: Node 73-77 MB, Rust 32 MB.

Re-run with:

```bash
CPDV_KEY=... python3 scripts/parity/parity.py --profile live \
  --a https://cpdv-bible-production.up.railway.app --b https://<rust url> \
  --pace 0.05 --min-remaining 20 --report parity-reports/live.json
```

## What was switched (2026-11-11)

1. Railway: created `CPDV-Bible-rs` in focused-youthfulness/production. Source
   is beasteward/CPDV-Bible, branch `rust`, `dockerfilePath=Dockerfile.rust`,
   healthcheck `/`. `API_KEYS` and `CORS_ORIGINS` were copied from `CPDV-Bible`
   via the API (values never printed), plus `RAILWAY_DOCKERFILE_PATH`. First
   deploy 8b07bd91 SUCCESS (bbe6af1). `CPDV-Bible` untouched: deploy 6bb13594,
   main 88989fa.
2. Emmaus Road: `apps/web/wrangler.jsonc` `CPDV_API_URL` ->
   `https://cpdv-bible-rs-production.up.railway.app/api/v1` (emmaus-road commit
   cb19488), Worker version b2be5bab-ecac-49c6-93d3-25bc5eafbe58.
   check:refs 49/49; /api/bible/Psalms/21 200 with 32 verses; unknown book or
   chapter still 404; / and /play 200.
3. BibleHodl (VPS 93.188.163.15): `/opt/biblehodl/.env` backed up to
   `.env.pre-cpdv-rs.bak`, only `CPDV_API_URL` changed (diff of all other
   lines empty), then `docker compose up -d --no-deps --force-recreate app`.
   Container healthy; from inside it, chapter/votd/search via the new URL
   return 200 `ok`; biblehodl.com 200; /api/bible/* 401 unauthenticated.

## Rollback (seconds, no rebuild)

* BibleHodl: `cd /opt/biblehodl && cp -p .env.pre-cpdv-rs.bak .env && sudo docker compose up -d --no-deps --force-recreate app`
* Emmaus Road: set `CPDV_API_URL` back to `https://cpdv-bible-production.up.railway.app/api/v1`
  in `apps/web/wrangler.jsonc` (or `git revert cb19488`), then from `apps/web`
  run `npx opennextjs-cloudflare build && npx wrangler deploy`. The edge cache
  holds chapter text either way, and the text is identical.
