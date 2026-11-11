//! The request pipeline, in the same order as `src/app.ts`:
//! helmet -> cors -> (json) -> rate limit -> API key -> /docs -> routers -> 404,
//! with Express's error handler semantics for thrown errors.
//!
//! Routing is done by hand inside one Axum handler instead of with
//! `axum::Router`, because callers can observe Express routing details that
//! matchit does not reproduce: case-insensitive paths, an optional trailing
//! slash, percent-decoded params with a 500 on malformed escapes, and the
//! middleware order (auth runs before routing, so unknown paths need a key).

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, Response};
use serde::Serialize;

use cpdv_bible::js::{self, decode_uri_component, number_to_string, parse_int, truthy};
use cpdv_bible::{display_name, BibleStore, ChapterKey, Verse};

use crate::config::Config;
use crate::cors::{self, Cors};
use crate::docs;
use crate::qs::{self, QVal};
use crate::ratelimit::RateLimiter;
use crate::respond::{finalize, helmet_headers, JsNum, Out};

pub struct AppState {
    pub store: BibleStore,
    pub cfg: Config,
    pub limiter: RateLimiter,
    pub started: Instant,
}

/// Node joins repeated request headers with ", " and reads bytes as latin1.
fn header_joined(h: &HeaderMap, name: &str) -> Option<String> {
    let vals: Vec<String> = h
        .get_all(name)
        .iter()
        .map(|v| v.as_bytes().iter().map(|&b| b as char).collect())
        .collect();
    if vals.is_empty() {
        None
    } else {
        Some(vals.join(", "))
    }
}

fn set(h: &mut HeaderMap, k: &'static str, v: String) {
    if let Ok(v) = HeaderValue::from_str(&v) {
        h.insert(HeaderName::from_static(k), v);
    }
}

/// Paths exempt from the API key (apiKeyAuth.ts). Case-sensitive, raw path.
pub fn is_public(path: &str) -> bool {
    path == "/" || path == "/health" || path == "/docs" || path.starts_with("/docs/")
}

pub async fn handle(State(st): State<Arc<AppState>>, req: Request) -> Response<Body> {
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip().to_canonical().to_string());
    let method = req.method().clone();
    let headers = req.headers().clone();
    let path = req.uri().path().to_owned();
    let query = req.uri().query().map(str::to_owned);
    pipeline(&st, &method, &headers, &path, query.as_deref(), peer)
}

pub fn pipeline(
    st: &AppState,
    method: &Method,
    headers: &HeaderMap,
    path: &str,
    query: Option<&str>,
    peer: Option<String>,
) -> Response<Body> {
    let mut base = helmet_headers();

    // --- cors -------------------------------------------------------------
    let origin = header_joined(headers, "origin");
    let acrh = header_joined(headers, "access-control-request-headers");
    match cors::apply(
        &st.cfg.cors_origins,
        method == Method::OPTIONS,
        origin.as_deref(),
        acrh.as_deref(),
        &mut base,
    ) {
        Cors::Rejected => {
            return finalize(
                method,
                headers,
                base,
                Out::error(500, "Not allowed by CORS"),
            )
        }
        Cors::Preflight => {
            let mut out = Out::new(204, "text/plain", Vec::new());
            out.headers.remove(header::CONTENT_TYPE);
            out.headers
                .insert(header::CONTENT_LENGTH, HeaderValue::from_static("0"));
            out.etag = false;
            return finalize(method, headers, base, out);
        }
        Cors::Continue => {}
    }

    // --- rate limit (express-rate-limit, draft-6 headers) ------------------
    let key = client_key(st, headers, peer);
    let hit = st.limiter.hit(&key);
    set(
        &mut base,
        "ratelimit-policy",
        format!("{};w={}", hit.limit, hit.window_secs),
    );
    set(&mut base, "ratelimit-limit", hit.limit.to_string());
    set(&mut base, "ratelimit-remaining", hit.remaining.to_string());
    if hit.reset_secs > 0 {
        set(&mut base, "ratelimit-reset", hit.reset_secs.to_string());
    }
    if hit.exceeded {
        set(&mut base, "retry-after", hit.reset_secs.to_string());
        let out = Out::error(429, "Too many requests, please try again later");
        return finalize(method, headers, base, out);
    }

    // --- API key -------------------------------------------------------------
    if !st.cfg.api_keys.is_empty() && !is_public(path) {
        match header_joined(headers, "x-api-key") {
            None => return finalize(method, headers, base, Out::error(401, MISSING_KEY)),
            Some(k) if k.is_empty() => {
                return finalize(method, headers, base, Out::error(401, MISSING_KEY))
            }
            Some(k) if !st.cfg.api_keys.contains(&k) => {
                return finalize(method, headers, base, Out::error(403, "Invalid API key."))
            }
            Some(_) => {}
        }
    }

    let out = route(st, method, path, query);
    finalize(method, headers, base, out)
}

const MISSING_KEY: &str = "Missing API key. Provide a valid key via the X-API-Key header.";

fn client_key(st: &AppState, headers: &HeaderMap, peer: Option<String>) -> String {
    if st.cfg.trust_proxy {
        if let Some(xff) = header_joined(headers, "x-forwarded-for") {
            if let Some(first) = xff.split(',').map(str::trim).find(|s| !s.is_empty()) {
                return first.to_owned();
            }
        }
    }
    peer.unwrap_or_else(|| "unknown".into())
}

// ---- Express-style matching ---------------------------------------------------

/// `app.use(prefix, ...)`: case-insensitive prefix ending at "/" or the end;
/// returns the inner URL path the mounted router sees.
pub fn mount<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    let n = prefix.len();
    if path.len() < n || !path.as_bytes()[..n].eq_ignore_ascii_case(prefix.as_bytes()) {
        return None;
    }
    let rest = &path[n..];
    if rest.is_empty() {
        return Some("/");
    }
    if !rest.starts_with('/') {
        return None;
    }
    let after = &rest[1..];
    if after.is_empty() {
        Some("/")
    } else if after.starts_with('/') {
        Some(after)
    } else {
        Some(rest)
    }
}

#[derive(Clone, Copy)]
enum Seg {
    Lit(&'static str),
    Param,
}

/// path-to-regexp 0.1 with `sensitive: false, strict: false, end: true`.
fn match_route<'a>(pat: &[Seg], path: &'a str) -> Option<Vec<&'a str>> {
    let p = path.strip_prefix('/')?;
    if pat.is_empty() {
        return p.is_empty().then(Vec::new);
    }
    let p = p.strip_suffix('/').unwrap_or(p);
    let segs: Vec<&str> = p.split('/').collect();
    if segs.len() != pat.len() {
        return None;
    }
    let mut params = Vec::new();
    for (s, want) in segs.iter().zip(pat) {
        match want {
            Seg::Lit(l) if s.eq_ignore_ascii_case(l) => {}
            Seg::Lit(_) => return None,
            Seg::Param if s.is_empty() => return None,
            Seg::Param => params.push(*s),
        }
    }
    Some(params)
}

/// Walk a router's routes like Express `router.handle`. Returns
/// `Some(Ok((route index, decoded params)))`, `Some(Err(message))` for a
/// param decode error (Express forwards it to the error handler, 500), or
/// `None` to fall through to the next middleware.
fn dispatch(
    routes: &[&[Seg]],
    path: &str,
    is_get: bool,
) -> Option<Result<(usize, Vec<String>), String>> {
    let mut layer_err: Option<String> = None;
    for (i, pat) in routes.iter().enumerate() {
        let Some(raw) = match_route(pat, path) else {
            continue;
        };
        let mut params = Vec::with_capacity(raw.len());
        let mut failed = false;
        for r in raw {
            match decode_uri_component(r) {
                Some(d) => params.push(d),
                None => {
                    layer_err.get_or_insert_with(|| format!("Failed to decode param '{r}'"));
                    failed = true;
                    break;
                }
            }
        }
        if failed || layer_err.is_some() || !is_get {
            continue;
        }
        return Some(Ok((i, params)));
    }
    layer_err.map(Err)
}

use Seg::{Lit, Param};
const BOOK_ROUTES: [&[Seg]; 5] = [
    &[],
    &[Param],
    &[Param, Lit("chapters")],
    &[Param, Param],
    &[Param, Param, Param],
];
const V1_ROUTES: [&[Seg]; 6] = [
    &[Lit("search")],
    &[Lit("random")],
    &[Lit("verse-of-the-day")],
    &[Lit("testaments")],
    &[Lit("stats")],
    &[Lit("health")],
];

fn route(st: &AppState, method: &Method, path: &str, query: Option<&str>) -> Out {
    let is_get = method == Method::GET || method == Method::HEAD;

    if let Some(inner) = mount(path, "/docs") {
        return docs::serve(method, inner, path, query);
    }
    if let Some(inner) = mount(path, "/api/v1/books") {
        match dispatch(&BOOK_ROUTES, inner, is_get) {
            Some(Ok((i, p))) => return books(st, i, &p),
            Some(Err(e)) => return Out::error(500, &e),
            None => {}
        }
    }
    if let Some(inner) = mount(path, "/api/v1") {
        match dispatch(&V1_ROUTES, inner, is_get) {
            Some(Ok((i, _))) => return v1(st, i, query),
            Some(Err(e)) => return Out::error(500, &e),
            None => {}
        }
    }
    if is_get && match_route(&[], path).is_some() {
        return Out::json(
            200,
            &serde_json::json!({
                "status": "ok",
                "data": { "name": "CPDV Bible API", "version": "1.0.0", "docs": "/docs" }
            }),
        );
    }
    Out::error(404, "Not found")
}

// ---- handlers -----------------------------------------------------------------

#[derive(Serialize)]
struct VerseOut<'a> {
    book: String,
    chapter: &'a ChapterKey,
    verse: u32,
    text: &'a str,
}

fn verse_out(v: &Verse) -> VerseOut<'_> {
    VerseOut {
        book: display_name(v.book),
        chapter: &v.chapter,
        verse: v.verse,
        text: &v.text,
    }
}

fn books(st: &AppState, route: usize, p: &[String]) -> Out {
    let s = &st.store;
    match route {
        0 => Out::ok(s.books()),
        1 => match s.book(&p[0]) {
            Some(m) => Out::ok(m),
            None => Out::error(404, &format!("Book not found: {}", p[0])),
        },
        2 => match s.chapters(&p[0]) {
            Some(c) => Out::ok(c),
            None => Out::error(404, &format!("Book not found: {}", p[0])),
        },
        3 => {
            let ch = parse_int(&p[1]);
            if ch.is_nan() {
                return Out::error(400, "Invalid chapter number");
            }
            match s.chapter_verses(&p[0], ch) {
                Some(vs) => Out::ok(vs.into_iter().map(verse_out).collect::<Vec<_>>()),
                None => Out::error(
                    404,
                    &format!("Chapter not found: {} {}", p[0], number_to_string(ch)),
                ),
            }
        }
        _ => get_verse(st, &p[0], &p[1], &p[2]),
    }
}

/// `/^(\d+)-(\d+)$/`
fn range_match(s: &str) -> Option<(&str, &str)> {
    let (a, b) = s.split_once('-')?;
    let digits = |x: &str| !x.is_empty() && x.bytes().all(|c| c.is_ascii_digit());
    (digits(a) && digits(b)).then_some((a, b))
}

fn get_verse(st: &AppState, book: &str, chapter: &str, verse: &str) -> Out {
    let s = &st.store;
    let ch = parse_int(chapter);
    if ch.is_nan() {
        return Out::error(400, "Invalid chapter number");
    }
    let chs = number_to_string(ch);
    if let Some((a, b)) = range_match(verse) {
        let (start, end) = (parse_int(a), parse_int(b));
        if start > end {
            return Out::error(400, "Start verse must be less than or equal to end verse");
        }
        return match s.verse_range(book, ch, start, end) {
            Some(vs) => Out::ok(vs.into_iter().map(verse_out).collect::<Vec<_>>()),
            None => Out::error(
                404,
                &format!(
                    "Verses not found: {book} {chs}:{}-{}",
                    number_to_string(start),
                    number_to_string(end)
                ),
            ),
        };
    }
    let v = parse_int(verse);
    if v.is_nan() {
        return Out::error(400, "Invalid verse number");
    }
    match s.verse(book, ch, v) {
        Some(found) => Out::ok(verse_out(found)),
        None => Out::error(
            404,
            &format!("Verse not found: {book} {chs}:{}", number_to_string(v)),
        ),
    }
}

#[derive(Serialize)]
struct PageMeta {
    total: usize,
    limit: JsNum,
    offset: JsNum,
}

fn v1(st: &AppState, route: usize, query: Option<&str>) -> Out {
    let s = &st.store;
    match route {
        0 => search(st, query),
        1 => Out::ok(verse_out(s.random_verse(rand::random::<f64>()))),
        2 => {
            use chrono::Datelike;
            let now = chrono::Local::now();
            Out::ok(verse_out(s.verse_of_the_day(
                now.year(),
                now.month(),
                now.day(),
            )))
        }
        3 => Out::ok(s.testaments()),
        4 => Out::ok(s.stats()),
        _ => Out::ok(serde_json::json!({
            "status": "healthy",
            "uptime": JsNum(st.started.elapsed().as_secs_f64()),
        })),
    }
}

fn search(st: &AppState, query: Option<&str>) -> Out {
    let q = match qs::get(query, "q") {
        None => return Out::error(400, "Query parameter \"q\" is required"),
        Some(QVal::Str(q)) => q,
        // Node: `q.trim()` on an array/object throws a TypeError -> 500.
        Some(_) => return Out::error(500, "q.trim is not a function"),
    };
    let q = js::trim(&q).to_owned();
    if q.is_empty() {
        return Out::error(400, "Query parameter \"q\" is required");
    }
    let num = |name: &str| {
        parse_int(&qs::get(query, name).map_or_else(|| "undefined".to_owned(), |v| v.js_string()))
    };
    let l = num("limit");
    // Math.min(Math.max(x, 1), 100); `clamp` would differ only for NaN, which `truthy` already excludes.
    let limit = (if truthy(l) { l } else { 20.0 }).clamp(1.0, 100.0);
    let o = num("offset");
    let offset = (if truthy(o) { o } else { 0.0 }).max(0.0);
    let meta = |total| PageMeta {
        total,
        limit: JsNum(limit),
        offset: JsNum(offset),
    };

    let store = &st.store;
    let Some(mut keys) = store.search_candidates(&q) else {
        return Out::ok_meta(Vec::<()>::new(), meta(0));
    };
    match qs::get(query, "book") {
        Some(QVal::Str(b)) if !b.is_empty() => keys = store.filter_book(keys, &b),
        Some(QVal::Str(_)) | None => {}
        Some(_) => return Out::error(500, "input.toLowerCase is not a function"),
    }
    match qs::get(query, "testament") {
        Some(QVal::Str(t)) if !t.is_empty() => keys = store.filter_testament(keys, &t),
        Some(QVal::Str(_)) | None => {}
        Some(_) => return Out::error(500, "testament.toUpperCase is not a function"),
    }
    let total = keys.len();
    Out::ok_meta(store.page(&keys, &q, limit, offset), meta(total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mounts() {
        assert_eq!(mount("/api/v1/books", "/api/v1/books"), Some("/"));
        assert_eq!(mount("/API/V1/Books/", "/api/v1/books"), Some("/"));
        assert_eq!(mount("/api/v1/books/gen", "/api/v1/books"), Some("/gen"));
        assert_eq!(mount("/api/v1/books//gen", "/api/v1/books"), Some("/gen"));
        assert_eq!(mount("/api/v1/booksx", "/api/v1/books"), None);
        assert_eq!(mount("/docs", "/docs"), Some("/"));
    }

    #[test]
    fn routes() {
        assert_eq!(match_route(&[], "/"), Some(vec![]));
        assert_eq!(match_route(&[], "//"), None);
        assert_eq!(match_route(&[Param], "/gen/"), Some(vec!["gen"]));
        assert_eq!(match_route(&[Param], "/gen//"), None);
        assert_eq!(
            match_route(&[Param, Lit("chapters")], "/gen/CHAPTERS"),
            Some(vec!["gen"])
        );
        assert_eq!(match_route(&[Lit("stats")], "/stats/x"), None);
    }

    #[test]
    fn dispatch_order_and_errors() {
        let r = dispatch(&BOOK_ROUTES, "/gen/chapters", true)
            .unwrap()
            .unwrap();
        assert_eq!(r.0, 2);
        let e = dispatch(&BOOK_ROUTES, "/%E0", true).unwrap().unwrap_err();
        assert_eq!(e, "Failed to decode param '%E0'");
        assert!(dispatch(&BOOK_ROUTES, "/gen", false).is_none());
        assert!(dispatch(&BOOK_ROUTES, "/a/b/c/d", true).is_none());
    }

    #[test]
    fn ranges() {
        assert_eq!(range_match("1-5"), Some(("1", "5")));
        assert_eq!(range_match("1-"), None);
        assert_eq!(range_match("1-2-3"), None);
        assert_eq!(range_match("a-2"), None);
    }

    #[test]
    fn public_paths() {
        for p in ["/", "/health", "/docs", "/docs/", "/docs/x.css"] {
            assert!(is_public(p), "{p}");
        }
        for p in ["/docsx", "/DOCS", "/health/", "//docs", "/api/v1/health"] {
            assert!(!is_public(p), "{p}");
        }
    }
}
