import { describe, it, expect } from 'vitest';
import { parsePageRanges, complementIndices } from '@lib/util/ranges.js';

describe('parsePageRanges', () => {
  it('parses single pages and ranges into 0-based indices', () => {
    expect(parsePageRanges('1-3, 5, 8-10', 10)).toEqual([0, 1, 2, 4, 7, 8, 9]);
  });

  it('de-duplicates and preserves order of appearance', () => {
    expect(parsePageRanges('3,1,3,2', 5)).toEqual([2, 0, 1]);
  });

  it('normalizes reversed ranges', () => {
    expect(parsePageRanges('3-1', 5)).toEqual([0, 1, 2]);
  });

  it('throws on out-of-range pages', () => {
    expect(() => parsePageRanges('1-6', 5)).toThrow(/out of range/);
  });

  it('throws on malformed input', () => {
    expect(() => parsePageRanges('1-a', 5)).toThrow(/Invalid/);
    expect(() => parsePageRanges('', 5)).toThrow(/No pages/);
  });
});

describe('complementIndices', () => {
  it('returns the indices not excluded', () => {
    expect(complementIndices([1, 3], 5)).toEqual([0, 2, 4]);
  });
});
