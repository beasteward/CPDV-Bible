import dotenv from 'dotenv';
dotenv.config();

function parseList(value: string | undefined): string[] {
  if (!value || value.trim() === '' || value.trim() === '*') return [];
  return value.split(',').map(s => s.trim()).filter(Boolean);
}

export const config = {
  port: parseInt(process.env.PORT || '3000', 10),
  nodeEnv: process.env.NODE_ENV || 'development',
  rateLimitWindowMs: parseInt(process.env.RATE_LIMIT_WINDOW_MS || '900000', 10),
  rateLimitMax: parseInt(process.env.RATE_LIMIT_MAX || '100', 10),
  logLevel: process.env.LOG_LEVEL || 'info',

  // CORS: comma-separated allowed origins. Empty or '*' = allow all.
  corsOrigins: parseList(process.env.CORS_ORIGINS),

  // API Keys: comma-separated valid keys. Empty = no key required.
  apiKeys: parseList(process.env.API_KEYS),

  // Public paths that bypass API key auth
  publicPaths: ['/health', '/docs', '/'],
};
