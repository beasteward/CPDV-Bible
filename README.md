# CPDV Bible API

REST API for the Catholic Public Domain Version (CPDV) Bible — 73 books, ~31,000 verses, full-text search.

## Quick Start

```bash
npm install
npm run dev        # development with hot reload
npm run build      # compile TypeScript
npm start          # production
```

## Docker

```bash
docker compose up --build
```

## API Endpoints

Base: `/api/v1`

### Books & Verses
| Method | Path | Description |
|--------|------|-------------|
| GET | `/books` | List all books with metadata |
| GET | `/books/:book` | Single book metadata |
| GET | `/books/:book/chapters` | List chapters with verse counts |
| GET | `/books/:book/:chapter` | All verses in a chapter |
| GET | `/books/:book/:chapter/:verse` | Single verse |
| GET | `/books/:book/:chapter/:start-:end` | Verse range |

### Search & Discovery
| Method | Path | Description |
|--------|------|-------------|
| GET | `/search?q=term` | Full-text search (params: `book`, `testament`, `limit`, `offset`) |
| GET | `/random` | Random verse |
| GET | `/verse-of-the-day` | Deterministic daily verse |

### Metadata & Health
| Method | Path | Description |
|--------|------|-------------|
| GET | `/testaments` | OT/NT book groupings |
| GET | `/stats` | Total books, chapters, verses |
| GET | `/health` | Health check |

## Book Name Resolution

Book names are case-insensitive and support common aliases:
- `genesis`, `Genesis`, `gen`, `Gen`
- `1samuel`, `1-samuel`, `1 Samuel`, `1sam`
- `song`, `Song of Solomon`, `Song of Songs`, `sos`
- `rev`, `Revelation`, `apocalypse`

## Response Format

```json
{
  "status": "ok",
  "data": { ... },
  "meta": { "total": 42, "limit": 20, "offset": 0 }
}
```

## Environment Variables

See `.env.example` for all options.

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | 3000 | Server port |
| `NODE_ENV` | development | Environment |
| `RATE_LIMIT_WINDOW_MS` | 900000 | Rate limit window (15 min) |
| `RATE_LIMIT_MAX` | 100 | Max requests per window |
| `LOG_LEVEL` | info | Pino log level |
| `CORS_ORIGIN` | * | CORS allowed origin |

## License

MIT
