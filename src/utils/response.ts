import type { Response } from 'express';
import type { ApiResponse } from '../types';

export function ok<T>(res: Response, data: T, meta?: Record<string, unknown>): void {
  const body: ApiResponse<T> = { status: 'ok', data };
  if (meta) body.meta = meta;
  res.json(body);
}

export function notFound(res: Response, message: string): void {
  res.status(404).json({ status: 'error', error: message } as ApiResponse);
}

export function badRequest(res: Response, message: string): void {
  res.status(400).json({ status: 'error', error: message } as ApiResponse);
}
