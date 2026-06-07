import { describe, expect, it } from 'vitest';
import { findContentBounds } from '@lib/browser/autoCrop.js';

function whitePixels(width: number, height: number): Uint8ClampedArray {
  const data = new Uint8ClampedArray(width * height * 4);
  for (let i = 0; i < data.length; i += 4) {
    data[i] = 255;
    data[i + 1] = 255;
    data[i + 2] = 255;
    data[i + 3] = 255;
  }
  return data;
}

describe('findContentBounds', () => {
  it('returns null for a blank white page', () => {
    expect(findContentBounds(whitePixels(4, 4), 4, 4)).toBeNull();
  });

  it('finds the bounds of non-white pixels', () => {
    const data = whitePixels(5, 5);
    for (const [x, y] of [[1, 2], [3, 4]]) {
      const i = (y * 5 + x) * 4;
      data[i] = 0;
      data[i + 1] = 0;
      data[i + 2] = 0;
    }

    expect(findContentBounds(data, 5, 5)).toEqual({ minX: 1, minY: 2, maxX: 3, maxY: 4 });
  });
});
