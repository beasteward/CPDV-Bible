# Node vs Rust parity

The Rust server (`crates/server`) must be a drop-in replacement for the Node
service on every observable detail of `/api/v1`: status codes, body bytes
(key order, number formatting, error messages), and headers (helmet, CORS,
rate limit, ETag/304). `scripts/parity/parity.py` checks that by sending the
same requests to both servers and diffing them.

## Running it

```bash
npm ci && npm run build          # Node reference (dist/)
cargo build --release            # Rust candidate
scripts/parity/run-local.sh              # ~6k requests x 3 configs, ~20 s
scripts/parity/run-local.sh --all-verses # + every verse of the Bible (~46k requests, ~75 s)
```

`run-local.sh` starts both servers with identical environment variables in
three configurations and writes JSON reports to `parity-reports/` (gitignored):

| profile     | env                                                                 | what it covers |
|-------------|---------------------------------------------------------------------|----------------|
| `main`      | `API_KEYS` = two keys, `CORS_ORIGINS=https://biblehodl.com,*.example.org` | production shape |
| `open`      | no keys, `CORS_ORIGINS=*`                                            | open mode, `Access-Control-Allow-Origin: *` |
| `ratelimit` | `RATE_LIMIT_MAX=3`                                                   | 429 body + `Retry-After`, preflights |

Against two deployments (no local servers): see "Live run" below.

## Request set

* Every route: `/`, `/api/v1/{stats,testaments,health,books,random,verse-of-the-day,search}`,
  `/api/v1/books/:book`, `/:book/chapters`, `/:book/:chapter`, `/:book/:chapter/:verse`.
* Every book (meta, chapter list, lookup by display name), every chapter,
  verse 1, range 2-5, last verse + 1; with `--all-verses` every single verse.
* Book-name resolution: ~40 aliases incl. case, spaces, `%20`, NBSP, zero-width
  space, Kelvin sign, curly quotes, unknown books.
* Psalm numbering: CPDV uses the Vulgate numbering and `/api/v1` does no
  conversion (Ps 22 = "The Lord is my shepherd"; 9:22-39 = Hebrew 10; 113:9 =
  Hebrew 115; 151 is 404). Conversion helpers live in `crates/bible/src/psalms.rs`.
* Non-numeric chapters: Sirach and Lamentations have a prologue chapter `"P"`,
  listed first, serialized as a string; numeric routes can't address it (`P` ->
  400 "Invalid chapter number"), exactly as in Node.
* Bad input: unknown book/chapter/verse, `abc`, `0`, `-1`, `-0`, `01`, `1abc`,
  `1.5`, `+1`, `1e3`, `0x10`, 30- and 400-digit numbers, reversed / partial /
  out-of-range verse ranges, malformed percent escapes (500 `Failed to decode
  param`), double slashes, trailing slashes, upper-case paths, extra segments.
* Search: empty / whitespace / missing `q`, punctuation-only, unicode (Greek,
  emoji, `Señor`, `İsrael`, U+2028), repeated and bracketed params (`q[]`,
  `limit[a]` -> the same 500s Node gives), `limit`/`offset` clamping and
  parsing (`0`, `-5`, `1000`, `5.9`, `1e3`, huge), book and testament filters
  incl. unknown/empty/repeated values, pagination past the end.
* Auth: missing / empty / wrong / trailing-space key, second key, public paths
  (`/`, `/health`, `/docs*`) vs near misses (`/DOCS`, `/docsx`, `/health/`).
* CORS: allowed, disallowed (500 `Not allowed by CORS`), wildcard subdomain,
  bare wildcard domain, `null`, empty Origin, preflights allowed/disallowed,
  OPTIONS without Origin, `*` mode.
* Methods and caching: HEAD, POST/PUT/DELETE (404), `If-None-Match` (304),
  `Cache-Control: no-cache`, `*`, `If-Modified-Since`.
* `/docs`: redirect, page, `swagger-ui-init.js` (embeds the spec), assets,
  `package.json` 404, unknown files.

## Compared vs ignored

Compared byte-for-byte: status, body, and all headers except those below.

Ignored as nondeterministic or transport-level (not part of the contract):

| what | why |
|------|-----|
| `date`, `connection`, `keep-alive`, `transfer-encoding`, `server`, `x-railway-*`, `x-hikari-*` | set by the HTTP stack / Railway edge, not the app |
| `ratelimit-reset` | seconds left in the window; depends on when each server saw the client first. Compared within ±2 s locally, presence only live |
| `/api/v1/health` `data.uptime` (+ its `etag`, `content-length`) | process uptime. Shape and `status:"healthy"` are compared |
| `/api/v1/random` body (+ `etag`, `content-length`) | uniform random pick in both. Checked instead: same shape and key order, and the verse equals the Node server's verse at that reference |
| live only: `ratelimit-remaining` | each deployment has its own counters, and Railway's edge spreads one client over many peer IPs |

## Explained differences

| id | difference | impact |
|----|-----------|--------|
| E1 | Reduced `/docs`: Node serves every file in `swagger-ui-dist`; Rust embeds only the 5 files the page loads (css, bundle, preset, 2 favicons). Other dist files (e.g. `/docs/index.css`) return the Swagger HTML page in Rust (Node's fallback for unknown files). Embedded assets have no `Last-Modified`/`Accept-Ranges`/Range support and a content-based ETag instead of an mtime-based one. | Docs page renders the same (same HTML, same spec). No API impact. |
| E2 | `204` responses (CORS preflights): Node sends `Content-Length: 0`, hyper omits it (RFC 9110 §8.6 forbids it on 204). | None. |

Everything else is identical, including the Node quirks callers might rely
on: errors as 500 with Node's exact messages (`q.trim is not a function`,
`input.toLowerCase is not a function`, `Failed to decode param '%E0'`),
`JSON.stringify` number formatting (`1e+21`, `null` for Infinity), Express
weak ETags (`W/"<len>-<sha1>"`) and 304 handling, verse-of-the-day hashing,
the inverted-index tokenizer (`[^\w\s]` stripping on lower-cased text), and
snippets cut on UTF-16 code units.

## Results

See `docs/CUTOVER.md` for the latest local and live numbers.
