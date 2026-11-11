//! The parts of `qs` (Express's "extended" query parser) that the search
//! endpoint can observe: whether a parameter is a string, an array (repeated
//! key or `k[]` / `k[n]`) or an object (`k[x]`), and its `String(...)` form.

use cpdv_bible::js::decode_uri_component;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QVal {
    Str(String),
    Arr(Vec<String>),
    Obj,
}

impl QVal {
    /// JavaScript `String(value)`.
    pub fn js_string(&self) -> String {
        match self {
            QVal::Str(s) => s.clone(),
            QVal::Arr(v) => v.join(","),
            QVal::Obj => "[object Object]".into(),
        }
    }
}

/// qs `decode`: '+' is a space; a malformed escape leaves the text as is.
fn decode(s: &str) -> String {
    let plus = s.replace('+', " ");
    decode_uri_component(&plus).unwrap_or(plus)
}

const ARRAY_LIMIT: u64 = 20;

/// Look up `name` in a raw query string.
pub fn get(query: Option<&str>, name: &str) -> Option<QVal> {
    let query = query?;
    let mut plain: Vec<String> = Vec::new();
    let mut indexed: Vec<(u64, String)> = Vec::new();
    let mut pushed: Vec<String> = Vec::new();
    let mut object = false;
    for part in query.split('&').take(1000) {
        if part.is_empty() {
            continue;
        }
        let pos = match part.find("]=") {
            Some(p) => Some(p + 1),
            None => part.find('='),
        };
        let (k, v) = match pos {
            Some(p) => (decode(&part[..p]), decode(&part[p + 1..])),
            None => (decode(part), String::new()),
        };
        if k == name {
            plain.push(v);
            continue;
        }
        let Some(rest) = k.strip_prefix(name) else {
            continue;
        };
        if !rest.starts_with('[') {
            continue;
        }
        let Some(close) = rest.find(']') else {
            continue;
        };
        let inner = &rest[1..close];
        if inner.contains('[') {
            continue;
        }
        if inner.is_empty() {
            pushed.push(v);
        } else if inner.bytes().all(|b| b.is_ascii_digit())
            && inner.parse::<u64>().is_ok_and(|n| n <= ARRAY_LIMIT)
        {
            indexed.push((inner.parse().unwrap_or(0), v));
        } else {
            object = true;
        }
    }
    let brackets = !indexed.is_empty() || !pushed.is_empty() || object;
    if plain.is_empty() && !brackets {
        return None;
    }
    if plain.len() == 1 && !brackets {
        return plain.pop().map(QVal::Str);
    }
    if object && plain.is_empty() {
        return Some(QVal::Obj);
    }
    indexed.sort_by_key(|(i, _)| *i);
    let mut all = plain;
    all.extend(indexed.into_iter().map(|(_, v)| v));
    all.extend(pushed);
    if object {
        all.push("[object Object]".into());
    }
    Some(QVal::Arr(all))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_and_decoding() {
        assert_eq!(
            get(Some("q=light&limit=5"), "q"),
            Some(QVal::Str("light".into()))
        );
        assert_eq!(
            get(Some("q=let+there%20be"), "q"),
            Some(QVal::Str("let there be".into()))
        );
        assert_eq!(get(Some("q=%E0x"), "q"), Some(QVal::Str("%E0x".into())));
        assert_eq!(get(Some("q"), "q"), Some(QVal::Str(String::new())));
        assert_eq!(get(Some("%71=x"), "q"), Some(QVal::Str("x".into())));
        assert_eq!(get(Some("x=1"), "q"), None);
        assert_eq!(get(None, "q"), None);
    }

    #[test]
    fn arrays_and_objects() {
        assert_eq!(
            get(Some("q=a&q=b"), "q"),
            Some(QVal::Arr(vec!["a".into(), "b".into()]))
        );
        assert_eq!(get(Some("q[]=a"), "q"), Some(QVal::Arr(vec!["a".into()])));
        assert_eq!(
            get(Some("limit[1]=5&limit[0]=7"), "limit")
                .unwrap()
                .js_string(),
            "7,5"
        );
        assert_eq!(get(Some("limit[a]=5"), "limit"), Some(QVal::Obj));
        assert_eq!(get(Some("limit[21]=5"), "limit"), Some(QVal::Obj));
        assert_eq!(
            get(Some("limit=5&limit=10"), "limit").unwrap().js_string(),
            "5,10"
        );
    }
}
