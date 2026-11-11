//! Scripture reference parser: "John 3:16", "1 Sam 3:1-10", "Gen 1:1-2:3",
//! "Ps 22", "Sirach P:1", "Jude 5" (single-chapter books take a verse).
//!
//! Not part of the Node `/api/v1` contract (Node had no reference parser); it
//! is library code for callers such as the Emmaus Road reader panel.

use std::fmt;

use crate::books::resolve_book_name;
use crate::psalms::{hebrew_chapter_span, hebrew_to_vulgate, PsalmRef};
use crate::store::ChapterKey;

/// Books with a single chapter: a bare number after them is a verse.
pub const SINGLE_CHAPTER_BOOKS: [&str; 5] = ["Obadiah", "Philemon", "2-John", "3-John", "Jude"];

/// A chapter, optionally narrowed to one verse. In a range end, `verse: None`
/// means "to the end of the chapter"; in a start, "from the start".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersePos {
    pub chapter: ChapterKey,
    pub verse: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    /// Canonical book key ("1-Samuel", "Song2", ...).
    pub book: &'static str,
    /// `None` = the whole book.
    pub start: Option<VersePos>,
    /// `None` = a single chapter or verse (just `start`).
    pub end: Option<VersePos>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceError {
    Empty,
    UnknownBook(String),
    BadNumber(String),
    BackwardsRange,
}

impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceError::Empty => f.write_str("empty reference"),
            ReferenceError::UnknownBook(b) => write!(f, "unknown book: {b}"),
            ReferenceError::BadNumber(s) => write!(f, "bad chapter/verse: {s}"),
            ReferenceError::BackwardsRange => f.write_str("range ends before it starts"),
        }
    }
}

impl std::error::Error for ReferenceError {}

fn is_tail_char(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, ':' | '.' | '-' | '\u{2013}' | '\u{2014}')
}

fn num(s: &str) -> Result<u32, ReferenceError> {
    match s.parse::<u32>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(ReferenceError::BadNumber(s.to_owned())),
    }
}

fn chapter(s: &str) -> Result<ChapterKey, ReferenceError> {
    if s.eq_ignore_ascii_case("p") {
        Ok(ChapterKey::Name("P".into()))
    } else {
        Ok(ChapterKey::Num(num(s)?))
    }
}

/// "C", "C:V" or "C.V".
fn pos(s: &str) -> Result<VersePos, ReferenceError> {
    match s.split_once([':', '.']) {
        Some((c, v)) => Ok(VersePos {
            chapter: chapter(c)?,
            verse: Some(num(v)?),
        }),
        None => Ok(VersePos {
            chapter: chapter(s)?,
            verse: None,
        }),
    }
}

/// Split "1 Sam 3:4" into ("1 Sam", "3:4"). The tail is the longest suffix of
/// digits and separators that follows a letter or whitespace; a lone "P" token
/// before it is a prologue chapter.
fn split_book(input: &str) -> (&str, &str) {
    let mut cut = input.len();
    for (i, c) in input.char_indices().rev() {
        if is_tail_char(c) {
            cut = i;
        } else {
            break;
        }
    }
    let (mut book, mut tail) = input.split_at(cut);
    // A tail that is all digits glued to a book like "1John" is not a tail
    // unless something separates it from the name.
    if book.is_empty() {
        return (input, "");
    }
    // Prologue: "Sirach P:1" / "Sirach P".
    let trimmed = book.trim_end();
    if (tail.starts_with(':') || tail.starts_with('.') || tail.is_empty())
        && (trimmed.ends_with(" P") || trimmed.ends_with(" p"))
    {
        let p_at = trimmed.len() - 1;
        tail = &input[p_at..];
        book = &input[..p_at];
    }
    (
        book.trim().trim_end_matches('.'),
        tail.trim_start_matches('.'),
    )
}

/// Parse a reference using the CPDV (Vulgate) psalm numbering.
pub fn parse_reference(input: &str) -> Result<Reference, ReferenceError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ReferenceError::Empty);
    }
    let (book_part, tail) = split_book(input);
    let book = resolve_book_name(book_part)
        .ok_or_else(|| ReferenceError::UnknownBook(book_part.to_owned()))?;
    if tail.is_empty() {
        return Ok(Reference {
            book,
            start: None,
            end: None,
        });
    }
    let (left, right) = match tail.split_once(['-', '\u{2013}', '\u{2014}']) {
        Some((l, r)) => (l, Some(r)),
        None => (tail, None),
    };
    let mut start = pos(left)?;
    let single = SINGLE_CHAPTER_BOOKS.contains(&book);
    if single && start.verse.is_none() {
        if let ChapterKey::Num(n) = start.chapter {
            start = VersePos {
                chapter: ChapterKey::Num(1),
                verse: Some(n),
            };
        }
    }
    let end = match right {
        None => None,
        Some(r) if r.contains([':', '.']) => Some(pos(r)?),
        Some(r) => Some(match start.verse {
            // "3:1-10": verse in the same chapter; "3-5": chapters.
            Some(_) => VersePos {
                chapter: start.chapter.clone(),
                verse: Some(num(r)?),
            },
            None => VersePos {
                chapter: chapter(r)?,
                verse: None,
            },
        }),
    };
    if let Some(e) = &end {
        if order_key(e) < order_key(&start) {
            return Err(ReferenceError::BackwardsRange);
        }
    }
    Ok(Reference {
        book,
        start: Some(start),
        end,
    })
}

fn order_key(p: &VersePos) -> (i64, u32) {
    let c = match p.chapter {
        ChapterKey::Num(n) => n as i64,
        ChapterKey::Name(_) => -1, // prologues come first, like the API
    };
    (c, p.verse.unwrap_or(0))
}

/// Parse a reference whose psalm numbers are Hebrew (most modern English
/// Bibles) and convert it to CPDV (Vulgate) numbering. Non-psalm references
/// are returned unchanged.
pub fn parse_reference_hebrew_psalms(input: &str) -> Result<Reference, ReferenceError> {
    let r = parse_reference(input)?;
    if r.book != "Psalms" {
        return Ok(r);
    }
    let Some(start) = &r.start else { return Ok(r) };
    let conv = |p: &VersePos| -> Result<PsalmRef, ReferenceError> {
        match p.chapter {
            ChapterKey::Num(c) => hebrew_to_vulgate(PsalmRef::new(c, p.verse))
                .ok_or_else(|| ReferenceError::BadNumber(c.to_string())),
            ChapterKey::Name(ref s) => Err(ReferenceError::BadNumber(s.clone())),
        }
    };
    let to_pos = |p: PsalmRef| VersePos {
        chapter: ChapterKey::Num(p.chapter),
        verse: p.verse,
    };
    let ChapterKey::Num(sc) = start.chapter else {
        unreachable!()
    };
    // Whole Hebrew chapter(s): expand to the exact Vulgate span.
    if start.verse.is_none() {
        let (s, _) =
            hebrew_chapter_span(sc).ok_or_else(|| ReferenceError::BadNumber(sc.to_string()))?;
        let last = match &r.end {
            Some(VersePos {
                chapter: ChapterKey::Num(ec),
                verse: None,
            }) => *ec,
            Some(e) => {
                return Ok(Reference {
                    book: r.book,
                    start: Some(to_pos(s)),
                    end: Some(to_pos(conv(e)?)),
                })
            }
            None => sc,
        };
        let (_, e) =
            hebrew_chapter_span(last).ok_or_else(|| ReferenceError::BadNumber(last.to_string()))?;
        let end = if s == e { None } else { Some(to_pos(e)) };
        return Ok(Reference {
            book: r.book,
            start: Some(to_pos(s)),
            end,
        });
    }
    let s = conv(start)?;
    let end = match &r.end {
        Some(e) => Some(to_pos(conv(e)?)),
        None => None,
    };
    Ok(Reference {
        book: r.book,
        start: Some(to_pos(s)),
        end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(c: u32, v: Option<u32>) -> VersePos {
        VersePos {
            chapter: ChapterKey::Num(c),
            verse: v,
        }
    }

    #[test]
    fn simple_forms() {
        let r = parse_reference("John 3:16").unwrap();
        assert_eq!(
            (r.book, r.start, r.end),
            ("John", Some(p(3, Some(16))), None)
        );
        let r = parse_reference("1 Sam 3:1-10").unwrap();
        assert_eq!(
            (r.book, r.start, r.end),
            ("1-Samuel", Some(p(3, Some(1))), Some(p(3, Some(10))))
        );
        let r = parse_reference("Gen 1:1-2:3").unwrap();
        assert_eq!((r.start, r.end), (Some(p(1, Some(1))), Some(p(2, Some(3)))));
        let r = parse_reference("Genesis").unwrap();
        assert_eq!((r.book, r.start), ("Genesis", None));
        let r = parse_reference("Isaiah 52-53").unwrap();
        assert_eq!((r.start, r.end), (Some(p(52, None)), Some(p(53, None))));
        let r = parse_reference("1John 4.8").unwrap();
        assert_eq!((r.book, r.start), ("1-John", Some(p(4, Some(8)))));
        let r = parse_reference("Gen. 1:1").unwrap();
        assert_eq!(r.book, "Genesis");
        let r = parse_reference("Song of Songs 2:1").unwrap();
        assert_eq!(r.book, "Song2");
        let r = parse_reference("Rom 5:1\u{2013}5").unwrap();
        assert_eq!(r.end, Some(p(5, Some(5))));
    }

    #[test]
    fn single_chapter_books() {
        let r = parse_reference("Jude 5").unwrap();
        assert_eq!(r.start, Some(p(1, Some(5))));
        let r = parse_reference("Philemon 1:4").unwrap();
        assert_eq!(r.start, Some(p(1, Some(4))));
    }

    #[test]
    fn prologue_chapters() {
        let r = parse_reference("Sirach P:2").unwrap();
        assert_eq!(r.book, "Sirach");
        assert_eq!(
            r.start,
            Some(VersePos {
                chapter: ChapterKey::Name("P".into()),
                verse: Some(2)
            })
        );
        let r = parse_reference("Lamentations P").unwrap();
        assert_eq!(
            r.start,
            Some(VersePos {
                chapter: ChapterKey::Name("P".into()),
                verse: None
            })
        );
    }

    #[test]
    fn errors() {
        assert_eq!(parse_reference("  "), Err(ReferenceError::Empty));
        assert!(matches!(
            parse_reference("Hezekiah 1:1"),
            Err(ReferenceError::UnknownBook(_))
        ));
        assert!(matches!(
            parse_reference("John 0:1"),
            Err(ReferenceError::BadNumber(_))
        ));
        assert!(matches!(
            parse_reference("John 3:"),
            Err(ReferenceError::BadNumber(_))
        ));
        assert_eq!(
            parse_reference("John 3:16-3:1"),
            Err(ReferenceError::BackwardsRange)
        );
        assert_eq!(
            parse_reference("John 4-3"),
            Err(ReferenceError::BackwardsRange)
        );
    }

    #[test]
    fn hebrew_psalm_numbering() {
        let r = parse_reference_hebrew_psalms("Psalm 23").unwrap();
        assert_eq!((r.start, r.end), (Some(p(22, None)), None));
        let r = parse_reference_hebrew_psalms("Ps 23:1").unwrap();
        assert_eq!(r.start, Some(p(22, Some(1))));
        let r = parse_reference_hebrew_psalms("Ps 10").unwrap();
        assert_eq!(
            (r.start, r.end),
            (Some(p(9, Some(22))), Some(p(9, Some(39))))
        );
        let r = parse_reference_hebrew_psalms("Ps 116").unwrap();
        assert_eq!((r.start, r.end), (Some(p(114, None)), Some(p(115, None))));
        let r = parse_reference_hebrew_psalms("Ps 116:10-12").unwrap();
        assert_eq!(
            (r.start, r.end),
            (Some(p(115, Some(1))), Some(p(115, Some(3))))
        );
        let r = parse_reference_hebrew_psalms("Ps 114-115").unwrap();
        assert_eq!(
            (r.start, r.end),
            (Some(p(113, Some(1))), Some(p(113, Some(26))))
        );
        // Non-psalms are untouched.
        let r = parse_reference_hebrew_psalms("John 3:16").unwrap();
        assert_eq!(r.start, Some(p(3, Some(16))));
    }
}
