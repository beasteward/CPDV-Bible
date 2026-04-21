import { config } from './config';
import { bibleStore } from './services/bibleStore';
import app from './app';
import pino from 'pino';

const logger = pino({ level: config.logLevel });

// Load Bible data
logger.info('Loading Bible data...');
const start = Date.now();
bibleStore.load();
const stats = bibleStore.getStats();
logger.info(`Bible data loaded in ${Date.now() - start}ms: ${stats.books} books, ${stats.chapters} chapters, ${stats.verses} verses`);

// Start server
app.listen(config.port, () => {
  logger.info(`CPDV Bible API running on port ${config.port} (${config.nodeEnv})`);
});
