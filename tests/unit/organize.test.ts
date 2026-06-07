import { describe, it, expect, vi } from 'vitest';
import { mergePdfs } from '@lib/tools/merge.js';
import {
  extractPages,
  removePages,
  reorderPages,
  splitToPages,
  splitByRanges,
  rotatePdf,
  nUpPdf,
  bookletPdf,
} from '@lib/tools/organize.js';
import { makeSamplePdf, pageCount, load, isPdf } from './_fixtures.js';

describe('mergePdfs', () => {
  it('concatenates pages from all inputs', async () => {
    const a = await makeSamplePdf(3);
    const b = await makeSamplePdf(2);
    const out = await mergePdfs([a, b]);
    expect(isPdf(out)).toBe(true);
    expect(await pageCount(out)).toBe(5);
  });

  it('reports merge progress per input file', async () => {
    const onProgress = vi.fn();
    const a = await makeSamplePdf(1);
    const b = await makeSamplePdf(1);

    await mergePdfs([a, b], onProgress);

    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ current: 0, total: 2 }));
    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ current: 2, total: 2 }));
  });

  it('throws with no files', async () => {
    await expect(mergePdfs([])).rejects.toThrow();
  });
});

describe('organize', () => {
  it('extractPages keeps only selected pages', async () => {
    const pdf = await makeSamplePdf(5);
    expect(await pageCount(await extractPages(pdf, [0, 2, 4]))).toBe(3);
  });

  it('removePages drops selected pages', async () => {
    const pdf = await makeSamplePdf(5);
    expect(await pageCount(await removePages(pdf, [1, 3]))).toBe(3);
  });

  it('removePages refuses to remove every page', async () => {
    const pdf = await makeSamplePdf(2);
    await expect(removePages(pdf, [0, 1])).rejects.toThrow();
  });

  it('reorderPages requires a full permutation', async () => {
    const pdf = await makeSamplePdf(3);
    expect(await pageCount(await reorderPages(pdf, [2, 0, 1]))).toBe(3);
    await expect(reorderPages(pdf, [0, 1])).rejects.toThrow(/permutation/);
  });

  it('splitToPages yields one PDF per page', async () => {
    const pdf = await makeSamplePdf(4);
    const parts = await splitToPages(pdf);
    expect(parts).toHaveLength(4);
    for (const p of parts) expect(await pageCount(p)).toBe(1);
  });

  it('reports split progress per page', async () => {
    const pdf = await makeSamplePdf(3);
    const onProgress = vi.fn();

    await splitToPages(pdf, onProgress);

    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ label: 'Splitting page 1 of 3…', current: 0, total: 3 }));
    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ current: 3, total: 3 }));
  });

  it('splitByRanges yields one PDF per range', async () => {
    const pdf = await makeSamplePdf(6);
    const parts = await splitByRanges(pdf, [[0, 1, 2], [3, 4, 5]]);
    expect(parts).toHaveLength(2);
    expect(await pageCount(parts[0])).toBe(3);
  });

  it('rotatePdf sets the page rotation', async () => {
    const pdf = await makeSamplePdf(2);
    const out = await rotatePdf(pdf, 90);
    const doc = await load(out);
    expect(doc.getPage(0).getRotation().angle).toBe(90);
    expect(doc.getPage(1).getRotation().angle).toBe(90);
  });

  it('reports rotate progress per targeted page', async () => {
    const pdf = await makeSamplePdf(2);
    const onProgress = vi.fn();

    await rotatePdf(pdf, 90, undefined, onProgress);

    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ current: 0, total: 2 }));
    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ current: 2, total: 2 }));
  });

  it('rotatePdf can target specific pages', async () => {
    const pdf = await makeSamplePdf(2);
    const out = await rotatePdf(pdf, 180, [1]);
    const doc = await load(out);
    expect(doc.getPage(0).getRotation().angle).toBe(0);
    expect(doc.getPage(1).getRotation().angle).toBe(180);
  });

  it('rotatePdf rejects non-multiples of 90', async () => {
    const pdf = await makeSamplePdf(1);
    await expect(rotatePdf(pdf, 45)).rejects.toThrow();
  });

  it('nUpPdf reduces page count', async () => {
    const pdf = await makeSamplePdf(8);
    const out = await nUpPdf(pdf, 4);
    expect(isPdf(out)).toBe(true);
    expect(await pageCount(out)).toBe(2);
  });

  it('bookletPdf pads to folded spreads on landscape sheets', async () => {
    const pdf = await makeSamplePdf(5);
    const out = await bookletPdf(pdf, { pageSize: 'letter' });
    const doc = await load(out);
    expect(isPdf(out)).toBe(true);
    expect(doc.getPageCount()).toBe(4);
    expect(doc.getPage(0).getWidth()).toBeGreaterThan(doc.getPage(0).getHeight());
  });

  it('bookletPdf reports progress per spread', async () => {
    const pdf = await makeSamplePdf(4);
    const onProgress = vi.fn();

    await bookletPdf(pdf, {}, onProgress);

    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ label: 'Creating booklet spread 1 of 2…', current: 0, total: 2 }));
    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ current: 2, total: 2 }));
  });
});
