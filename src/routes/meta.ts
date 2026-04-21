import { Router } from 'express';
import { getTestaments, getStats, healthCheck } from '../controllers/metaController';

const router = Router();

router.get('/testaments', getTestaments);
router.get('/stats', getStats);
router.get('/health', healthCheck);

export default router;
