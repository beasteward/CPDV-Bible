import type { Request, Response, NextFunction } from 'express';
import type { ApiResponse } from '../types';

export function errorHandler(err: Error, _req: Request, res: Response, _next: NextFunction): void {
  const statusCode = res.statusCode !== 200 ? res.statusCode : 500;
  const response: ApiResponse = {
    status: 'error',
    error: err.message || 'Internal Server Error',
  };
  res.status(statusCode).json(response);
}
