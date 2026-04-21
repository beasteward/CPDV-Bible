export interface Verse {
  book: string;
  chapter: number | string;
  verse: number;
  text: string;
}

export interface BookMeta {
  name: string;
  key: string;
  testament: 'OT' | 'NT';
  chapterCount: number;
  verseCount: number;
}

export interface ChapterMeta {
  chapter: number | string;
  verseCount: number;
}

export interface ApiResponse<T = unknown> {
  status: 'ok' | 'error';
  data?: T;
  error?: string;
  meta?: Record<string, unknown>;
}

export interface SearchResult {
  book: string;
  chapter: number | string;
  verse: number;
  text: string;
  snippet: string;
}

export interface PaginationMeta {
  total: number;
  limit: number;
  offset: number;
}

export type BibleJson = Record<string, Record<string, Record<string, string>>>;
