import type { Request, Response } from 'express';
import { bibleStore } from '../services/bibleStore';
import { ok } from '../utils/response';

export function getTestaments(_req: Request, res: Response): void {
  ok(res, bibleStore.getTestaments());
}

export function getStats(_req: Request, res: Response): void {
  ok(res, bibleStore.getStats());
}

export function healthCheck(_req: Request, res: Response): void {
  ok(res, { status: 'healthy', uptime: process.uptime() });
}
