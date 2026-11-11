//! Environment configuration. Same variables and defaults as `src/config.ts`.

use std::path::PathBuf;

use cpdv_bible::js;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub node_env: String,
    pub rate_limit_window_ms: u64,
    pub rate_limit_max: u64,
    pub log_level: String,
    /// Empty = allow all origins (`Access-Control-Allow-Origin: *`).
    pub cors_origins: Vec<String>,
    /// Empty = open access (no `X-API-Key` required).
    pub api_keys: Vec<String>,
    /// Rust-only: key the rate limiter by the first `X-Forwarded-For` address
    /// instead of the TCP peer. OFF by default = Node's behavior (Express had
    /// no `trust proxy`). See docs/CUTOVER.md before turning it on.
    pub trust_proxy: bool,
    /// Rust-only: path of the CPDV JSON file.
    pub data_path: PathBuf,
}

/// `parseList` from config.ts: empty or `*` = [], else split on commas.
pub fn parse_list(value: Option<&str>) -> Vec<String> {
    let Some(v) = value else { return Vec::new() };
    let t = js::trim(v);
    if t.is_empty() || t == "*" {
        return Vec::new();
    }
    v.split(',')
        .map(|s| js::trim(s).to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

fn int_env(name: &str, default: u64) -> u64 {
    let raw = std::env::var(name).ok().filter(|s| !s.is_empty());
    let n = js::parse_int(raw.as_deref().unwrap_or(""));
    if n.is_finite() && n >= 0.0 {
        n as u64
    } else {
        default
    }
}

impl Config {
    pub fn from_env() -> Self {
        let var = |k: &str| std::env::var(k).ok();
        Config {
            port: int_env("PORT", 3000).min(u16::MAX as u64) as u16,
            node_env: var("NODE_ENV")
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "development".into()),
            rate_limit_window_ms: int_env("RATE_LIMIT_WINDOW_MS", 900_000),
            rate_limit_max: int_env("RATE_LIMIT_MAX", 100),
            log_level: var("LOG_LEVEL")
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "info".into()),
            cors_origins: parse_list(var("CORS_ORIGINS").as_deref()),
            api_keys: parse_list(var("API_KEYS").as_deref()),
            trust_proxy: matches!(var("TRUST_PROXY").as_deref(), Some("1" | "true" | "yes")),
            data_path: var("DATA_PATH")
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("data/EntireBible-CPDV.json")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_parsing() {
        assert!(parse_list(None).is_empty());
        assert!(parse_list(Some("")).is_empty());
        assert!(parse_list(Some(" * ")).is_empty());
        assert_eq!(parse_list(Some("a, b,,c ")), vec!["a", "b", "c"]);
    }
}
