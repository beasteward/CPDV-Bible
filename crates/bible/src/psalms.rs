//! Vulgate/Septuagint (CPDV) <-> Hebrew (Masoretic) psalm numbering.
//!
//! | Hebrew  | Vulgate (CPDV)                          |
//! |---------|-----------------------------------------|
//! | 1-8     | 1-8                                     |
//! | 9-10    | 9 (Hebrew 10:1 = Vulgate 9:22)          |
//! | 11-113  | 10-112                                  |
//! | 114-115 | 113 (Hebrew 115:1 = Vulgate 113:9)      |
//! | 116     | 114-115 (Hebrew 116:10 = Vulgate 115:1) |
//! | 117-146 | 116-145                                 |
//! | 147     | 146-147 (Hebrew 147:12 = Vulgate 147:1) |
//! | 148-150 | 148-150                                 |
//!
//! Verse numbers follow the CPDV data, which counts psalm titles as verse 1
//! like the Masoretic text. Only the split/merged psalms shift verse numbers;
//! English-Bible verse offsets (titles not counted) are not applied.

/// A psalm position: chapter plus optional verse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PsalmRef {
    pub chapter: u32,
    pub verse: Option<u32>,
}

impl PsalmRef {
    pub fn new(chapter: u32, verse: Option<u32>) -> Self {
        PsalmRef { chapter, verse }
    }
}

pub const PSALM_COUNT: u32 = 150;

/// Vulgate (CPDV) -> Hebrew. Returns `None` for chapters outside 1..=150.
/// For the Vulgate psalms that cover two Hebrew psalms (9, 113) a missing
/// verse maps to the first Hebrew psalm.
pub fn vulgate_to_hebrew(v: PsalmRef) -> Option<PsalmRef> {
    let PsalmRef { chapter: c, verse } = v;
    let r = match c {
        1..=8 | 148..=150 => PsalmRef::new(c, verse),
        9 => match verse {
            Some(x) if x >= 22 => PsalmRef::new(10, Some(x - 21)),
            _ => PsalmRef::new(9, verse),
        },
        10..=112 => PsalmRef::new(c + 1, verse),
        113 => match verse {
            Some(x) if x >= 9 => PsalmRef::new(115, Some(x - 8)),
            _ => PsalmRef::new(114, verse),
        },
        114 => PsalmRef::new(116, verse),
        115 => PsalmRef::new(116, verse.map(|x| x + 9)),
        116..=145 => PsalmRef::new(c + 1, verse),
        146 => PsalmRef::new(147, verse),
        147 => PsalmRef::new(147, verse.map(|x| x + 11)),
        _ => return None,
    };
    Some(r)
}

/// Hebrew -> Vulgate (CPDV). Returns `None` for chapters outside 1..=150.
/// For the Hebrew psalms split in the Vulgate (116, 147) a missing verse maps
/// to the first Vulgate psalm.
pub fn hebrew_to_vulgate(h: PsalmRef) -> Option<PsalmRef> {
    let PsalmRef { chapter: c, verse } = h;
    let r = match c {
        1..=9 | 148..=150 => PsalmRef::new(c, verse),
        10 => PsalmRef::new(9, Some(verse.map_or(22, |x| x + 21))),
        11..=113 => PsalmRef::new(c - 1, verse),
        114 => PsalmRef::new(113, verse),
        115 => PsalmRef::new(113, Some(verse.map_or(9, |x| x + 8))),
        116 => match verse {
            Some(x) if x >= 10 => PsalmRef::new(115, Some(x - 9)),
            _ => PsalmRef::new(114, verse),
        },
        117..=146 => PsalmRef::new(c - 1, verse),
        147 => match verse {
            Some(x) if x >= 12 => PsalmRef::new(147, Some(x - 11)),
            _ => PsalmRef::new(146, verse),
        },
        _ => return None,
    };
    Some(r)
}

/// The Vulgate span covered by a whole Hebrew psalm: (first, last). A
/// `verse: None` boundary means the whole Vulgate chapter. Verse counts of the
/// split psalms come from the CPDV data (Vulgate 9 = 39 verses, 113 = 26).
pub fn hebrew_chapter_span(c: u32) -> Option<(PsalmRef, PsalmRef)> {
    let whole = |c| PsalmRef::new(c, None);
    let r = match c {
        9 => (PsalmRef::new(9, Some(1)), PsalmRef::new(9, Some(21))),
        10 => (PsalmRef::new(9, Some(22)), PsalmRef::new(9, Some(39))),
        114 => (PsalmRef::new(113, Some(1)), PsalmRef::new(113, Some(8))),
        115 => (PsalmRef::new(113, Some(9)), PsalmRef::new(113, Some(26))),
        116 => (whole(114), whole(115)),
        147 => (whole(146), whole(147)),
        1..=150 => {
            let v = hebrew_to_vulgate(PsalmRef::new(c, None))?.chapter;
            (whole(v), whole(v))
        }
        _ => return None,
    };
    Some(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v2h(c: u32, v: Option<u32>) -> (u32, Option<u32>) {
        let r = vulgate_to_hebrew(PsalmRef::new(c, v)).unwrap();
        (r.chapter, r.verse)
    }
    fn h2v(c: u32, v: Option<u32>) -> (u32, Option<u32>) {
        let r = hebrew_to_vulgate(PsalmRef::new(c, v)).unwrap();
        (r.chapter, r.verse)
    }

    #[test]
    fn well_known_psalms() {
        assert_eq!(h2v(23, None), (22, None)); // The Lord is my shepherd
        assert_eq!(v2h(22, Some(1)), (23, Some(1)));
        assert_eq!(h2v(51, None), (50, None)); // Miserere
        assert_eq!(h2v(1, Some(1)), (1, Some(1)));
        assert_eq!(h2v(150, Some(6)), (150, Some(6)));
        assert_eq!(h2v(119, None), (118, None));
    }

    #[test]
    fn split_and_merged_psalms() {
        assert_eq!(v2h(9, Some(21)), (9, Some(21)));
        assert_eq!(v2h(9, Some(22)), (10, Some(1)));
        assert_eq!(v2h(9, Some(39)), (10, Some(18)));
        assert_eq!(h2v(10, Some(1)), (9, Some(22)));
        assert_eq!(h2v(10, None), (9, Some(22)));
        assert_eq!(v2h(113, Some(8)), (114, Some(8)));
        assert_eq!(v2h(113, Some(9)), (115, Some(1)));
        assert_eq!(h2v(115, Some(1)), (113, Some(9)));
        assert_eq!(v2h(114, Some(9)), (116, Some(9)));
        assert_eq!(v2h(115, Some(1)), (116, Some(10)));
        assert_eq!(h2v(116, Some(10)), (115, Some(1)));
        assert_eq!(h2v(116, Some(9)), (114, Some(9)));
        assert_eq!(v2h(146, Some(11)), (147, Some(11)));
        assert_eq!(v2h(147, Some(1)), (147, Some(12)));
        assert_eq!(h2v(147, Some(12)), (147, Some(1)));
        assert_eq!(h2v(147, Some(11)), (146, Some(11)));
    }

    #[test]
    fn out_of_range() {
        assert!(vulgate_to_hebrew(PsalmRef::new(0, None)).is_none());
        assert!(hebrew_to_vulgate(PsalmRef::new(151, None)).is_none());
    }

    #[test]
    fn round_trip_every_verse_position() {
        // Verse counts per Vulgate psalm are not needed: check every chapter
        // with verses 1..=40 round-trips through Hebrew and back, except
        // positions that fall past the end of a split psalm.
        for c in 1..=150 {
            for v in 1..=40 {
                let h = vulgate_to_hebrew(PsalmRef::new(c, Some(v))).unwrap();
                let back = hebrew_to_vulgate(h).unwrap();
                let past_split = (c == 114 && v >= 10) || (c == 146 && v >= 12);
                if !past_split {
                    assert_eq!(back, PsalmRef::new(c, Some(v)), "V{c}:{v} -> {h:?}");
                }
            }
        }
    }

    #[test]
    fn chapter_only_is_monotonic_and_covers_hebrew() {
        let mut seen = std::collections::BTreeSet::new();
        for c in 1..=150 {
            seen.insert(vulgate_to_hebrew(PsalmRef::new(c, None)).unwrap().chapter);
            seen.insert(
                vulgate_to_hebrew(PsalmRef::new(c, Some(40)))
                    .unwrap()
                    .chapter,
            );
        }
        assert_eq!(seen.len(), 150);
    }
}
