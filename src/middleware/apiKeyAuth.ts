import type { Request, Response, NextFunction } from 'express';
import { config } from '../config';

/**
 * API Key authentication middleware.
 * - If no API_KEYS are configured, all requests pass through (open mode).
 * - Public paths (health, docs, root) are always exempt.
 * - Valid key must be sent via X-API-Key header.
 */
export function apiKeyAuth(req: Request, res: Response, next: NextFunction): void {
  // No keys configured = open access
  if (config.apiKeys.length === 0) return next();

  // Public paths bypass auth
  const path = req.path;
  if (config.publicPaths.some(p => path === p || path.startsWith(p + '/'))) {
    // Special case: /docs needs prefix match, others are exact
    if (path === '/' || path === '/health' || path.startsWith('/docs')) {
      return next();
    }
  }

  const apiKey = req.header('X-API-Key');

  if (!apiKey) {
    res.status(401).json({
      status: 'error',
      error: 'Missing API key. Provide a valid key via the X-API-Key header.',
    });
    return;
  }

  if (!config.apiKeys.includes(apiKey)) {
    res.status(403).json({
      status: 'error',
      error: 'Invalid API key.',
    });
    return;
  }

  next();
}
