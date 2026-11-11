//! CORS with the semantics of the `cors` package as configured in app.ts.
//!
//! tower-http's `CorsLayer` is not used on purpose: it cannot reproduce the
//! Node behavior callers can observe (every OPTIONS is answered as a preflight
//! with 204; a disallowed Origin gets HTTP 500 `Not allowed by CORS`; allowed
//! request headers are reflected; `Access-Control-Allow-Credentials: true`
//! even in `*` mode).

use axum::http::{HeaderMap, HeaderName, HeaderValue};

pub const ALLOW_METHODS: &str = "GET,HEAD,PUT,PATCH,POST,DELETE";

/// `allowed === origin || (allowed.startsWith('*.') && origin.endsWith(allowed.slice(1)))`
pub fn origin_allowed(allowlist: &[String], origin: &str) -> bool {
    allowlist
        .iter()
        .any(|a| a == origin || (a.starts_with("*.") && origin.ends_with(&a[1..])))
}

pub enum Cors {
    /// Normal request; headers were added.
    Continue,
    /// OPTIONS: answer 204 now with the headers added.
    Preflight,
    /// Origin not in the allowlist: Express error handler -> 500.
    Rejected,
}

fn vary(h: &mut HeaderMap, field: &str) {
    let cur = h
        .get("vary")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let mut parts: Vec<String> = cur
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    if !parts
        .iter()
        .any(|p| p.eq_ignore_ascii_case(field) || p == "*")
    {
        parts.push(field.to_owned());
    }
    h.insert(
        HeaderName::from_static("vary"),
        HeaderValue::from_str(&parts.join(", ")).expect("vary"),
    );
}

fn set(h: &mut HeaderMap, k: &'static str, v: &str) {
    if let Ok(v) = HeaderValue::from_str(v) {
        h.insert(HeaderName::from_static(k), v);
    }
}

/// `origin`: the Origin header (None when absent). `req_headers`: the
/// Access-Control-Request-Headers value.
pub fn apply(
    allowlist: &[String],
    is_options: bool,
    origin: Option<&str>,
    req_headers: Option<&str>,
    out: &mut HeaderMap,
) -> Cors {
    // Resolve the origin option: '*' when no allowlist, else the callback.
    let wildcard = allowlist.is_empty();
    let reflect: Option<&str> = if wildcard {
        None
    } else {
        match origin.filter(|o| !o.is_empty()) {
            None => None, // no Origin: allowed, nothing to reflect
            Some(o) if origin_allowed(allowlist, o) => Some(o),
            Some(_) => return Cors::Rejected,
        }
    };

    if wildcard {
        set(out, "access-control-allow-origin", "*");
    } else {
        if let Some(o) = reflect {
            set(out, "access-control-allow-origin", o);
        }
        vary(out, "Origin");
    }
    set(out, "access-control-allow-credentials", "true");
    if !is_options {
        return Cors::Continue;
    }
    set(out, "access-control-allow-methods", ALLOW_METHODS);
    vary(out, "Access-Control-Request-Headers");
    if let Some(rh) = req_headers.filter(|s| !s.is_empty()) {
        set(out, "access-control-allow-headers", rh);
    }
    Cors::Preflight
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> Vec<String> {
        vec!["https://biblehodl.com".into(), "*.example.org".into()]
    }

    #[test]
    fn allowlist_semantics() {
        assert!(origin_allowed(&list(), "https://biblehodl.com"));
        assert!(!origin_allowed(&list(), "https://app.biblehodl.com"));
        assert!(origin_allowed(&list(), "https://a.example.org"));
        assert!(origin_allowed(&list(), "http://a.b.example.org"));
        assert!(!origin_allowed(&list(), "https://example.org"));
        assert!(!origin_allowed(&list(), "https://evilexample.org"));
    }

    #[test]
    fn preflight_headers() {
        let mut h = HeaderMap::new();
        let r = apply(
            &list(),
            true,
            Some("https://biblehodl.com"),
            Some("x-api-key"),
            &mut h,
        );
        assert!(matches!(r, Cors::Preflight));
        assert_eq!(h["vary"], "Origin, Access-Control-Request-Headers");
        assert_eq!(h["access-control-allow-headers"], "x-api-key");
        assert_eq!(h["access-control-allow-origin"], "https://biblehodl.com");
        let mut h = HeaderMap::new();
        assert!(matches!(
            apply(&list(), false, Some("https://x.com"), None, &mut h),
            Cors::Rejected
        ));
        let mut h = HeaderMap::new();
        assert!(matches!(
            apply(&[], false, Some("https://x.com"), None, &mut h),
            Cors::Continue
        ));
        assert_eq!(h["access-control-allow-origin"], "*");
        assert!(h.get("vary").is_none());
    }
}
