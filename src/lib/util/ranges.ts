/**
 * Parse a human page-range string like "1-3, 5, 8-10" into a list of 0-based
 * page indices, in order of appearance, de-duplicated. 1-based input.
 *
 * @throws if a token is malformed or a page is out of [1, pageCount].
 */
export function parsePageRanges(input: string, pageCount: number): number[] {
  const result: number[] = [];
  const seen = new Set<number>();

  const push = (oneBased: number) => {
    const idx = oneBased - 1;
    if (!Number.isInteger(oneBased) || idx < 0 || idx >= pageCount) {
      throw new Error(`Page ${oneBased} is out of range (document has ${pageCount} pages)`);
    }
    if (!seen.has(idx)) {
      seen.add(idx);
      result.push(idx);
    }
  };

  const parts = input
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);

  if (parts.length === 0) {
    throw new Error('No pages specified');
  }

  for (const part of parts) {
    const range = part.match(/^(\d+)\s*-\s*(\d+)$/);
    if (range) {
      let a = parseInt(range[1], 10);
      let b = parseInt(range[2], 10);
      if (a > b) [a, b] = [b, a];
      for (let i = a; i <= b; i++) push(i);
    } else if (/^\d+$/.test(part)) {
      push(parseInt(part, 10));
    } else {
      throw new Error(`Invalid page range: "${part}"`);
    }
  }

  return result;
}

/** All 0-based indices NOT in `exclude`. */
export function complementIndices(exclude: number[], pageCount: number): number[] {
  const ex = new Set(exclude);
  const out: number[] = [];
  for (let i = 0; i < pageCount; i++) if (!ex.has(i)) out.push(i);
  return out;
}
