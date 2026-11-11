//! CPDV Bible core: data model, loader, book-name resolution, search,
//! reference parsing and Vulgate <-> Hebrew psalm numbering.
//!
//! Everything that is part of the public `/api/v1` contract reproduces the
//! original Node/TypeScript service (`src/services/bibleStore.ts`,
//! `src/utils/bookNames.ts`) exactly, including its JavaScript quirks; the
//! helpers for those quirks live in [`js`].

pub mod books;
pub mod js;
pub mod psalms;
pub mod reference;
pub mod store;

pub use books::{display_name, resolve_book_name, Testament, BOOK_ORDER};
pub use reference::{
    parse_reference, parse_reference_hebrew_psalms, Reference, ReferenceError, VersePos,
};
pub use store::{BibleStore, Book, BookMeta, Chapter, ChapterKey, ChapterMeta, LoadError, Verse};
