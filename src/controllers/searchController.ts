import type { Request, Response } from 'express';
import { bibleStore } from '../services/bibleStore';
import { ok, badRequest } from '../utils/response';
import { displayName } from '../utils/bookNames';

export function search(req: Request, res: Response): void {
  const q = req.query.q as string | undefined;
  if (!q || q.trim().length === 0) return badRequest(res, 'Query parameter "q" is required');

  const book = req.query.book as string | undefined;
  const testament = req.query.testament as string | undefined;
  const limit = Math.min(Math.max(parseInt(req.query.limit as string, 10) || 20, 1), 100);
  const offset = Math.max(parseInt(req.query.offset as string, 10) || 0, 0);

  const { results, total } = bibleStore.search(q.trim(), { book, testament, limit, offset });

  ok(
    res,
    results.map(r => ({ ...r, book: displayName(r.book) })),
    { total, limit, offset }
  );
}

export function randomVerse(_req: Request, res: Response): void {
  const verse = bibleStore.getRandomVerse();
  ok(res, { ...verse, book: displayName(verse.book) });
}

export function verseOfTheDay(_req: Request, res: Response): void {
  const verse = bibleStore.getVerseOfTheDay();
  ok(res, { ...verse, book: displayName(verse.book) });
}
