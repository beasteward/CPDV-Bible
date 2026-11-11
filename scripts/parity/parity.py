#!/usr/bin/env python3
"""Parity harness: send the same requests to two CPDV API servers (A = Node
reference, B = Rust candidate) and diff status codes, raw body bytes and
headers.

Usage:
  parity.py --a URL --b URL --profile main|open|ratelimit|live [--key-env NAME]
            [--all-verses] [--report FILE]

The API key is read from the environment variable named by --key-env (never
from the command line) so it is not printed or stored.

IGNORED (nondeterministic), each documented in docs/PARITY.md:
  * headers: date, connection, keep-alive, transfer-encoding, server and the
    Railway edge headers (x-railway-*, x-hikari-*) - transport, not app.
  * ratelimit-reset: seconds until the window resets; depends on when each
    server saw its first request (compared within +-2 s locally).
  * /api/v1/health data.uptime (and so its etag/content-length).
  * /api/v1/random data (and so etag/content-length): both pick uniformly at
    random; checked instead for shape and that the verse is real.
  * live profile only: ratelimit-remaining (each deployment has its own
    counters and Railway's edge spreads clients over several peer IPs).
  * E2: Content-Length on 204 responses (preflights): Node sends "0", hyper
    omits it (RFC 9110 8.6 forbids it on 204). No client impact.
  * /docs static assets: etag/last-modified (serve-static derives them from
    file mtime) - compared by body bytes instead.
"""
import argparse
import http.client
import json
import os
import sys
import time
import urllib.parse
from collections import Counter

HOP = {"date", "connection", "keep-alive", "transfer-encoding", "server"}
EDGE_PREFIX = ("x-railway-", "x-hikari-")

ALL_BOOK_KEYS = []  # filled from server A's /books


class Resp:
    def __init__(self, status, headers, body, ms):
        self.status, self.headers, self.body, self.ms = status, headers, body, ms

    def json(self):
        try:
            return json.loads(self.body)
        except Exception:
            return None


class Client:
    def __init__(self, base):
        u = urllib.parse.urlsplit(base)
        self.https = u.scheme == "https"
        self.host = u.hostname
        self.port = u.port or (443 if self.https else 80)
        self.conn = None

    def _connect(self):
        if self.https:
            # Live: go through the environment's HTTPS proxy if there is one.
            proxy = os.environ.get("HTTPS_PROXY") or os.environ.get("https_proxy")
            if proxy:
                p = urllib.parse.urlsplit(proxy)
                c = http.client.HTTPSConnection(p.hostname, p.port or 80, timeout=60)
                c.set_tunnel(self.host, self.port)
                return c
            return http.client.HTTPSConnection(self.host, self.port, timeout=60)
        return http.client.HTTPConnection(self.host, self.port, timeout=60)

    def req(self, method, path, headers):
        for attempt in range(3):
            if self.conn is None:
                self.conn = self._connect()
            try:
                t = time.perf_counter()
                self.conn.putrequest(method, path, skip_accept_encoding=True)
                for k, v in headers:
                    self.conn.putheader(k, v)
                self.conn.endheaders()
                r = self.conn.getresponse()
                body = r.read()
                ms = (time.perf_counter() - t) * 1000
                hs = [(k.lower(), v) for k, v in r.getheaders()]
                if r.getheader("connection", "").lower() == "close":
                    self.conn.close()
                    self.conn = None
                return Resp(r.status, hs, body, ms)
            except (http.client.HTTPException, OSError):
                self.conn.close()
                self.conn = None
                if attempt == 2:
                    raise
                time.sleep(0.2)


def norm_headers(hs, live, static_asset, status=None):
    out = {}
    for k, v in hs:
        # E2: hyper never sends Content-Length on 204 (RFC 9110 8.6); Node sends "0".
        if status == 204 and k == "content-length":
            continue
        if k in HOP or k.startswith(EDGE_PREFIX):
            continue
        if live and k == "ratelimit-remaining":
            continue
        # E1 (reduced docs): no Range support on embedded assets, so no Accept-Ranges.
        if static_asset and k in ("etag", "last-modified", "accept-ranges"):
            continue
        out[k] = v if k not in out else out[k] + ", " + v
    return out


# ---------------------------------------------------------------------------
# request set

def H(**kw):
    return kw


def cases(profile, books, chapters_of, all_verses):
    """Yield (name, method, path, headers, flags)."""
    live = profile == "live"
    K = [("X-API-Key", "$KEY")] if profile in ("main", "live") else []
    O = [("Origin", "https://biblehodl.com")]

    def g(name, path, hdrs=None, method="GET", **flags):
        return (name, method, path, (hdrs if hdrs is not None else K), flags)

    if profile == "ratelimit":
        for i in range(5):
            yield g(f"ratelimit #{i+1} stats", "/api/v1/stats")
        yield g("ratelimit after limit: root", "/")
        yield g("ratelimit after limit: preflight not counted", "/api/v1/stats",
                O + [("Access-Control-Request-Method", "GET")], method="OPTIONS")
        return

    # --- meta / root -----------------------------------------------------
    yield g("root", "/")
    yield g("root without key", "/", [])
    yield g("stats", "/api/v1/stats")
    yield g("testaments", "/api/v1/testaments")
    yield g("health", "/api/v1/health", ignore="uptime")
    yield g("books list", "/api/v1/books")
    yield g("books list trailing slash", "/api/v1/books/")
    yield g("verse of the day", "/api/v1/verse-of-the-day")
    yield g("random #1", "/api/v1/random", ignore="random")
    yield g("random #2", "/api/v1/random", ignore="random")

    # --- every book: meta, chapter list, display-name + alias -------------
    for b in books:
        key = b["key"]
        yield g(f"book {key}", f"/api/v1/books/{urllib.parse.quote(key)}")
        yield g(f"chapters {key}", f"/api/v1/books/{urllib.parse.quote(key)}/chapters")
        if not live:
            yield g(f"book by name {b['name']}", "/api/v1/books/" + urllib.parse.quote(b["name"]))

    # --- every chapter of every book (local), sample (live) ----------------
    for b in books:
        key = b["key"]
        chs = chapters_of[key]
        sample = chs if not live else [chs[0], chs[-1]]
        for c in sample:
            ch = c["chapter"]
            q = urllib.parse.quote(str(ch))
            yield g(f"chapter {key} {ch}", f"/api/v1/books/{key}/{q}")
            if not live:
                yield g(f"verse {key} {ch}:1", f"/api/v1/books/{key}/{q}/1")
                yield g(f"range {key} {ch}:2-5", f"/api/v1/books/{key}/{q}/2-5")
                yield g(f"last verse+1 {key} {ch}", f"/api/v1/books/{key}/{q}/{c['verseCount'] + 1}")
                if all_verses:
                    for v in range(2, c["verseCount"] + 1):
                        yield g(f"verse {key} {ch}:{v}", f"/api/v1/books/{key}/{q}/{v}")
        if live:
            yield g(f"verse {key} 1:1", f"/api/v1/books/{key}/1/1")

    # --- aliases / book-name resolution -----------------------------------
    for alias in ["gen", "GEN", "Gen", "1sam", "1 Samuel", "1%20Samuel", "1-samuel", "sam", "ps", "psalm",
                  "Psalms", "song", "Song%20of%20Songs", "songofsolomon", "canticles", "sos", "sir",
                  "ecclesiasticus", "jn", "1jn", "1%20John", "3john", "apocalypse", "rev", "php", "phm",
                  "qoheleth", "wisdomofsolomon", "eze", "jon", "joh", "mt", "Nope", "Gen%C3%A8sis",
                  "%E2%80%9CGenesis%E2%80%9D", "Gen%C2%A0esis", "Gen%E2%80%8Besis", "1%E2%84%AAgs", "Song2"]:
        a = urllib.parse.quote(alias, safe="%")
        yield g(f"alias {alias}", f"/api/v1/books/{a}")
        if not live:
            yield g(f"alias verse {alias} 1:1", f"/api/v1/books/{a}/1/1")

    # --- psalm numbering (CPDV = Vulgate numbering, no conversion in /api/v1)
    for p in ["/api/v1/books/ps/22", "/api/v1/books/ps/22/1", "/api/v1/books/ps/23/1",
              "/api/v1/books/psalms/9/22", "/api/v1/books/psalms/9/39", "/api/v1/books/psalms/9/40",
              "/api/v1/books/psalms/10", "/api/v1/books/psalms/113/9", "/api/v1/books/psalms/113/26",
              "/api/v1/books/psalms/114", "/api/v1/books/psalms/115/1", "/api/v1/books/psalms/146",
              "/api/v1/books/psalms/147/1", "/api/v1/books/psalms/150/6", "/api/v1/books/psalms/151",
              "/api/v1/books/psalms/151/1", "/api/v1/books/psalms/0"]:
        yield g(f"psalm {p}", p)

    # --- non-numeric chapters (Sirach/Lamentations "P") --------------------
    for p in ["/api/v1/books/sirach/chapters", "/api/v1/books/lamentations/chapters",
              "/api/v1/books/sirach/P", "/api/v1/books/sirach/p", "/api/v1/books/sirach/P/1",
              "/api/v1/books/lamentations/P/1-3", "/api/v1/books/sirach/1", "/api/v1/books/lamentations/1/1"]:
        yield g(f"prologue {p}", p)

    # --- bad book / chapter / verse / ranges -----------------------------
    for p in ["/api/v1/books/nope", "/api/v1/books/nope/chapters", "/api/v1/books/nope/1",
              "/api/v1/books/nope/1/1", "/api/v1/books/nope/1/1-3", "/api/v1/books/gen/abc",
              "/api/v1/books/gen/abc/1", "/api/v1/books/gen/1/abc", "/api/v1/books/gen/0",
              "/api/v1/books/gen/51", "/api/v1/books/gen/-1", "/api/v1/books/gen/-0", "/api/v1/books/gen/01",
              "/api/v1/books/gen/1abc", "/api/v1/books/gen/1.5", "/api/v1/books/gen/%201",
              "/api/v1/books/gen/+1", "/api/v1/books/gen/1e3", "/api/v1/books/gen/0x10",
              "/api/v1/books/gen/" + "9" * 30, "/api/v1/books/gen/" + "9" * 400,
              "/api/v1/books/gen/1/0", "/api/v1/books/gen/1/32", "/api/v1/books/gen/1/01",
              "/api/v1/books/gen/1/1.9", "/api/v1/books/gen/1/-1", "/api/v1/books/gen/1/" + "9" * 30,
              "/api/v1/books/gen/1/1-5", "/api/v1/books/gen/1/5-1", "/api/v1/books/gen/1/5-5",
              "/api/v1/books/gen/1/30-40", "/api/v1/books/gen/1/32-40", "/api/v1/books/gen/1/0-0",
              "/api/v1/books/gen/1/01-03", "/api/v1/books/gen/1/1-", "/api/v1/books/gen/1/-5",
              "/api/v1/books/gen/1/1-2-3", "/api/v1/books/gen/1/a-b", "/api/v1/books/gen/1/1-31",
              "/api/v1/books/gen/99/1-3", "/api/v1/books/gen/1/1,2",
              "/api/v1/books/gen/1/1/extra", "/api/v1/books/gen/chapters/1",
              "/api/v1/books/gen/CHAPTERS", "/api/v1/books/gen/chapters/", "/api/v1/books/%E0",
              "/api/v1/books/gen/%E0", "/api/v1/books/gen/1/%E0", "/api/v1/books/%E0/chapters",
              "/api/v1/books/gen%2F1", "/api/v1/books//gen", "/api/v1/books/gen//1",
              "/api/v1/books/gen/1/", "/api/v1/books/gen/1/1/", "/API/V1/BOOKS/GEN/1/1",
              "/Api/V1/Stats", "/api/v1/stats/", "/api/v1//stats", "/api/v1/stats/x", "/api/v1",
              "/api/v1/", "/api", "/nope", "/api/v2/stats", "//", "/health", "/index.html",
              "/api/v1/stats?x=%E0", "/api/v1/books/gen/1/1?foo=bar"]:
        if live and "9" * 400 in p:
            continue
        yield g(f"edge {p[:80]}", p)

    # --- search -------------------------------------------------------------
    qs = ["q=light", "q=Light", "q=let%20there%20be%20light", "q=let+there+be+light", "q=God%20created",
          "q=jesus", "q=lords", "q=lord%27s", "q=%E2%80%9Cthe", "q=the", "q=zzzzzz", "q=light%20zzzz",
          "q=", "q=%20%20", "q", "", "x=1", "q=%21%21%21", "q=Se%C3%B1or", "q=%E2%84%AAing", "q=%E0",
          "q=a&q=b", "q[]=light", "q[x]=light", "q=light%00", "q=light&limit=5", "q=light&limit=0",
          "q=light&limit=-5", "q=light&limit=1000", "q=light&limit=abc", "q=light&limit=5.9",
          "q=light&limit=1e3", "q=light&limit=%205", "q=light&limit=5&limit=7", "q=light&limit[]=3",
          "q=light&limit[a]=3", "q=light&offset=10", "q=light&offset=-3", "q=light&offset=abc",
          "q=light&offset=100000", "q=light&offset=" + "9" * 400, "q=light&limit=" + "9" * 400,
          "q=light&offset=5&limit=3", "q=light&book=john", "q=light&book=John", "q=light&book=1%20John",
          "q=light&book=nope", "q=light&book=", "q=light&book=jn&book=gen", "q=zzzz&book=jn&book=gen",
          "q=light&testament=OT", "q=light&testament=nt", "q=light&testament=xx", "q=light&testament=",
          "q=light&testament=nt&testament=ot", "q=light&book=gen&testament=NT", "q=light&book=gen&testament=ot",
          "q=and", "q=and&offset=19990&limit=100", "q=in%20the%20beginning", "q=beginning%2C%20God",
          "q=The%20LORD%20is%20my%20shepherd", "q=spirit&testament=NT&limit=100&offset=50",
          "q=%CE%A3%CE%BF%CF%86%CE%AF%CE%B1", "q=%F0%9F%99%8F", "q=light%E2%80%A8dark",
          "q=%C4%B0srael", "q=israel", "q=light&q=dark", "q=_", "q=1", "q=7"]
    for s in qs:
        if live and ("9" * 400 in s):
            continue
        yield g(f"search {s[:60]}", "/api/v1/search" + ("?" + s if s else ""))

    # --- auth ---------------------------------------------------------------
    if profile in ("main", "live"):
        yield g("auth missing on stats", "/api/v1/stats", [])
        yield g("auth empty on stats", "/api/v1/stats", [("X-API-Key", "")])
        yield g("auth wrong on stats", "/api/v1/stats", [("X-API-Key", "wrong")])
        yield g("auth wrong case header name", "/api/v1/stats", [("x-api-key", "$KEY")])
        yield g("auth key with space", "/api/v1/stats", [("X-API-Key", "$KEY ")])
        yield g("auth missing on unknown path", "/nope", [])
        yield g("auth missing on /api/v1/health", "/api/v1/health", [])
        yield g("auth missing on /health (public, then 404)", "/health", [])
        yield g("auth missing on /docs", "/docs", [])
        yield g("auth missing on /docs/", "/docs/", [])
        yield g("auth missing on /DOCS (case)", "/DOCS", [])
        yield g("auth missing on /docsx", "/docsx", [])
        yield g("auth missing on /health/", "/health/", [])
        yield g("auth missing on books", "/api/v1/books", [])
        yield g("auth missing on search", "/api/v1/search?q=light", [])
        yield g("auth on root with wrong key", "/", [("X-API-Key", "wrong")])
        if not live:
            yield g("auth second key", "/api/v1/stats", [("X-API-Key", "$KEY2")])

    # --- CORS -----------------------------------------------------------------
    yield g("cors allowed origin", "/api/v1/stats", K + O)
    yield g("cors allowed origin on 404", "/nope", K + O)
    yield g("cors disallowed origin", "/api/v1/stats", K + [("Origin", "https://evil.example")])
    yield g("cors disallowed subdomain", "/api/v1/stats", K + [("Origin", "https://app.biblehodl.com")])
    yield g("cors disallowed no key", "/api/v1/stats", [("Origin", "https://evil.example")])
    yield g("cors origin null", "/api/v1/stats", K + [("Origin", "null")])
    yield g("cors empty origin", "/api/v1/stats", K + [("Origin", "")])
    if not live:
        yield g("cors wildcard subdomain", "/api/v1/stats", K + [("Origin", "https://a.example.org")])
        yield g("cors wildcard bare domain", "/api/v1/stats", K + [("Origin", "https://example.org")])
    yield g("preflight allowed", "/api/v1/stats",
            O + [("Access-Control-Request-Method", "GET"), ("Access-Control-Request-Headers", "x-api-key")],
            method="OPTIONS")
    yield g("preflight allowed no request headers", "/api/v1/books/gen",
            O + [("Access-Control-Request-Method", "GET")], method="OPTIONS")
    yield g("preflight disallowed", "/api/v1/stats",
            [("Origin", "https://evil.example"), ("Access-Control-Request-Method", "GET")], method="OPTIONS")
    yield g("options without origin", "/api/v1/stats", [], method="OPTIONS")
    yield g("options unknown path", "/nope", O, method="OPTIONS")

    # --- methods / caching ------------------------------------------------
    yield g("HEAD stats", "/api/v1/stats", method="HEAD")
    yield g("HEAD chapter", "/api/v1/books/gen/1", method="HEAD")
    yield g("POST stats", "/api/v1/stats", method="POST")
    yield g("PUT book", "/api/v1/books/gen", method="PUT")
    yield g("DELETE root", "/", method="DELETE")
    yield g("POST root no key", "/", [], method="POST")
    yield g("POST bad decode", "/api/v1/books/%E0", method="POST")
    yield g("conditional GET (etag)", "/api/v1/stats", K + [("If-None-Match", "$ETAG:/api/v1/stats")])
    yield g("conditional GET (wrong etag)", "/api/v1/stats", K + [("If-None-Match", 'W/"nope"')])
    yield g("conditional GET (no-cache)", "/api/v1/stats",
            K + [("If-None-Match", "$ETAG:/api/v1/stats"), ("Cache-Control", "no-cache")])
    yield g("conditional GET (*)", "/api/v1/stats", K + [("If-None-Match", "*")])
    yield g("conditional 404", "/api/v1/books/nope", K + [("If-None-Match", "$ETAG:/api/v1/books/nope")])
    yield g("if-modified-since", "/api/v1/stats", K + [("If-Modified-Since", "Wed, 11 Nov 2099 00:00:00 GMT")])

    # --- docs -------------------------------------------------------------
    yield g("docs redirect", "/docs", [])
    yield g("docs redirect with query", "/docs?a=1", [])
    yield g("docs page", "/docs/", [])
    yield g("docs init js", "/docs/swagger-ui-init.js", [])
    yield g("docs package.json", "/docs/package.json", [])
    yield g("docs unknown asset -> page", "/docs/nope.js", [])
    yield g("docs POST -> page", "/docs/", [], method="POST")
    yield g("docs DOCS case", "/DOCS/", K)
    for a in ["swagger-ui.css", "swagger-ui-bundle.js", "swagger-ui-standalone-preset.js",
              "favicon-16x16.png", "favicon-32x32.png"]:
        yield g(f"docs asset {a}", f"/docs/{a}", [], static=True)
    yield g("docs asset HEAD", "/docs/swagger-ui.css", [], method="HEAD", static=True)
    # Reduced docs: Node serves every swagger-ui-dist file; Rust only the five
    # the page loads (explained diff E1).
    yield g("docs other dist file", "/docs/index.css", [], static=True, explained="E1")


def sub(headers, key, key2, etags):
    out = []
    for k, v in headers:
        if v == "$KEY":
            v = key
        elif v == "$KEY ":
            v = key + " "
        elif v == "$KEY2":
            v = key2
        elif v.startswith("$ETAG:"):
            v = etags.get(v[6:], 'W/"none"')
        out.append((k, v))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--a", required=True)
    ap.add_argument("--b", required=True)
    ap.add_argument("--profile", default="main", choices=["main", "open", "ratelimit", "live"])
    ap.add_argument("--key-env", default="CPDV_KEY")
    ap.add_argument("--key2-env", default="CPDV_KEY2")
    ap.add_argument("--all-verses", action="store_true")
    ap.add_argument("--report")
    ap.add_argument("--pace", type=float, default=0.0, help="seconds between requests (live)")
    ap.add_argument("--min-remaining", type=int, default=0,
                    help="abort if server A's RateLimit-Remaining drops below this (protects prod)")
    args = ap.parse_args()

    key = os.environ.get(args.key_env, "")
    key2 = os.environ.get(args.key2_env, "")
    live = args.profile == "live"
    A, B = Client(args.a), Client(args.b)
    kh = [("X-API-Key", key)] if key else []

    books = json.loads(A.req("GET", "/api/v1/books", kh).body)["data"] if args.profile != "ratelimit" else []
    chapters_of = {}
    for b in books:
        chapters_of[b["key"]] = json.loads(A.req("GET", f"/api/v1/books/{b['key']}/chapters", kh).body)["data"]
    # Mirror the setup requests on B so both rate-limit counters stay in step.
    if args.profile != "ratelimit":
        B.req("GET", "/api/v1/books", kh)
        for b in books:
            B.req("GET", f"/api/v1/books/{b['key']}/chapters", kh)
    etags = {}
    for p in ["/api/v1/stats", "/api/v1/books/nope"]:
        if args.profile != "ratelimit":
            r = A.req("GET", p, kh)
            etags[p] = dict(r.headers).get("etag", "")
            B.req("GET", p, kh)  # keep the rate-limit counters in step

    results, diffs, explained, counts = [], [], [], Counter()
    lat = {"a": [], "b": []}
    for name, method, path, headers, flags in cases(args.profile, books, chapters_of, args.all_verses):
        hs = sub(headers, key, key2, etags)
        ra = A.req(method, path, hs)
        rb = B.req(method, path, hs)
        lat["a"].append(ra.ms)
        lat["b"].append(rb.ms)
        counts["requests"] += 1
        if args.min_remaining:
            rem = dict(ra.headers).get("ratelimit-remaining")
            if rem is not None and int(rem) < args.min_remaining:
                print(f"ABORT: server A ratelimit-remaining={rem} < {args.min_remaining}", file=sys.stderr)
                break
        problems = []
        if ra.status != rb.status:
            problems.append(f"status {ra.status} != {rb.status}")
        ignore = flags.get("ignore")
        static = flags.get("static", False)
        ha = norm_headers(ra.headers, live, static, ra.status)
        hb = norm_headers(rb.headers, live, static, rb.status)
        if ignore:
            for h in (ha, hb):
                h.pop("etag", None)
                h.pop("content-length", None)
            ja, jb = ra.json(), rb.json()
            if ignore == "uptime":
                ok = (isinstance(ja, dict) and isinstance(jb, dict)
                      and ja.get("status") == jb.get("status") == "ok"
                      and set(ja["data"]) == set(jb["data"]) == {"status", "uptime"}
                      and ja["data"]["status"] == jb["data"]["status"] == "healthy"
                      and isinstance(jb["data"]["uptime"], (int, float)))
                if not ok:
                    problems.append("health shape differs")
            elif ignore == "random":
                ok = (isinstance(jb, dict) and jb.get("status") == "ok"
                      and list(jb["data"]) == ["book", "chapter", "verse", "text"])
                if ok:
                    d = jb["data"]
                    bk = urllib.parse.quote(d["book"])
                    ch = urllib.parse.quote(str(d["chapter"]))
                    ref = A.req("GET", f"/api/v1/books/{bk}/{ch}/{d['verse']}", hs).json()
                    B.req("GET", f"/api/v1/books/{bk}/{ch}/{d['verse']}", hs)
                    if isinstance(d["chapter"], str):  # "P": not addressable by number in Node either
                        ref = {"data": d}
                    ok = ref and ref.get("data") == d
                if not ok:
                    problems.append(f"random verse invalid: {rb.body[:200]!r}")
        else:
            if ra.body != rb.body:
                problems.append(f"body differs:\n    A: {ra.body[:300]!r}\n    B: {rb.body[:300]!r}")
        # ratelimit-reset: compare loosely
        for h in (ha, hb):
            h.setdefault("ratelimit-reset", None)
        try:
            ra_r, rb_r = ha.pop("ratelimit-reset"), hb.pop("ratelimit-reset")
            if (ra_r is None) != (rb_r is None) or (ra_r and not live and abs(int(ra_r) - int(rb_r)) > 2):
                problems.append(f"ratelimit-reset {ra_r} vs {rb_r}")
        except ValueError:
            problems.append("ratelimit-reset not an int")
        if ha != hb:
            keys = sorted(set(ha) | set(hb))
            hd = [f"{k}: {ha.get(k)!r} != {hb.get(k)!r}" for k in keys if ha.get(k) != hb.get(k)]
            problems.append("headers: " + "; ".join(hd))
        rec = {"name": name, "method": method, "path": path, "status": [ra.status, rb.status],
               "ms": [round(ra.ms, 2), round(rb.ms, 2)], "problems": problems}
        results.append(rec)
        if problems:
            (explained if flags.get("explained") else diffs).append(rec)
        if args.pace:
            time.sleep(args.pace)

    def pct(xs, p):
        xs = sorted(xs)
        return round(xs[min(len(xs) - 1, int(len(xs) * p))], 2) if xs else None

    summary = {
        "profile": args.profile,
        "requests": counts["requests"],
        "unexplained_diffs": len(diffs),
        "explained_diffs": [{"name": e["name"], "id": "E1"} for e in explained],
        "latency_ms": {s: {"p50": pct(lat[s], .5), "p95": pct(lat[s], .95), "p99": pct(lat[s], .99),
                           "mean": round(sum(lat[s]) / len(lat[s]), 2) if lat[s] else None} for s in lat},
        "status_counts": dict(Counter(r["status"][0] for r in results)),
    }
    print(json.dumps(summary, indent=1))
    for d in diffs[:40]:
        print(f"DIFF {d['method']} {d['path']} ({d['name']})")
        for p in d["problems"]:
            print("   ", p)
    if args.report:
        with open(args.report, "w") as f:
            json.dump({"summary": summary, "diffs": diffs, "explained": explained}, f, indent=1)
    sys.exit(1 if diffs else 0)


if __name__ == "__main__":
    main()
