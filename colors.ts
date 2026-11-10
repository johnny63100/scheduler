// Milky pastel palette (text on it is always dark; text in these colors is deepened, see `color-mix(... var(--text))`). Colors are stored as 0xRRGGBB integers in the database.
// (Older saved colors from the previous, brighter palette are moved over once; see db.rs `pastel_palette`.)
export const PALETTE: { name: string; value: number }[] = [
  { name: 'Lavender', value: 0xc3c8f4 },
  { name: 'Sky', value: 0xbfe3ee },
  { name: 'Mint', value: 0xc4e3cc },
  { name: 'Lemon', value: 0xf6ebb8 },
  { name: 'Peach', value: 0xf8d6c0 },
  { name: 'Rose', value: 0xf3c6d2 },
  { name: 'Grape', value: 0xdccaef },
  { name: 'Teal', value: 0xbfe2df },
  { name: 'Slate', value: 0xd6dde2 },
];

export const DEFAULT_COLOR = PALETTE[0].value;

export const hex = (n: number) => '#' + (n >>> 0).toString(16).padStart(6, '0').slice(-6);
export const fromHex = (s: string) => parseInt(s.replace('#', ''), 16);

/** Stable color for an arbitrary name (apps in charts). */
export function colorForName(name: string): string {
  let h = 5381;
  for (let i = 0; i < name.length; i++) h = ((h * 33) ^ name.charCodeAt(i)) >>> 0;
  return hex(PALETTE[h % PALETTE.length].value);
}

/** Cloth-cover colors for books without a cover image (TRPG and Reading shelves). */
const COVER_COLORS = ['#2f4a7a', '#7a2e3a', '#2f6b4f', '#b0812a', '#21757d', '#5b3f86', '#8a5a3c', '#3d4f63', '#a8466a'];

/** A stable cover color for a book title. */
export function coverColor(title: string): string {
  let h = 0x811c9dc5;
  for (const ch of title) h = Math.imul(h ^ ch.codePointAt(0)!, 0x01000193) >>> 0;
  return COVER_COLORS[h % COVER_COLORS.length];
}

/** Readable text color (dark or white) on a solid block of color `n`. */
export function textOn(n: number): string {
  const r = (n >> 16) & 255;
  const g = (n >> 8) & 255;
  const b = n & 255;
  return (r * 299 + g * 587 + b * 114) / 1000 > 150 ? '#1e2232' : '#ffffff';
}

/** `a` mixed into `b`: weight 1 gives `a`, 0 gives `b`. Both are 0xRRGGBB. */
export function mix(a: number, b: number, weight: number): number {
  const ch = (shift: number) => Math.round(((a >> shift) & 255) * weight + ((b >> shift) & 255) * (1 - weight));
  return (ch(16) << 16) | (ch(8) << 8) | ch(0);
}
