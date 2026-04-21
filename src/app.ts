import express from 'express';
import cors from 'cors';
import helmet from 'helmet';
import pinoHttp from 'pino-http';
import swaggerUi from 'swagger-ui-express';
import { config } from './config';
import { rateLimiter } from './middleware/rateLimiter';
import { errorHandler } from './middleware/errorHandler';
import { openApiSpec } from './docs/openapi';
import booksRouter from './routes/books';
import searchRouter from './routes/search';
import metaRouter from './routes/meta';

const app = express();

// Security & utility middleware
app.use(helmet());
app.use(cors({ origin: config.corsOrigin }));
app.use(express.json());
app.use(pinoHttp({ level: config.logLevel }));
app.use(rateLimiter);

// API docs
app.use('/docs', swaggerUi.serve, swaggerUi.setup(openApiSpec, {
  customCss: '.swagger-ui .topbar { display: none }',
  customSiteTitle: 'CPDV Bible API Docs',
}));

// Routes
app.use('/api/v1/books', booksRouter);
app.use('/api/v1', searchRouter);
app.use('/api/v1', metaRouter);

// Root redirect
app.get('/', (_req, res) => {
  res.json({
    status: 'ok',
    data: {
      name: 'CPDV Bible API',
      version: '1.0.0',
      docs: '/docs',
    },
  });
});

// 404 handler
app.use((_req, res) => {
  res.status(404).json({ status: 'error', error: 'Not found' });
});

// Error handler
app.use(errorHandler);

export default app;
