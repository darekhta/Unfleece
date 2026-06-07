import { describe, expect, it } from 'vitest';
import { normalizedRedaction, redactionToPixels } from '@lib/browser/redact.js';

describe('redaction helpers', () => {
  it('normalizes dragged rectangles and clamps to the page', () => {
    const rect = normalizedRedaction({ pageIndex: 2.8, x: 0.8, y: 0.7, w: -0.4, h: -0.2 });
    expect(rect?.pageIndex).toBe(2);
    expect(rect?.x).toBeCloseTo(0.4);
    expect(rect?.y).toBeCloseTo(0.5);
    expect(rect?.w).toBeCloseTo(0.4);
    expect(rect?.h).toBeCloseTo(0.2);
    expect(normalizedRedaction({ pageIndex: 0, x: 0.1, y: 0.1, w: 0.001, h: 0.2 })).toBeNull();
  });

  it('converts fractional redactions to expanded pixel rectangles', () => {
    expect(redactionToPixels({ pageIndex: 0, x: 0.25, y: 0.2, w: 0.5, h: 0.4 }, 400, 500, 2)).toEqual({
      x: 98,
      y: 98,
      w: 204,
      h: 204,
    });
  });
});
