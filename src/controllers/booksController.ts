import type { Request, Response } from 'express';
import { bibleStore } from '../services/bibleStore';
import { ok, notFound, badRequest } from '../utils/response';
import { displayName } from '../utils/bookNames';

export function listBooks(_req: Request, res: Response): void {
  ok(res, bibleStore.getBooks());
}

export function getBook(req: Request, res: Response): void {
  const meta = bibleStore.getBook(req.params.book);
  if (!meta) return notFound(res, `Book not found: ${req.params.book}`);
  ok(res, meta);
}

export function listChapters(req: Request, res: Response): void {
  const chapters = bibleStore.getChapters(req.params.book);
  if (!chapters) return notFound(res, `Book not found: ${req.params.book}`);
  ok(res, chapters);
}

export function getChapter(req: Request, res: Response): void {
  const chapter = parseInt(req.params.chapter, 10);
  if (isNaN(chapter)) return badRequest(res, 'Invalid chapter number');
  const verses = bibleStore.getChapterVerses(req.params.book, chapter);
  if (!verses) return notFound(res, `Chapter not found: ${req.params.book} ${chapter}`);
  ok(res, verses.map(v => ({ ...v, book: displayName(v.book) })));
}

export function getVerse(req: Request, res: Response): void {
  const chapter = parseInt(req.params.chapter, 10);
  const verseParam = req.params.verse;

  if (isNaN(chapter)) return badRequest(res, 'Invalid chapter number');

  // Check for verse range (e.g., "1-5")
  const rangeMatch = verseParam.match(/^(\d+)-(\d+)$/);
  if (rangeMatch) {
    const start = parseInt(rangeMatch[1], 10);
    const end = parseInt(rangeMatch[2], 10);
    if (start > end) return badRequest(res, 'Start verse must be less than or equal to end verse');
    const verses = bibleStore.getVerseRange(req.params.book, chapter, start, end);
    if (!verses) return notFound(res, `Verses not found: ${req.params.book} ${chapter}:${start}-${end}`);
    ok(res, verses.map(v => ({ ...v, book: displayName(v.book) })));
    return;
  }

  const verse = parseInt(verseParam, 10);
  if (isNaN(verse)) return badRequest(res, 'Invalid verse number');
  const result = bibleStore.getVerse(req.params.book, chapter, verse);
  if (!result) return notFound(res, `Verse not found: ${req.params.book} ${chapter}:${verse}`);
  ok(res, { ...result, book: displayName(result.book) });
}
