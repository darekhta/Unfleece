import { describe, it, expect, vi } from 'vitest';
import { imagesToPdf } from '@lib/tools/imagesToPdf.js';
import { listFormFields, fillForm, flattenForm } from '@lib/tools/forms.js';
import { stampImage, stampImageMany } from '@lib/tools/sign.js';
import { makeSamplePdf, makeFormPdf, pageCount, isPdf, PNG_1x1 } from './_fixtures.js';

describe('imagesToPdf', () => {
  it('creates one page per image (fit)', async () => {
    const out = await imagesToPdf([
      { bytes: PNG_1x1, type: 'png' },
      { bytes: PNG_1x1, type: 'png' },
    ]);
    expect(isPdf(out)).toBe(true);
    expect(await pageCount(out)).toBe(2);
  });

  it('reports image-to-pdf progress per image', async () => {
    const onProgress = vi.fn();

    await imagesToPdf([
      { bytes: PNG_1x1, type: 'png' },
      { bytes: PNG_1x1, type: 'png' },
    ], {}, onProgress);

    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ label: 'Adding image 1 of 2…', current: 0, total: 2 }));
    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ current: 2, total: 2 }));
  });

  it('supports a fixed page size', async () => {
    const out = await imagesToPdf([{ bytes: PNG_1x1, type: 'png' }], { pageSize: 'a4' });
    expect(await pageCount(out)).toBe(1);
  });

  it('throws with no images', async () => {
    await expect(imagesToPdf([])).rejects.toThrow();
  });
});

describe('forms', () => {
  it('lists fields with types', async () => {
    const fields = await listFormFields(await makeFormPdf());
    const byName = Object.fromEntries(fields.map((f) => [f.name, f.type]));
    expect(byName.fullName).toBe('text');
    expect(byName.agree).toBe('checkbox');
    expect(byName.plan).toBe('dropdown');
    expect(fields.find((f) => f.name === 'plan')?.options).toEqual(['Free', 'Pro', 'Team']);
  });

  it('fills and flattens', async () => {
    const filled = await fillForm(await makeFormPdf(), { fullName: 'Grace', agree: true, plan: 'Pro' });
    expect(isPdf(filled)).toBe(true);
    expect((await listFormFields(filled)).find((f) => f.name === 'plan')?.value).toBe('Pro');
    const flat = await flattenForm(filled);
    expect(await listFormFields(flat)).toHaveLength(0);
  });
});

describe('stampImage', () => {
  it('stamps a signature image onto the last page', async () => {
    const pdf = await makeSamplePdf(2);
    const out = await stampImage(pdf, {
      image: PNG_1x1,
      imageType: 'png',
      x: 50,
      y: 50,
      width: 100,
    });
    expect(isPdf(out)).toBe(true);
    expect(await pageCount(out)).toBe(2);
  });
});

describe('stampImageMany', () => {
  it('stamps multiple placements across different pages in one pass', async () => {
    const pdf = await makeSamplePdf(3);
    const out = await stampImageMany(pdf, [
      { image: PNG_1x1, imageType: 'png', pageIndex: 0, x: 40, y: 40, width: 80, height: 30 },
      { image: PNG_1x1, imageType: 'png', pageIndex: 2, x: 120, y: 200, width: 60, height: 20 },
      { image: PNG_1x1, imageType: 'png', pageIndex: 2, x: 10, y: 10, width: 50, height: 18 },
    ]);
    expect(isPdf(out)).toBe(true);
    expect(await pageCount(out)).toBe(3);
  });

  it('rejects an empty placement list and out-of-range pages', async () => {
    const pdf = await makeSamplePdf(1);
    await expect(stampImageMany(pdf, [])).rejects.toThrow(/No signatures/);
    await expect(
      stampImageMany(pdf, [{ image: PNG_1x1, imageType: 'png', pageIndex: 5, x: 0, y: 0, width: 10 }]),
    ).rejects.toThrow(/out of range/);
  });
});
