import fs from 'fs';
import path from 'path';
import { BOOK_ORDER, displayName, resolveBookName } from '../utils/bookNames';
import type { BibleJson, BookMeta, ChapterMeta, SearchResult, Verse } from '../types';

class BibleStore {
  private bookMap = new Map<string, Record<string, Record<string, string>>>();
  private verseMap = new Map<string, Verse>();
  private invertedIndex = new Map<string, string[]>(); // word → verse keys
  private allVerseKeys: string[] = [];
  private bookMetaCache: BookMeta[] = [];
  private loaded = false;

  load(filePath?: string): void {
    if (this.loaded) return;
    const p = filePath || path.join(__dirname, '../../data/EntireBible-CPDV.json');
    const raw: BibleJson = JSON.parse(fs.readFileSync(p, 'utf-8'));

    for (const { key, testament } of BOOK_ORDER) {
      const bookData = raw[key];
      if (!bookData) continue;
      this.bookMap.set(key, bookData);

      let bookVerseCount = 0;
      const chapterKeys = Object.keys(bookData);
      // Sort numerically where possible; non-numeric keys (e.g. "P" for Prologue) come first
      const chapters = chapterKeys.sort((a, b) => {
        const na = Number(a), nb = Number(b);
        if (isNaN(na) && isNaN(nb)) return a.localeCompare(b);
        if (isNaN(na)) return -1;
        if (isNaN(nb)) return 1;
        return na - nb;
      });

      for (const chKey of chapters) {
        const ch = isNaN(Number(chKey)) ? chKey : Number(chKey);
        const chData = bookData[chKey];
        const verses = Object.keys(chData).map(Number).sort((a, b) => a - b);
        for (const v of verses) {
          const text = chData[String(v)];
          const verseKey = `${key}:${ch}:${v}`;
          const verse: Verse = { book: key, chapter: ch, verse: v, text };
          this.verseMap.set(verseKey, verse);
          this.allVerseKeys.push(verseKey);

          // Build inverted index
          const words = text.toLowerCase().replace(/[^\w\s]/g, '').split(/\s+/).filter(Boolean);
          const unique = new Set(words);
          for (const word of unique) {
            let list = this.invertedIndex.get(word);
            if (!list) {
              list = [];
              this.invertedIndex.set(word, list);
            }
            list.push(verseKey);
          }
          bookVerseCount++;
        }
      }

      this.bookMetaCache.push({
        name: displayName(key),
        key,
        testament,
        chapterCount: chapters.length,
        verseCount: bookVerseCount,
      });
    }

    this.loaded = true;
  }

  getBooks(): BookMeta[] {
    return this.bookMetaCache;
  }

  getBook(input: string): BookMeta | undefined {
    const key = resolveBookName(input);
    if (!key) return undefined;
    return this.bookMetaCache.find(b => b.key === key);
  }

  getChapters(input: string): ChapterMeta[] | undefined {
    const key = resolveBookName(input);
    if (!key) return undefined;
    const bookData = this.bookMap.get(key);
    if (!bookData) return undefined;
    const chKeys = Object.keys(bookData);
    chKeys.sort((a, b) => {
      const na = Number(a), nb = Number(b);
      if (isNaN(na) && isNaN(nb)) return a.localeCompare(b);
      if (isNaN(na)) return -1;
      if (isNaN(nb)) return 1;
      return na - nb;
    });
    return chKeys.map(ch => ({
      chapter: isNaN(Number(ch)) ? ch : Number(ch),
      verseCount: Object.keys(bookData[ch]).length,
    }));
  }

  getChapterVerses(input: string, chapter: number | string): Verse[] | undefined {
    const key = resolveBookName(input);
    if (!key) return undefined;
    const bookData = this.bookMap.get(key);
    if (!bookData) return undefined;
    const chData = bookData[String(chapter)];
    if (!chData) return undefined;
    return Object.keys(chData)
      .map(Number)
      .sort((a, b) => a - b)
      .map(v => ({ book: key, chapter, verse: v, text: chData[String(v)] }));
  }

  getVerse(input: string, chapter: number | string, verse: number): Verse | undefined {
    const key = resolveBookName(input);
    if (!key) return undefined;
    return this.verseMap.get(`${key}:${chapter}:${verse}`);
  }

  getVerseRange(input: string, chapter: number | string, start: number, end: number): Verse[] | undefined {
    const key = resolveBookName(input);
    if (!key) return undefined;
    const bookData = this.bookMap.get(key);
    if (!bookData) return undefined;
    const chData = bookData[String(chapter)];
    if (!chData) return undefined;
    const verses: Verse[] = [];
    for (let v = start; v <= end; v++) {
      const text = chData[String(v)];
      if (text) verses.push({ book: key, chapter, verse: v, text });
    }
    return verses.length > 0 ? verses : undefined;
  }

  search(query: string, options: { book?: string; testament?: string; limit?: number; offset?: number }): { results: SearchResult[]; total: number } {
    const { book, testament, limit = 20, offset = 0 } = options;
    const words = query.toLowerCase().replace(/[^\w\s]/g, '').split(/\s+/).filter(Boolean);
    if (words.length === 0) return { results: [], total: 0 };

    // Intersect posting lists for all query words
    let candidateKeys: Set<string> | null = null;
    for (const word of words) {
      const list = this.invertedIndex.get(word);
      if (!list) return { results: [], total: 0 };
      const listSet = new Set(list);
      if (candidateKeys === null) {
        candidateKeys = listSet;
      } else {
        candidateKeys = new Set([...candidateKeys].filter((k: string) => listSet.has(k)));
      }
      if (candidateKeys.size === 0) return { results: [], total: 0 };
    }

    let keys = [...candidateKeys!];

    // Filter by book or testament
    const bookKey = book ? resolveBookName(book) : undefined;
    if (bookKey) {
      keys = keys.filter(k => k.startsWith(bookKey + ':'));
    }
    if (testament) {
      const t = testament.toUpperCase();
      const testamentBooks = new Set(
        this.bookMetaCache.filter(b => b.testament === t).map(b => b.key)
      );
      keys = keys.filter(k => testamentBooks.has(k.split(':')[0]));
    }

    const total = keys.length;
    const paged = keys.slice(offset, offset + limit);

    const queryLower = query.toLowerCase();
    const results: SearchResult[] = paged.map(k => {
      const verse = this.verseMap.get(k)!;
      // Create snippet with highlighted context
      const idx = verse.text.toLowerCase().indexOf(queryLower);
      let snippet: string;
      if (idx >= 0) {
        const start = Math.max(0, idx - 40);
        const end = Math.min(verse.text.length, idx + query.length + 40);
        snippet = (start > 0 ? '...' : '') + verse.text.slice(start, end) + (end < verse.text.length ? '...' : '');
      } else {
        snippet = verse.text.slice(0, 100) + (verse.text.length > 100 ? '...' : '');
      }
      return { ...verse, snippet };
    });

    return { results, total };
  }

  getRandomVerse(): Verse {
    const idx = Math.floor(Math.random() * this.allVerseKeys.length);
    return this.verseMap.get(this.allVerseKeys[idx])!;
  }

  getVerseOfTheDay(): Verse {
    const now = new Date();
    const dateStr = `${now.getFullYear()}-${now.getMonth() + 1}-${now.getDate()}`;
    // Simple deterministic hash
    let hash = 0;
    for (let i = 0; i < dateStr.length; i++) {
      hash = ((hash << 5) - hash + dateStr.charCodeAt(i)) | 0;
    }
    const idx = Math.abs(hash) % this.allVerseKeys.length;
    return this.verseMap.get(this.allVerseKeys[idx])!;
  }

  getTestaments(): { oldTestament: string[]; newTestament: string[] } {
    return {
      oldTestament: this.bookMetaCache.filter(b => b.testament === 'OT').map(b => b.name),
      newTestament: this.bookMetaCache.filter(b => b.testament === 'NT').map(b => b.name),
    };
  }

  getStats(): { books: number; chapters: number; verses: number } {
    return {
      books: this.bookMetaCache.length,
      chapters: this.bookMetaCache.reduce((sum, b) => sum + b.chapterCount, 0),
      verses: this.allVerseKeys.length,
    };
  }
}

export const bibleStore = new BibleStore();
