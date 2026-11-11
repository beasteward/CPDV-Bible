//! Response building with Express semantics: helmet headers, weak ETag
//! (`W/"<len hex>-<sha1 base64 27>"`), conditional GET -> 304, HEAD bodies
//! dropped, `JSON.stringify`-compatible numbers.

use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, Response, StatusCode};
use base64::Engine;
use serde::{Serialize, Serializer};
use sha1::{Digest, Sha1};

use cpdv_bible::js;

/// helmet 7 defaults, exactly as the Node service sends them.
pub const HELMET: [(&str, &str); 12] = [
    (
        "content-security-policy",
        "default-src 'self';base-uri 'self';font-src 'self' https: data:;form-action 'self';frame-ancestors 'self';img-src 'self' data:;object-src 'none';script-src 'self';script-src-attr 'none';style-src 'self' https: 'unsafe-inline';upgrade-insecure-requests",
    ),
    ("cross-origin-opener-policy", "same-origin"),
    ("cross-origin-resource-policy", "same-origin"),
    ("origin-agent-cluster", "?1"),
    ("referrer-policy", "no-referrer"),
    ("strict-transport-security", "max-age=15552000; includeSubDomains"),
    ("x-content-type-options", "nosniff"),
    ("x-dns-prefetch-control", "off"),
    ("x-download-options", "noopen"),
    ("x-frame-options", "SAMEORIGIN"),
    ("x-permitted-cross-domain-policies", "none"),
    ("x-xss-protection", "0"),
];

pub fn helmet_headers() -> HeaderMap {
    let mut h = HeaderMap::with_capacity(24);
    for (k, v) in HELMET {
        h.insert(HeaderName::from_static(k), HeaderValue::from_static(v));
    }
    h
}

pub const JSON_CT: &str = "application/json; charset=utf-8";

/// A response before the common Express post-processing.
pub struct Out {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
    /// Generate an Express weak ETag (res.send does; redirects/204 don't).
    pub etag: bool,
}

impl Out {
    pub fn new(status: u16, content_type: &'static str, body: Vec<u8>) -> Out {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
        Out {
            status: StatusCode::from_u16(status).expect("status"),
            headers,
            body,
            etag: true,
        }
    }

    pub fn json<T: Serialize + ?Sized>(status: u16, v: &T) -> Out {
        Out::new(status, JSON_CT, serde_json::to_vec(v).expect("serialize"))
    }

    pub fn ok<T: Serialize>(data: T) -> Out {
        Out::json(
            200,
            &OkBody {
                status: "ok",
                data,
                meta: None::<()>,
            },
        )
    }

    pub fn ok_meta<T: Serialize, M: Serialize>(data: T, meta: M) -> Out {
        Out::json(
            200,
            &OkBody {
                status: "ok",
                data,
                meta: Some(meta),
            },
        )
    }

    pub fn error(status: u16, msg: &str) -> Out {
        Out::json(
            status,
            &ErrBody {
                status: "error",
                error: msg,
            },
        )
    }
}

#[derive(Serialize)]
struct OkBody<T, M> {
    status: &'static str,
    data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<M>,
}

#[derive(Serialize)]
struct ErrBody<'a> {
    status: &'static str,
    error: &'a str,
}

/// A JS number serialized exactly like `JSON.stringify` (non-finite -> null).
#[derive(Clone, Copy, Debug)]
pub struct JsNum(pub f64);

impl Serialize for JsNum {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if !self.0.is_finite() {
            return s.serialize_none();
        }
        let raw = serde_json::value::RawValue::from_string(js::number_to_string(self.0))
            .map_err(serde::ser::Error::custom)?;
        raw.serialize(s)
    }
}

/// `etag` package, weak form, as used by Express `res.send`.
pub fn express_etag(body: &[u8]) -> String {
    if body.is_empty() {
        return "W/\"0-2jmj7l5rSw0yVb/vlWAYkK/YBwk\"".to_owned();
    }
    let hash = base64::engine::general_purpose::STANDARD.encode(Sha1::digest(body));
    format!("W/\"{:x}-{}\"", body.len(), &hash[..27])
}

/// The `fresh` package (used by `req.fresh`). We never send Last-Modified.
pub fn is_fresh(req: &HeaderMap, etag: &str) -> bool {
    let get = |h: header::HeaderName| req.get(h).and_then(|v| v.to_str().ok()).map(str::to_owned);
    let none_match = get(header::IF_NONE_MATCH).filter(|s| !s.is_empty());
    let modified_since = get(header::IF_MODIFIED_SINCE).filter(|s| !s.is_empty());
    if none_match.is_none() && modified_since.is_none() {
        return false;
    }
    if let Some(cc) = get(header::CACHE_CONTROL) {
        if cc.split(',').any(|t| t.trim() == "no-cache") {
            return false;
        }
    }
    if let Some(nm) = &none_match {
        if nm != "*" {
            let hit = nm
                .split(',')
                .map(str::trim)
                .filter(|m| !m.is_empty())
                .any(|m| m == etag || format!("W/{m}") == etag || m == format!("W/{etag}"));
            if !hit {
                return false;
            }
        }
    }
    // A conditional with If-Modified-Since is stale without Last-Modified.
    modified_since.is_none()
}

/// Apply Express's `res.send` post-processing and build the final response.
pub fn finalize(method: &Method, req: &HeaderMap, mut base: HeaderMap, out: Out) -> Response<Body> {
    let Out {
        mut status,
        headers,
        mut body,
        etag,
    } = out;
    for (k, v) in headers.iter() {
        base.insert(k.clone(), v.clone());
    }
    if etag && !base.contains_key(header::ETAG) {
        let tag = express_etag(&body);
        base.insert(header::ETAG, HeaderValue::from_str(&tag).expect("etag"));
    }
    let cacheable = status.is_success() || status == StatusCode::NOT_MODIFIED;
    let is_get = method == Method::GET || method == Method::HEAD;
    let tag = base
        .get(header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    if is_get && cacheable && tag.as_deref().is_some_and(|t| is_fresh(req, t)) {
        status = StatusCode::NOT_MODIFIED;
        body.clear();
        base.remove(header::CONTENT_TYPE);
        base.remove(header::CONTENT_LENGTH);
        base.remove(header::TRANSFER_ENCODING);
    } else if status != StatusCode::NO_CONTENT {
        base.insert(header::CONTENT_LENGTH, HeaderValue::from(body.len()));
    }
    if method == Method::HEAD {
        body.clear();
    }
    let mut resp = Response::new(Body::from(body));
    *resp.status_mut() = status;
    *resp.headers_mut() = base;
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etag_matches_express() {
        // Express: etag('{"status":"ok"}') with weak: true
        let t = express_etag(br#"{"status":"ok"}"#);
        assert!(t.starts_with("W/\"f-"), "{t}");
        assert_eq!(t.len(), "W/\"f-\"".len() + 27);
    }

    #[test]
    fn js_numbers() {
        let s =
            serde_json::to_string(&[JsNum(20.0), JsNum(1e30), JsNum(f64::INFINITY), JsNum(-0.0)])
                .unwrap();
        assert_eq!(s, "[20,1e+30,null,0]");
    }

    #[test]
    fn freshness() {
        let mut h = HeaderMap::new();
        h.insert(
            header::IF_NONE_MATCH,
            HeaderValue::from_static("W/\"a\", W/\"b\""),
        );
        assert!(is_fresh(&h, "W/\"b\""));
        assert!(!is_fresh(&h, "W/\"c\""));
        h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        assert!(!is_fresh(&h, "W/\"b\""));
    }
}
