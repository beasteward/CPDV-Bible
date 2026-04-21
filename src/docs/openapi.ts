import type { JsonObject } from 'swagger-ui-express';

const verseSchema = {
  type: 'object',
  properties: {
    book: { type: 'string', example: 'Genesis' },
    chapter: { type: 'integer', example: 1 },
    verse: { type: 'integer', example: 1 },
    text: { type: 'string', example: 'In the beginning, God created heaven and earth' },
  },
};

const bookMetaSchema = {
  type: 'object',
  properties: {
    name: { type: 'string', example: 'Genesis' },
    key: { type: 'string', example: 'Genesis' },
    testament: { type: 'string', enum: ['OT', 'NT'], example: 'OT' },
    chapterCount: { type: 'integer', example: 50 },
    verseCount: { type: 'integer', example: 1530 },
  },
};

const chapterMetaSchema = {
  type: 'object',
  properties: {
    chapter: { type: 'integer', example: 1 },
    verseCount: { type: 'integer', example: 31 },
  },
};

const searchResultSchema = {
  type: 'object',
  properties: {
    book: { type: 'string', example: 'Genesis' },
    chapter: { type: 'integer', example: 1 },
    verse: { type: 'integer', example: 1 },
    text: { type: 'string' },
    snippet: { type: 'string' },
  },
};

function envelope(dataSchema: object, meta?: object) {
  const props: Record<string, object> = {
    status: { type: 'string', example: 'ok' },
    data: dataSchema,
  };
  if (meta) props.meta = meta;
  return { type: 'object', properties: props };
}

function ok200(dataSchema: object, meta?: object) {
  return {
    200: {
      description: 'Successful response',
      content: { 'application/json': { schema: envelope(dataSchema, meta) } },
    },
  };
}

function notFound404(example: string) {
  return {
    404: {
      description: 'Not found',
      content: {
        'application/json': {
          schema: {
            type: 'object',
            properties: {
              status: { type: 'string', example: 'error' },
              error: { type: 'string', example: example },
            },
          },
        },
      },
    },
  };
}

const paginationMeta = {
  type: 'object',
  properties: {
    total: { type: 'integer', example: 42 },
    limit: { type: 'integer', example: 20 },
    offset: { type: 'integer', example: 0 },
  },
};

export const openApiSpec: JsonObject = {
  openapi: '3.0.3',
  info: {
    title: 'CPDV Bible API',
    description:
      'A production-ready REST API serving the Catholic Public Domain Version (CPDV) of the Bible. 73 books, 1,335 chapters, 35,837 verses. All data served from memory for sub-millisecond responses.',
    version: '1.0.0',
    license: { name: 'Public Domain' },
  },
  servers: [{ url: '/api/v1', description: 'API v1' }],
  tags: [
    { name: 'Books', description: 'Browse books, chapters, and verses' },
    { name: 'Search', description: 'Full-text search and discovery' },
    { name: 'Metadata', description: 'Bible statistics and groupings' },
    { name: 'Health', description: 'Service health' },
  ],
  paths: {
    '/books': {
      get: {
        tags: ['Books'],
        summary: 'List all books',
        description: 'Returns metadata for all 73 books in the CPDV Bible.',
        responses: ok200({ type: 'array', items: bookMetaSchema }),
      },
    },
    '/books/{book}': {
      get: {
        tags: ['Books'],
        summary: 'Get book metadata',
        description: 'Returns metadata for a single book. Accepts case-insensitive names and common aliases (e.g. "gen", "1samuel", "1-samuel", "sos", "rev").',
        parameters: [
          { name: 'book', in: 'path', required: true, schema: { type: 'string' }, example: 'Genesis' },
        ],
        responses: { ...ok200(bookMetaSchema), ...notFound404('Book not found: xyz') },
      },
    },
    '/books/{book}/chapters': {
      get: {
        tags: ['Books'],
        summary: 'List chapters in a book',
        description: 'Returns chapter numbers and verse counts for the specified book.',
        parameters: [
          { name: 'book', in: 'path', required: true, schema: { type: 'string' }, example: 'Genesis' },
        ],
        responses: { ...ok200({ type: 'array', items: chapterMetaSchema }), ...notFound404('Book not found: xyz') },
      },
    },
    '/books/{book}/{chapter}': {
      get: {
        tags: ['Books'],
        summary: 'Get all verses in a chapter',
        description: 'Returns all verses for the specified book and chapter.',
        parameters: [
          { name: 'book', in: 'path', required: true, schema: { type: 'string' }, example: 'Genesis' },
          { name: 'chapter', in: 'path', required: true, schema: { type: 'integer' }, example: 1 },
        ],
        responses: { ...ok200({ type: 'array', items: verseSchema }), ...notFound404('Chapter not found') },
      },
    },
    '/books/{book}/{chapter}/{verse}': {
      get: {
        tags: ['Books'],
        summary: 'Get a verse or verse range',
        description: 'Returns a single verse or a range (e.g. "1-5"). Use format `startVerse-endVerse` for ranges.',
        parameters: [
          { name: 'book', in: 'path', required: true, schema: { type: 'string' }, example: 'Genesis' },
          { name: 'chapter', in: 'path', required: true, schema: { type: 'integer' }, example: 1 },
          { name: 'verse', in: 'path', required: true, schema: { type: 'string' }, description: 'Single verse number or range (e.g. "1" or "1-5")', example: '1' },
        ],
        responses: { ...ok200(verseSchema), ...notFound404('Verse not found') },
      },
    },
    '/search': {
      get: {
        tags: ['Search'],
        summary: 'Full-text search',
        description: 'Search verse text with optional book and testament filters. Returns paginated results.',
        parameters: [
          { name: 'q', in: 'query', required: true, schema: { type: 'string' }, description: 'Search query', example: 'faith hope love' },
          { name: 'book', in: 'query', required: false, schema: { type: 'string' }, description: 'Filter by book name' },
          { name: 'testament', in: 'query', required: false, schema: { type: 'string', enum: ['OT', 'NT'] }, description: 'Filter by testament' },
          { name: 'limit', in: 'query', required: false, schema: { type: 'integer', default: 20, minimum: 1, maximum: 100 } },
          { name: 'offset', in: 'query', required: false, schema: { type: 'integer', default: 0, minimum: 0 } },
        ],
        responses: ok200({ type: 'array', items: searchResultSchema }, paginationMeta),
      },
    },
    '/random': {
      get: {
        tags: ['Search'],
        summary: 'Random verse',
        description: 'Returns a random verse from the entire Bible.',
        responses: ok200(verseSchema),
      },
    },
    '/verse-of-the-day': {
      get: {
        tags: ['Search'],
        summary: 'Verse of the day',
        description: 'Returns a deterministic verse based on the current date. Same verse for all requests on the same day.',
        responses: ok200(verseSchema),
      },
    },
    '/testaments': {
      get: {
        tags: ['Metadata'],
        summary: 'Testament groupings',
        description: 'Returns Old and New Testament book lists.',
        responses: ok200({
          type: 'object',
          properties: {
            OT: { type: 'array', items: { type: 'string' } },
            NT: { type: 'array', items: { type: 'string' } },
          },
        }),
      },
    },
    '/stats': {
      get: {
        tags: ['Metadata'],
        summary: 'Bible statistics',
        description: 'Returns total counts for books, chapters, and verses.',
        responses: ok200({
          type: 'object',
          properties: {
            books: { type: 'integer', example: 73 },
            chapters: { type: 'integer', example: 1335 },
            verses: { type: 'integer', example: 35837 },
          },
        }),
      },
    },
    '/health': {
      get: {
        tags: ['Health'],
        summary: 'Health check',
        description: 'Returns service health status and uptime.',
        responses: ok200({
          type: 'object',
          properties: {
            status: { type: 'string', example: 'healthy' },
            uptime: { type: 'number', example: 123.456 },
          },
        }),
      },
    },
  },
};
