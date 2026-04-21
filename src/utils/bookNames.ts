// Canonical book order with testament groupings
export const BOOK_ORDER: Array<{ key: string; testament: 'OT' | 'NT' }> = [
  // Old Testament (46 books - Catholic canon)
  { key: 'Genesis', testament: 'OT' },
  { key: 'Exodus', testament: 'OT' },
  { key: 'Leviticus', testament: 'OT' },
  { key: 'Numbers', testament: 'OT' },
  { key: 'Deuteronomy', testament: 'OT' },
  { key: 'Joshua', testament: 'OT' },
  { key: 'Judges', testament: 'OT' },
  { key: 'Ruth', testament: 'OT' },
  { key: '1-Samuel', testament: 'OT' },
  { key: '2-Samuel', testament: 'OT' },
  { key: '1-Kings', testament: 'OT' },
  { key: '2-Kings', testament: 'OT' },
  { key: '1-Chronicles', testament: 'OT' },
  { key: '2-Chronicles', testament: 'OT' },
  { key: 'Ezra', testament: 'OT' },
  { key: 'Nehemiah', testament: 'OT' },
  { key: 'Tobit', testament: 'OT' },
  { key: 'Judith', testament: 'OT' },
  { key: 'Esther', testament: 'OT' },
  { key: 'Job', testament: 'OT' },
  { key: 'Psalms', testament: 'OT' },
  { key: 'Proverbs', testament: 'OT' },
  { key: 'Ecclesiastes', testament: 'OT' },
  { key: 'Song2', testament: 'OT' },
  { key: 'Wisdom', testament: 'OT' },
  { key: 'Sirach', testament: 'OT' },
  { key: 'Isaiah', testament: 'OT' },
  { key: 'Jeremiah', testament: 'OT' },
  { key: 'Lamentations', testament: 'OT' },
  { key: 'Baruch', testament: 'OT' },
  { key: 'Ezekiel', testament: 'OT' },
  { key: 'Daniel', testament: 'OT' },
  { key: 'Hosea', testament: 'OT' },
  { key: 'Joel', testament: 'OT' },
  { key: 'Amos', testament: 'OT' },
  { key: 'Obadiah', testament: 'OT' },
  { key: 'Jonah', testament: 'OT' },
  { key: 'Micah', testament: 'OT' },
  { key: 'Nahum', testament: 'OT' },
  { key: 'Habakkuk', testament: 'OT' },
  { key: 'Zephaniah', testament: 'OT' },
  { key: 'Haggai', testament: 'OT' },
  { key: 'Zechariah', testament: 'OT' },
  { key: 'Malachi', testament: 'OT' },
  { key: '1-Maccabees', testament: 'OT' },
  { key: '2-Maccabees', testament: 'OT' },
  // New Testament (27 books)
  { key: 'Matthew', testament: 'NT' },
  { key: 'Mark', testament: 'NT' },
  { key: 'Luke', testament: 'NT' },
  { key: 'John', testament: 'NT' },
  { key: 'Acts', testament: 'NT' },
  { key: 'Romans', testament: 'NT' },
  { key: '1-Corinthians', testament: 'NT' },
  { key: '2-Corinthians', testament: 'NT' },
  { key: 'Galatians', testament: 'NT' },
  { key: 'Ephesians', testament: 'NT' },
  { key: 'Philippians', testament: 'NT' },
  { key: 'Colossians', testament: 'NT' },
  { key: '1-Thessalonians', testament: 'NT' },
  { key: '2-Thessalonians', testament: 'NT' },
  { key: '1-Timothy', testament: 'NT' },
  { key: '2-Timothy', testament: 'NT' },
  { key: 'Titus', testament: 'NT' },
  { key: 'Philemon', testament: 'NT' },
  { key: 'Hebrews', testament: 'NT' },
  { key: 'James', testament: 'NT' },
  { key: '1-Peter', testament: 'NT' },
  { key: '2-Peter', testament: 'NT' },
  { key: '1-John', testament: 'NT' },
  { key: '2-John', testament: 'NT' },
  { key: '3-John', testament: 'NT' },
  { key: 'Jude', testament: 'NT' },
  { key: 'Revelation', testament: 'NT' },
];

// Build alias map: lowercased variant → canonical key
const ALIAS_MAP = new Map<string, string>();

function addAlias(alias: string, key: string): void {
  ALIAS_MAP.set(alias.toLowerCase().replace(/[\s-]/g, ''), key);
}

for (const { key } of BOOK_ORDER) {
  // Exact key normalized
  addAlias(key, key);
  // Without hyphens
  addAlias(key.replace(/-/g, ''), key);
  // With spaces instead of hyphens
  addAlias(key.replace(/-/g, ' '), key);
}

// Special aliases
const EXTRA_ALIASES: Record<string, string> = {
  'gen': 'Genesis',
  'ex': 'Exodus',
  'lev': 'Leviticus',
  'num': 'Numbers',
  'deut': 'Deuteronomy',
  'josh': 'Joshua',
  'judg': 'Judges',
  'sam': '1-Samuel',
  '1sam': '1-Samuel',
  '2sam': '2-Samuel',
  '1kgs': '1-Kings',
  '1kings': '1-Kings',
  '2kgs': '2-Kings',
  '2kings': '2-Kings',
  '1chr': '1-Chronicles',
  '1chron': '1-Chronicles',
  '1chronicles': '1-Chronicles',
  '2chr': '2-Chronicles',
  '2chron': '2-Chronicles',
  '2chronicles': '2-Chronicles',
  'neh': 'Nehemiah',
  'tob': 'Tobit',
  'jdt': 'Judith',
  'est': 'Esther',
  'ps': 'Psalms',
  'psalm': 'Psalms',
  'prov': 'Proverbs',
  'eccl': 'Ecclesiastes',
  'eccles': 'Ecclesiastes',
  'qoheleth': 'Ecclesiastes',
  'song': 'Song2',
  'songofsolomon': 'Song2',
  'songofsongs': 'Song2',
  'canticle': 'Song2',
  'canticles': 'Song2',
  'canticleofcanticles': 'Song2',
  'sos': 'Song2',
  'wis': 'Wisdom',
  'wisdomofsolomon': 'Wisdom',
  'sir': 'Sirach',
  'ecclesiasticus': 'Sirach',
  'isa': 'Isaiah',
  'jer': 'Jeremiah',
  'lam': 'Lamentations',
  'bar': 'Baruch',
  'ezek': 'Ezekiel',
  'eze': 'Ezekiel',
  'dan': 'Daniel',
  'hos': 'Hosea',
  'ob': 'Obadiah',
  'jon': 'Jonah',
  'mic': 'Micah',
  'nah': 'Nahum',
  'hab': 'Habakkuk',
  'zeph': 'Zephaniah',
  'hag': 'Haggai',
  'zech': 'Zechariah',
  'mal': 'Malachi',
  '1macc': '1-Maccabees',
  '1maccabees': '1-Maccabees',
  '2macc': '2-Maccabees',
  '2maccabees': '2-Maccabees',
  'matt': 'Matthew',
  'mt': 'Matthew',
  'mk': 'Mark',
  'lk': 'Luke',
  'jn': 'John',
  'joh': 'John',
  'rom': 'Romans',
  '1cor': '1-Corinthians',
  '1corinthians': '1-Corinthians',
  '2cor': '2-Corinthians',
  '2corinthians': '2-Corinthians',
  'gal': 'Galatians',
  'eph': 'Ephesians',
  'phil': 'Philippians',
  'php': 'Philippians',
  'col': 'Colossians',
  '1thess': '1-Thessalonians',
  '1thessalonians': '1-Thessalonians',
  '2thess': '2-Thessalonians',
  '2thessalonians': '2-Thessalonians',
  '1tim': '1-Timothy',
  '1timothy': '1-Timothy',
  '2tim': '2-Timothy',
  '2timothy': '2-Timothy',
  'tit': 'Titus',
  'phlm': 'Philemon',
  'phm': 'Philemon',
  'heb': 'Hebrews',
  'jas': 'James',
  '1pet': '1-Peter',
  '1peter': '1-Peter',
  '2pet': '2-Peter',
  '2peter': '2-Peter',
  '1jn': '1-John',
  '1john': '1-John',
  '2jn': '2-John',
  '2john': '2-John',
  '3jn': '3-John',
  '3john': '3-John',
  'rev': 'Revelation',
  'apocalypse': 'Revelation',
};

for (const [alias, key] of Object.entries(EXTRA_ALIASES)) {
  addAlias(alias, key);
}

/**
 * Resolve a user-provided book name to the canonical JSON key.
 * Returns undefined if no match found.
 */
export function resolveBookName(input: string): string | undefined {
  const normalized = input.toLowerCase().replace(/[\s-]/g, '');
  return ALIAS_MAP.get(normalized);
}

/**
 * Get the display name for a book key (converts hyphens to spaces, handles Song2).
 */
export function displayName(key: string): string {
  if (key === 'Song2') return 'Song of Songs';
  return key.replace(/-/g, ' ');
}
