import { describe, it, expect } from 'vitest';
import { viewToPdf, pdfToView, cropMargins, stampRect, clampRect, snapRectToGuides, type StageDims } from '@lib/browser/stageCoords.js';

// A 300x400pt page rendered at 600px wide => scale 2, cssHeight 800.
const d: StageDims = { scale: 2, cssWidth: 600, cssHeight: 800, pageWidthPt: 300, pageHeightPt: 400 };

describe('stageCoords', () => {
  it('viewToPdf flips Y and divides by scale', () => {
    expect(viewToPdf(0, 0, d)).toEqual({ x: 0, y: 400 }); // top-left screen = top-left page
    expect(viewToPdf(600, 800, d)).toEqual({ x: 300, y: 0 }); // bottom-right screen = bottom-right page
    expect(viewToPdf(300, 400, d)).toEqual({ x: 150, y: 200 }); // center
  });

  it('pdfToView is the inverse of viewToPdf', () => {
    for (const [vx, vy] of [[0, 0], [120, 250], [600, 800], [333, 111]]) {
      const p = viewToPdf(vx, vy, d);
      const back = pdfToView(p.x, p.y, d);
      expect(back.x).toBeCloseTo(vx, 6);
      expect(back.y).toBeCloseTo(vy, 6);
    }
  });

  it('cropMargins computes per-edge margins in points', () => {
    // rect inset 40px on each side => 20pt margins all around
    const m = cropMargins({ x: 40, y: 40, w: 520, h: 720 }, d);
    expect(m.left).toBeCloseTo(20);
    expect(m.top).toBeCloseTo(20);
    expect(m.right).toBeCloseTo(20);
    expect(m.bottom).toBeCloseTo(20);
  });

  it('stampRect gives bottom-left origin in points', () => {
    // a 100x50px box at (200,100) => x100,w50,h25; bottom edge at screen 150 => pdf y = 400-75 = 325
    const s = stampRect({ x: 200, y: 100, w: 100, h: 50 }, d);
    expect(s.x).toBeCloseTo(100);
    expect(s.width).toBeCloseTo(50);
    expect(s.height).toBeCloseTo(25);
    expect(s.y).toBeCloseTo(400 - 75); // page height - (y+h)/scale
  });

  it('clampRect keeps the rect inside the page with a minimum size', () => {
    expect(clampRect({ x: -50, y: -50, w: 100, h: 100 }, d)).toMatchObject({ x: 0, y: 0 });
    const big = clampRect({ x: 0, y: 0, w: 9999, h: 9999 }, d);
    expect(big.w).toBe(600);
    expect(big.h).toBe(800);
    const tiny = clampRect({ x: 10, y: 10, w: 2, h: 2 }, d, 16);
    expect(tiny.w).toBe(16);
  });

  it('snapRectToGuides snaps a rect center to page center guides', () => {
    const snapped = snapRectToGuides({ x: 246, y: 374, w: 100, h: 50 }, d, 8);
    expect(snapped.rect.x).toBe(250);
    expect(snapped.rect.y).toBe(375);
    expect(snapped.guides).toEqual({ vertical: 300, horizontal: 400 });
  });

  it('snapRectToGuides leaves distant rects alone', () => {
    const snapped = snapRectToGuides({ x: 40, y: 60, w: 100, h: 50 }, d, 8);
    expect(snapped.rect).toEqual({ x: 40, y: 60, w: 100, h: 50 });
    expect(snapped.guides).toEqual({});
  });
});
