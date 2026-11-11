//! In-memory Bible store. Port of `src/services/bibleStore.ts`.
//!
//! Lookups that take a chapter/verse as `f64` receive the result of JS
//! `parseInt(...)` from the HTTP layer and look the data up exactly the way
//! Node did (`bookData[String(chapter)]`), so `"01"` finds chapter 1 and a
//! huge number simply finds nothing.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use serde::{Serialize, Serializer};
use serde_json::{Map, Value};

use crate::books::{display_name, resolve_book_name, Testament, BOOK_ORDER};
use crate::js;

/// A chapter key from the data file. Numeric keys serialize as JSON numbers,
/// non-numeric ones (the "P" prologues of Sirach and Lamentations) as strings.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ChapterKey {
    Num(u32),
    Name(String),
}

impl ChapterKey {
    /// Node used `isNaN(Number(key))`. The data only has all-digit keys and
    /// "P"; anything that is not all digits is treated as a name.
    pub fn from_raw(raw: &str) -> Self {
        if !raw.is_empty() && raw.bytes().all(|b| b.is_ascii_digit()) {
            if let Ok(n) = raw.parse::<u32>() {
                return ChapterKey::Num(n);
            }
        }
        ChapterKey::Name(raw.to_owned())
    }
}

impl fmt::Display for ChapterKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChapterKey::Num(n) => write!(f, "{n}"),
            ChapterKey::Name(s) => f.write_str(s),
        }
    }
}

impl Serialize for ChapterKey {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            ChapterKey::Num(n) => s.serialize_u32(*n),
            ChapterKey::Name(n) => s.serialize_str(n),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Verse {
    /// Canonical book key (e.g. "1-Samuel").
    pub book: &'static str,
    pub chapter: ChapterKey,
    pub verse: u32,
    pub text: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookMeta {
    pub name: String,
    pub key: &'static str,
    pub testament: Testament,
    pub chapter_count: usize,
    pub verse_count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterMeta {
    pub chapter: ChapterKey,
    pub verse_count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Testaments {
    pub old_testament: Vec<String>,
    pub new_testament: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct Stats {
    pub books: usize,
    pub chapters: usize,
    pub verses: usize,
}

#[derive(Debug)]
pub struct Chapter {
    pub key: ChapterKey,
    pub raw: String,
    /// Global verse indices, sorted by verse number.
    pub verses: Vec<u32>,
    by_num: HashMap<u32, u32>,
}

#[derive(Debug)]
pub struct Book {
    pub key: &'static str,
    pub testament: Testament,
    /// Sorted: non-numeric keys first, then numeric ascending.
    pub chapters: Vec<Chapter>,
    by_raw: HashMap<String, usize>,
}

impl Book {
    pub fn chapter_by_raw(&self, raw: &str) -> Option<&Chapter> {
        self.by_raw.get(raw).map(|&i| &self.chapters[i])
    }
}

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Shape(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "read error: {e}"),
            LoadError::Json(e) => write!(f, "json error: {e}"),
            LoadError::Shape(s) => write!(f, "unexpected data shape: {s}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// One search hit, shaped like the Node response (`book` is the display name).
#[derive(Clone, Debug, Serialize)]
pub struct SearchHit<'a> {
    pub book: String,
    pub chapter: &'a ChapterKey,
    pub verse: u32,
    pub text: &'a str,
    pub snippet: String,
}

pub struct BibleStore {
    books: Vec<Book>,
    by_key: HashMap<&'static str, usize>,
    /// Every verse in canonical order (Node's `allVerseKeys` order).
    verses: Vec<Verse>,
    index: HashMap<String, Vec<u32>>,
    meta: Vec<BookMeta>,
}

/// Node's chapter-key comparator.
fn chapter_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let na = matches!(ChapterKey::from_raw(a), ChapterKey::Num(_));
    let nb = matches!(ChapterKey::from_raw(b), ChapterKey::Num(_));
    match (na, nb) {
        (false, false) => a.cmp(b),
        (false, true) => Ordering::Less,
        (true, false) => Ordering::Greater,
        (true, true) => {
            let x: f64 = a.parse().unwrap_or(0.0);
            let y: f64 = b.parse().unwrap_or(0.0);
            x.partial_cmp(&y).unwrap_or(Ordering::Equal)
        }
    }
}

/// Exact lookup by `String(n)` for a JS number coming from `parseInt`.
fn num_key(n: f64) -> String {
    js::number_to_string(n)
}

fn as_u32(n: f64) -> Option<u32> {
    if n.is_finite() && n >= 0.0 && n.fract() == 0.0 && n <= u32::MAX as f64 {
        Some(n as u32)
    } else {
        None
    }
}

impl BibleStore {
    pub fn load_file(path: impl AsRef<Path>) -> Result<Self, LoadError> {
        let bytes = std::fs::read(path).map_err(LoadError::Io)?;
        Self::from_json_bytes(&bytes)
    }

    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, LoadError> {
        let raw: Map<String, Value> = serde_json::from_slice(bytes).map_err(LoadError::Json)?;
        let mut store = BibleStore {
            books: Vec::with_capacity(73),
            by_key: HashMap::new(),
            verses: Vec::with_capacity(36_000),
            index: HashMap::new(),
            meta: Vec::with_capacity(73),
        };

        for (key, testament) in BOOK_ORDER {
            // Non-book keys such as "charset" are never visited.
            let Some(book_val) = raw.get(key) else {
                continue;
            };
            let book_obj = book_val
                .as_object()
                .ok_or_else(|| LoadError::Shape(format!("{key} is not an object")))?;

            let mut raw_keys: Vec<&String> = book_obj.keys().collect();
            raw_keys.sort_by(|a, b| chapter_cmp(a, b));

            let mut chapters = Vec::with_capacity(raw_keys.len());
            let mut by_raw = HashMap::new();
            let mut book_verse_count = 0usize;

            for ch_raw in raw_keys {
                let ch_obj = book_obj[ch_raw]
                    .as_object()
                    .ok_or_else(|| LoadError::Shape(format!("{key} {ch_raw} is not an object")))?;
                let ch_key = ChapterKey::from_raw(ch_raw);

                let mut nums: Vec<(u32, &str)> = Vec::with_capacity(ch_obj.len());
                for (v_raw, text) in ch_obj {
                    let v: u32 = v_raw.parse().map_err(|_| {
                        LoadError::Shape(format!("verse key {key} {ch_raw}:{v_raw}"))
                    })?;
                    let text = text
                        .as_str()
                        .ok_or_else(|| LoadError::Shape(format!("text {key} {ch_raw}:{v_raw}")))?;
                    nums.push((v, text));
                }
                nums.sort_by_key(|(v, _)| *v);

                let mut idxs = Vec::with_capacity(nums.len());
                let mut by_num = HashMap::with_capacity(nums.len());
                for (v, text) in nums {
                    let idx = store.verses.len() as u32;
                    let mut words = js::tokenize(text);
                    words.sort_unstable();
                    words.dedup();
                    for w in words {
                        store.index.entry(w).or_default().push(idx);
                    }
                    store.verses.push(Verse {
                        book: key,
                        chapter: ch_key.clone(),
                        verse: v,
                        text: text.to_owned(),
                    });
                    idxs.push(idx);
                    by_num.insert(v, idx);
                    book_verse_count += 1;
                }

                by_raw.insert(ch_raw.clone(), chapters.len());
                chapters.push(Chapter {
                    key: ch_key,
                    raw: ch_raw.clone(),
                    verses: idxs,
                    by_num,
                });
            }

            store.meta.push(BookMeta {
                name: display_name(key),
                key,
                testament,
                chapter_count: chapters.len(),
                verse_count: book_verse_count,
            });
            store.by_key.insert(key, store.books.len());
            store.books.push(Book {
                key,
                testament,
                chapters,
                by_raw,
            });
        }
        Ok(store)
    }

    pub fn verse_at(&self, idx: u32) -> &Verse {
        &self.verses[idx as usize]
    }

    pub fn all_verses(&self) -> &[Verse] {
        &self.verses
    }

    pub fn book_by_key(&self, key: &str) -> Option<&Book> {
        self.by_key.get(key).map(|&i| &self.books[i])
    }

    fn resolve(&self, input: &str) -> Option<&Book> {
        self.book_by_key(resolve_book_name(input)?)
    }

    // ---- endpoints -------------------------------------------------------

    pub fn books(&self) -> &[BookMeta] {
        &self.meta
    }

    pub fn book(&self, input: &str) -> Option<&BookMeta> {
        let key = resolve_book_name(input)?;
        self.meta.iter().find(|m| m.key == key)
    }

    pub fn chapters(&self, input: &str) -> Option<Vec<ChapterMeta>> {
        let book = self.resolve(input)?;
        Some(
            book.chapters
                .iter()
                .map(|c| ChapterMeta {
                    chapter: c.key.clone(),
                    verse_count: c.verses.len(),
                })
                .collect(),
        )
    }

    fn chapter_by_num(&self, input: &str, chapter: f64) -> Option<&Chapter> {
        self.resolve(input)?.chapter_by_raw(&num_key(chapter))
    }

    pub fn chapter_verses(&self, input: &str, chapter: f64) -> Option<Vec<&Verse>> {
        let ch = self.chapter_by_num(input, chapter)?;
        Some(ch.verses.iter().map(|&i| self.verse_at(i)).collect())
    }

    pub fn verse(&self, input: &str, chapter: f64, verse: f64) -> Option<&Verse> {
        let ch = self.chapter_by_num(input, chapter)?;
        let v = as_u32(verse)?;
        ch.by_num.get(&v).map(|&i| self.verse_at(i))
    }

    /// Verses `start..=end` that exist in the chapter (Node looped over every
    /// integer in between; this gives the same result without the loop).
    pub fn verse_range(
        &self,
        input: &str,
        chapter: f64,
        start: f64,
        end: f64,
    ) -> Option<Vec<&Verse>> {
        let ch = self.chapter_by_num(input, chapter)?;
        let out: Vec<&Verse> = ch
            .verses
            .iter()
            .map(|&i| self.verse_at(i))
            .filter(|v| (v.verse as f64) >= start && (v.verse as f64) <= end)
            .collect();
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// First half of Node's `search`: tokenize and intersect posting lists.
    /// `None` means Node returned `{results: [], total: 0}` before reaching the
    /// book/testament filters (this matters for the error quirks upstream).
    pub fn search_candidates(&self, query: &str) -> Option<Vec<u32>> {
        let words = js::tokenize(query);
        if words.is_empty() {
            return None;
        }
        let mut cand: Option<Vec<u32>> = None;
        for w in &words {
            let list = self.index.get(w)?;
            cand = Some(match cand {
                None => list.clone(),
                Some(c) => intersect_sorted(&c, list),
            });
            if cand.as_ref().is_some_and(Vec::is_empty) {
                return None;
            }
        }
        cand
    }

    /// Book filter: unknown book names are ignored (Node behavior).
    pub fn filter_book(&self, cands: Vec<u32>, book: &str) -> Vec<u32> {
        match resolve_book_name(book) {
            Some(key) => cands
                .into_iter()
                .filter(|&i| self.verse_at(i).book == key)
                .collect(),
            None => cands,
        }
    }

    /// Testament filter: `testament.toUpperCase()` must be "OT" or "NT",
    /// anything else matches nothing.
    pub fn filter_testament(&self, cands: Vec<u32>, testament: &str) -> Vec<u32> {
        let t = testament.to_uppercase();
        cands
            .into_iter()
            .filter(|&i| {
                let v = self.verse_at(i);
                self.book_by_key(v.book)
                    .is_some_and(|b| b.testament.as_str() == t)
            })
            .collect()
    }

    /// `keys.slice(offset, offset + limit)` plus snippets.
    pub fn page<'a>(
        &'a self,
        keys: &[u32],
        query: &str,
        limit: f64,
        offset: f64,
    ) -> Vec<SearchHit<'a>> {
        let total = keys.len();
        // NaN-safe on purpose: JS `slice` treats a non-number offset as past the end.
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if !(offset < total as f64) {
            return Vec::new();
        }
        let start = offset as usize;
        let end = (start as f64 + limit).min(total as f64) as usize;
        keys[start..end]
            .iter()
            .map(|&i| {
                let v = self.verse_at(i);
                SearchHit {
                    book: display_name(v.book),
                    chapter: &v.chapter,
                    verse: v.verse,
                    text: &v.text,
                    snippet: snippet(&v.text, query),
                }
            })
            .collect()
    }

    pub fn random_verse(&self, r: f64) -> &Verse {
        let idx = (r * self.verses.len() as f64).floor() as usize;
        &self.verses[idx.min(self.verses.len() - 1)]
    }

    pub fn verse_of_the_day(&self, year: i32, month: u32, day: u32) -> &Verse {
        &self.verses[votd_index(year, month, day, self.verses.len())]
    }

    pub fn testaments(&self) -> Testaments {
        let names = |t: Testament| {
            self.meta
                .iter()
                .filter(|b| b.testament == t)
                .map(|b| b.name.clone())
                .collect()
        };
        Testaments {
            old_testament: names(Testament::OT),
            new_testament: names(Testament::NT),
        }
    }

    pub fn stats(&self) -> Stats {
        Stats {
            books: self.meta.len(),
            chapters: self.meta.iter().map(|b| b.chapter_count).sum(),
            verses: self.verses.len(),
        }
    }
}

fn intersect_sorted(a: &[u32], b: &[u32]) -> Vec<u32> {
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
        }
    }
    out
}

/// Node's snippet, computed on UTF-16 code units like JS strings.
pub fn snippet(text: &str, query: &str) -> String {
    let t16: Vec<u16> = text.encode_utf16().collect();
    let tl16: Vec<u16> = text.to_lowercase().encode_utf16().collect();
    let ql16: Vec<u16> = query.to_lowercase().encode_utf16().collect();
    let qlen = query.encode_utf16().count();
    let idx = find_u16(&tl16, &ql16);
    let len = t16.len();
    match idx {
        Some(idx) => {
            let start = idx.saturating_sub(40);
            let end = (idx + qlen + 40).min(len);
            let mut s = String::new();
            if start > 0 {
                s.push_str("...");
            }
            s.push_str(&String::from_utf16_lossy(&t16[start.min(end)..end]));
            if end < len {
                s.push_str("...");
            }
            s
        }
        None => {
            let end = 100.min(len);
            let mut s = String::from_utf16_lossy(&t16[..end]);
            if len > 100 {
                s.push_str("...");
            }
            s
        }
    }
}

fn find_u16(hay: &[u16], needle: &[u16]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Node: `${getFullYear()}-${getMonth()+1}-${getDate()}` hashed with
/// `hash = ((hash << 5) - hash + c) | 0`, index `Math.abs(hash) % n`.
pub fn votd_index(year: i32, month: u32, day: u32, n: usize) -> usize {
    let date = format!("{year}-{month}-{day}");
    let mut hash: i32 = 0;
    for c in date.encode_utf16() {
        hash = hash
            .wrapping_shl(5)
            .wrapping_sub(hash)
            .wrapping_add(c as i32);
    }
    ((hash as i64).abs() % n as i64) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_sort_puts_names_first() {
        let mut v = vec!["10", "2", "P", "1"];
        v.sort_by(|a, b| chapter_cmp(a, b));
        assert_eq!(v, vec!["P", "1", "2", "10"]);
    }

    #[test]
    fn votd_hash_matches_js() {
        // Values computed with the Node implementation.
        assert_eq!(
            votd_index(2026, 11, 11, 35_817),
            js_votd("2026-11-11", 35_817)
        );
        assert_eq!(votd_index(2026, 1, 1, 1000), js_votd("2026-1-1", 1000));
    }

    fn js_votd(s: &str, n: usize) -> usize {
        let mut h: i32 = 0;
        for c in s.chars() {
            h = h.wrapping_mul(31).wrapping_add(c as i32);
        }
        (h as i64).unsigned_abs() as usize % n
    }

    #[test]
    fn snippet_shapes() {
        let text = "In the beginning, God created heaven and earth.";
        assert_eq!(snippet(text, "God"), text);
        let long = format!("{}needle{}", "a".repeat(60), "b".repeat(60));
        let s = snippet(&long, "needle");
        assert!(s.starts_with("...") && s.ends_with("..."));
        assert_eq!(s.len(), 3 + 40 + 6 + 40 + 3);
        assert_eq!(
            snippet(&"x".repeat(120), "zzz"),
            format!("{}...", "x".repeat(100))
        );
    }
}
