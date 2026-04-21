import { Router } from 'express';
import { listBooks, getBook, listChapters, getChapter, getVerse } from '../controllers/booksController';

const router = Router();

router.get('/', listBooks);
router.get('/:book', getBook);
router.get('/:book/chapters', listChapters);
router.get('/:book/:chapter', getChapter);
router.get('/:book/:chapter/:verse', getVerse);

export default router;
