//! JavaScript semantics the Node service depends on, reproduced in Rust so the
//! two implementations return byte-identical results.

/// JavaScript `\s` (WhiteSpace + LineTerminator, ES2015+). Differs from
/// `char::is_whitespace`: JS includes U+FEFF and excludes U+0085.
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// `String.prototype.trim`.
pub fn trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// `parseInt(s, 10)`: skips leading JS whitespace, optional sign, longest run
/// of ASCII digits. Returns NaN when there are no digits. Long digit strings
/// round to the nearest double (overflow gives Infinity), like V8.
pub fn parse_int(s: &str) -> f64 {
    let t = s.trim_start_matches(is_js_space);
    let (neg, rest) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let n = rest.bytes().take_while(u8::is_ascii_digit).count();
    if n == 0 {
        return f64::NAN;
    }
    let v: f64 = rest[..n].parse().unwrap_or(f64::INFINITY);
    if neg {
        -v
    } else {
        v
    }
}

/// `String(n)` / template-literal formatting of a JS number.
pub fn number_to_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 {
            "Infinity".into()
        } else {
            "-Infinity".into()
        };
    }
    if n == 0.0 {
        return "0".into(); // String(-0) === "0"
    }
    ryu_js::Buffer::new().format_finite(n).to_string()
}

/// JS truthiness of a number (`n || fallback`).
pub fn truthy(n: f64) -> bool {
    !(n.is_nan() || n == 0.0)
}

/// Search tokenizer shared by the index and queries:
/// `s.toLowerCase().replace(/[^\w\s]/g, '').split(/\s+/).filter(Boolean)`.
pub fn tokenize(s: &str) -> Vec<String> {
    let lower = s.to_lowercase();
    let kept: String = lower
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || is_js_space(*c))
        .collect();
    kept.split(is_js_space)
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// `decodeURIComponent`, returning `None` where JS would throw `URIError`.
pub fn decode_uri_component(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hi = b.get(i + 1).and_then(|c| (*c as char).to_digit(16))?;
            let lo = b.get(i + 2).and_then(|c| (*c as char).to_digit(16))?;
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_int_matches_js() {
        assert_eq!(parse_int("12"), 12.0);
        assert_eq!(parse_int("  7abc"), 7.0);
        assert_eq!(parse_int("+3"), 3.0);
        assert_eq!(parse_int("-4"), -4.0);
        assert_eq!(parse_int("1.9"), 1.0);
        assert_eq!(parse_int("1e3"), 1.0);
        assert_eq!(parse_int("0x10"), 0.0);
        assert!(parse_int("abc").is_nan());
        assert!(parse_int("").is_nan());
        assert!(parse_int("-").is_nan());
        assert!(parse_int("-0").is_sign_negative());
        assert_eq!(parse_int("\u{A0}5"), 5.0);
        assert_eq!(parse_int(&"9".repeat(400)), f64::INFINITY);
    }

    #[test]
    fn number_formatting_matches_js() {
        assert_eq!(number_to_string(1.0), "1");
        assert_eq!(number_to_string(-1.0), "-1");
        assert_eq!(number_to_string(-0.0), "0");
        assert_eq!(number_to_string(1e20), "100000000000000000000");
        assert_eq!(number_to_string(1e21), "1e+21");
        assert_eq!(number_to_string(f64::NAN), "NaN");
        assert_eq!(number_to_string(1.5), "1.5");
    }

    #[test]
    fn tokenizer_matches_js() {
        assert_eq!(
            tokenize("“Let there be light.”"),
            vec!["let", "there", "be", "light"]
        );
        assert_eq!(tokenize("the Lord’s day"), vec!["the", "lords", "day"]);
        assert_eq!(tokenize("a\u{A0}b"), vec!["a", "b"]);
        assert_eq!(tokenize("Señor"), vec!["seor"]);
        assert!(tokenize("!!! ").is_empty());
        // Kelvin sign lowercases to ASCII k before the \w filter.
        assert_eq!(tokenize("\u{212A}ing"), vec!["king"]);
    }

    #[test]
    fn decode_matches_js() {
        assert_eq!(
            decode_uri_component("1%20Samuel").as_deref(),
            Some("1 Samuel")
        );
        assert_eq!(decode_uri_component("a+b").as_deref(), Some("a+b"));
        assert_eq!(decode_uri_component("%E0"), None);
        assert_eq!(decode_uri_component("%ZZ"), None);
        assert_eq!(decode_uri_component("%"), None);
        assert_eq!(decode_uri_component("%ED%A0%80"), None); // lone surrogate
        assert_eq!(decode_uri_component("%E2%80%9C").as_deref(), Some("“"));
    }

    #[test]
    fn trim_uses_js_whitespace() {
        assert_eq!(trim("\u{FEFF} x \u{3000}"), "x");
        assert_eq!(trim("\u{85}x"), "\u{85}x");
    }
}
