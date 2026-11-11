//! `/docs`: Swagger UI, mirroring `swagger-ui-express` mounted at /docs.
//!
//! The HTML page and `swagger-ui-init.js` (which embeds the OpenAPI spec from
//! `src/docs/openapi.ts`) are byte-for-byte captures of the Node service's
//! output; regenerate them with `scripts/capture-docs.sh` when the spec
//! changes. Reduced vs Node: only the swagger-ui-dist files the page loads are
//! embedded (other dist files fall through to the HTML page, as unknown paths
//! do in Node), and static files carry no Last-Modified / Range support.

use axum::http::{header, HeaderValue, Method};

use cpdv_bible::js::decode_uri_component;

use crate::respond::Out;

const INDEX_HTML: &[u8] = include_bytes!("../assets/docs/index.html");
const INIT_JS: &[u8] = include_bytes!("../assets/docs/swagger-ui-init.js");

const ASSETS: [(&str, &str, &[u8]); 5] = [
    (
        "/swagger-ui.css",
        "text/css; charset=UTF-8",
        include_bytes!("../assets/docs/swagger-ui.css"),
    ),
    (
        "/swagger-ui-bundle.js",
        "application/javascript; charset=UTF-8",
        include_bytes!("../assets/docs/swagger-ui-bundle.js"),
    ),
    (
        "/swagger-ui-standalone-preset.js",
        "application/javascript; charset=UTF-8",
        include_bytes!("../assets/docs/swagger-ui-standalone-preset.js"),
    ),
    (
        "/favicon-16x16.png",
        "image/png",
        include_bytes!("../assets/docs/favicon-16x16.png"),
    ),
    (
        "/favicon-32x32.png",
        "image/png",
        include_bytes!("../assets/docs/favicon-32x32.png"),
    ),
];

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// `inner`: path inside the /docs mount; `orig`: the full request path.
pub fn serve(method: &Method, inner: &str, orig: &str, query: Option<&str>) -> Out {
    if inner.ends_with("/package.json") {
        return Out::new(404, "text/plain; charset=utf-8", b"Not Found".to_vec());
    }
    if inner.ends_with("/swagger-ui-init.js") {
        return Out::new(
            200,
            "application/javascript; charset=utf-8",
            INIT_JS.to_vec(),
        );
    }
    if method == Method::GET || method == Method::HEAD {
        if inner == "/" && !orig.ends_with('/') {
            // serve-static directory redirect: /docs -> /docs/
            let mut loc = format!("{orig}/");
            if let Some(q) = query {
                loc.push('?');
                loc.push_str(q);
            }
            let body = format!(
                "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>Redirecting</title>\n</head>\n<body>\n<pre>Redirecting to {}</pre>\n</body>\n</html>\n",
                escape_html(&loc)
            );
            let mut out = Out::new(301, "text/html; charset=UTF-8", body.into_bytes());
            out.etag = false;
            if let Ok(v) = HeaderValue::from_str(&loc) {
                out.headers.insert(header::LOCATION, v);
            }
            out.headers.insert(
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static("default-src 'none'"),
            );
            return out;
        }
        let decoded = decode_uri_component(inner);
        if let Some((_, ct, bytes)) = ASSETS
            .iter()
            .find(|(p, _, _)| Some(*p) == decoded.as_deref())
        {
            let mut out = Out::new(200, ct, bytes.to_vec());
            out.headers.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=0"),
            );
            return out;
        }
    }
    Out::new(200, "text/html; charset=utf-8", INDEX_HTML.to_vec())
}
