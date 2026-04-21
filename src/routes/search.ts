import { Router } from 'express';
import { search, randomVerse, verseOfTheDay } from '../controllers/searchController';

const router = Router();

router.get('/search', search);
router.get('/random', randomVerse);
router.get('/verse-of-the-day', verseOfTheDay);

export default router;
