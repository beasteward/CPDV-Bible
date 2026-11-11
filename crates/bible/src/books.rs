//! Canonical book order and user-input book-name resolution.
//! Port of `src/utils/bookNames.ts` (same aliases, same normalization).

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Serialize;

use crate::js::is_js_space;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Testament {
    OT,
    NT,
}

impl Testament {
    pub fn as_str(self) -> &'static str {
        match self {
            Testament::OT => "OT",
            Testament::NT => "NT",
        }
    }
}

use Testament::{NT, OT};

/// Canonical order (Catholic canon, 46 OT + 27 NT). Keys are the JSON keys.
pub const BOOK_ORDER: [(&str, Testament); 73] = [
    ("Genesis", OT),
    ("Exodus", OT),
    ("Leviticus", OT),
    ("Numbers", OT),
    ("Deuteronomy", OT),
    ("Joshua", OT),
    ("Judges", OT),
    ("Ruth", OT),
    ("1-Samuel", OT),
    ("2-Samuel", OT),
    ("1-Kings", OT),
    ("2-Kings", OT),
    ("1-Chronicles", OT),
    ("2-Chronicles", OT),
    ("Ezra", OT),
    ("Nehemiah", OT),
    ("Tobit", OT),
    ("Judith", OT),
    ("Esther", OT),
    ("Job", OT),
    ("Psalms", OT),
    ("Proverbs", OT),
    ("Ecclesiastes", OT),
    ("Song2", OT),
    ("Wisdom", OT),
    ("Sirach", OT),
    ("Isaiah", OT),
    ("Jeremiah", OT),
    ("Lamentations", OT),
    ("Baruch", OT),
    ("Ezekiel", OT),
    ("Daniel", OT),
    ("Hosea", OT),
    ("Joel", OT),
    ("Amos", OT),
    ("Obadiah", OT),
    ("Jonah", OT),
    ("Micah", OT),
    ("Nahum", OT),
    ("Habakkuk", OT),
    ("Zephaniah", OT),
    ("Haggai", OT),
    ("Zechariah", OT),
    ("Malachi", OT),
    ("1-Maccabees", OT),
    ("2-Maccabees", OT),
    ("Matthew", NT),
    ("Mark", NT),
    ("Luke", NT),
    ("John", NT),
    ("Acts", NT),
    ("Romans", NT),
    ("1-Corinthians", NT),
    ("2-Corinthians", NT),
    ("Galatians", NT),
    ("Ephesians", NT),
    ("Philippians", NT),
    ("Colossians", NT),
    ("1-Thessalonians", NT),
    ("2-Thessalonians", NT),
    ("1-Timothy", NT),
    ("2-Timothy", NT),
    ("Titus", NT),
    ("Philemon", NT),
    ("Hebrews", NT),
    ("James", NT),
    ("1-Peter", NT),
    ("2-Peter", NT),
    ("1-John", NT),
    ("2-John", NT),
    ("3-John", NT),
    ("Jude", NT),
    ("Revelation", NT),
];

/// Extra aliases, in the same order as the Node source (later entries win).
pub const EXTRA_ALIASES: &[(&str, &str)] = &[
    ("gen", "Genesis"),
    ("ex", "Exodus"),
    ("lev", "Leviticus"),
    ("num", "Numbers"),
    ("deut", "Deuteronomy"),
    ("josh", "Joshua"),
    ("judg", "Judges"),
    ("sam", "1-Samuel"),
    ("1sam", "1-Samuel"),
    ("2sam", "2-Samuel"),
    ("1kgs", "1-Kings"),
    ("1kings", "1-Kings"),
    ("2kgs", "2-Kings"),
    ("2kings", "2-Kings"),
    ("1chr", "1-Chronicles"),
    ("1chron", "1-Chronicles"),
    ("1chronicles", "1-Chronicles"),
    ("2chr", "2-Chronicles"),
    ("2chron", "2-Chronicles"),
    ("2chronicles", "2-Chronicles"),
    ("neh", "Nehemiah"),
    ("tob", "Tobit"),
    ("jdt", "Judith"),
    ("est", "Esther"),
    ("ps", "Psalms"),
    ("psalm", "Psalms"),
    ("prov", "Proverbs"),
    ("eccl", "Ecclesiastes"),
    ("eccles", "Ecclesiastes"),
    ("qoheleth", "Ecclesiastes"),
    ("song", "Song2"),
    ("songofsolomon", "Song2"),
    ("songofsongs", "Song2"),
    ("canticle", "Song2"),
    ("canticles", "Song2"),
    ("canticleofcanticles", "Song2"),
    ("sos", "Song2"),
    ("wis", "Wisdom"),
    ("wisdomofsolomon", "Wisdom"),
    ("sir", "Sirach"),
    ("ecclesiasticus", "Sirach"),
    ("isa", "Isaiah"),
    ("jer", "Jeremiah"),
    ("lam", "Lamentations"),
    ("bar", "Baruch"),
    ("ezek", "Ezekiel"),
    ("eze", "Ezekiel"),
    ("dan", "Daniel"),
    ("hos", "Hosea"),
    ("ob", "Obadiah"),
    ("jon", "Jonah"),
    ("mic", "Micah"),
    ("nah", "Nahum"),
    ("hab", "Habakkuk"),
    ("zeph", "Zephaniah"),
    ("hag", "Haggai"),
    ("zech", "Zechariah"),
    ("mal", "Malachi"),
    ("1macc", "1-Maccabees"),
    ("1maccabees", "1-Maccabees"),
    ("2macc", "2-Maccabees"),
    ("2maccabees", "2-Maccabees"),
    ("matt", "Matthew"),
    ("mt", "Matthew"),
    ("mk", "Mark"),
    ("lk", "Luke"),
    ("jn", "John"),
    ("joh", "John"),
    ("rom", "Romans"),
    ("1cor", "1-Corinthians"),
    ("1corinthians", "1-Corinthians"),
    ("2cor", "2-Corinthians"),
    ("2corinthians", "2-Corinthians"),
    ("gal", "Galatians"),
    ("eph", "Ephesians"),
    ("phil", "Philippians"),
    ("php", "Philippians"),
    ("col", "Colossians"),
    ("1thess", "1-Thessalonians"),
    ("1thessalonians", "1-Thessalonians"),
    ("2thess", "2-Thessalonians"),
    ("2thessalonians", "2-Thessalonians"),
    ("1tim", "1-Timothy"),
    ("1timothy", "1-Timothy"),
    ("2tim", "2-Timothy"),
    ("2timothy", "2-Timothy"),
    ("tit", "Titus"),
    ("phlm", "Philemon"),
    ("phm", "Philemon"),
    ("heb", "Hebrews"),
    ("jas", "James"),
    ("1pet", "1-Peter"),
    ("1peter", "1-Peter"),
    ("2pet", "2-Peter"),
    ("2peter", "2-Peter"),
    ("1jn", "1-John"),
    ("1john", "1-John"),
    ("2jn", "2-John"),
    ("2john", "2-John"),
    ("3jn", "3-John"),
    ("3john", "3-John"),
    ("rev", "Revelation"),
    ("apocalypse", "Revelation"),
];

/// `input.toLowerCase().replace(/[\s-]/g, '')`
pub fn normalize(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .filter(|c| !is_js_space(*c) && *c != '-')
        .collect()
}

static ALIAS_MAP: LazyLock<HashMap<String, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    for (key, _) in BOOK_ORDER {
        m.insert(normalize(key), key);
        m.insert(normalize(&key.replace('-', "")), key);
        m.insert(normalize(&key.replace('-', " ")), key);
    }
    for (alias, key) in EXTRA_ALIASES {
        m.insert(normalize(alias), *key);
    }
    m
});

/// Resolve user input ("1 Sam", "jn", "Song of Songs") to the canonical key.
pub fn resolve_book_name(input: &str) -> Option<&'static str> {
    ALIAS_MAP.get(&normalize(input)).copied()
}

/// Display name: hyphens become spaces, `Song2` becomes "Song of Songs".
pub fn display_name(key: &str) -> String {
    if key == "Song2" {
        "Song of Songs".to_owned()
    } else {
        key.replace('-', " ")
    }
}

pub fn testament_of(key: &str) -> Option<Testament> {
    BOOK_ORDER.iter().find(|(k, _)| *k == key).map(|(_, t)| *t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_keys_and_aliases() {
        assert_eq!(resolve_book_name("Genesis"), Some("Genesis"));
        assert_eq!(resolve_book_name("GEN"), Some("Genesis"));
        assert_eq!(resolve_book_name("1 Samuel"), Some("1-Samuel"));
        assert_eq!(resolve_book_name("1samuel"), Some("1-Samuel"));
        assert_eq!(resolve_book_name("sam"), Some("1-Samuel"));
        assert_eq!(resolve_book_name("Song of Songs"), Some("Song2"));
        assert_eq!(resolve_book_name("song2"), Some("Song2"));
        assert_eq!(resolve_book_name("Ps"), Some("Psalms"));
        assert_eq!(resolve_book_name("psalm"), Some("Psalms"));
        assert_eq!(resolve_book_name("jn"), Some("John"));
        assert_eq!(resolve_book_name("1 jn"), Some("1-John"));
        assert_eq!(resolve_book_name("Gen\u{A0}esis"), Some("Genesis"));
        assert_eq!(resolve_book_name("1\u{212A}gs"), Some("1-Kings"));
        assert_eq!(resolve_book_name("Nope"), None);
        assert_eq!(resolve_book_name(""), None);
        assert_eq!(resolve_book_name("\u{200B}Genesis"), None); // ZWSP is not \s
    }

    #[test]
    fn display_names() {
        assert_eq!(display_name("Song2"), "Song of Songs");
        assert_eq!(display_name("1-Samuel"), "1 Samuel");
        assert_eq!(display_name("Genesis"), "Genesis");
    }

    #[test]
    fn every_key_resolves_to_itself() {
        for (key, _) in BOOK_ORDER {
            assert_eq!(resolve_book_name(key), Some(key));
            assert_eq!(resolve_book_name(&display_name(key)), Some(key), "{key}");
        }
    }
}
