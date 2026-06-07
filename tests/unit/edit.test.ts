import { describe, it, expect } from 'vitest';
import { addPageNumbers, addTextWatermark } from '@lib/tools/annotate.js';
import { cropPdf } from '@lib/tools/crop.js';
import { getMetadata, setMetadata, stripMetadata } from '@lib/tools/metadata.js';
import { makeSamplePdf, pageCount, load, isPdf } from './_fixtures.js';

describe('annotate', () => {
  it('addPageNumbers keeps page count and stays valid', async () => {
    const pdf = await makeSamplePdf(3);
    const out = await addPageNumbers(pdf, { format: '{n} / {total}' });
    expect(isPdf(out)).toBe(true);
    expect(await pageCount(out)).toBe(3);
  });

  it('addPageNumbers reports per-page progress', async () => {
    const pdf = await makeSamplePdf(3);
    const events: string[] = [];
    await addPageNumbers(pdf, { format: '{n}' }, (progress) => events.push(progress.label));

    expect(events).toContain('Adding page number 1 of 3…');
    expect(events).toContain('Adding page number 3 of 3…');
    expect(events.at(-1)).toBe('Saving numbered PDF…');
  });

  it('addTextWatermark requires text', async () => {
    const pdf = await makeSamplePdf(1);
    // @ts-expect-error intentionally missing text
    await expect(addTextWatermark(pdf, {})).rejects.toThrow();
  });

  it('addTextWatermark produces a valid PDF', async () => {
    const pdf = await makeSamplePdf(2);
    const out = await addTextWatermark(pdf, { text: 'CONFIDENTIAL' });
    expect(isPdf(out)).toBe(true);
    expect(await pageCount(out)).toBe(2);
  });

  it('addTextWatermark reports per-page progress', async () => {
    const pdf = await makeSamplePdf(2);
    const events: string[] = [];
    await addTextWatermark(pdf, { text: 'CONFIDENTIAL' }, (progress) => events.push(progress.label));

    expect(events).toContain('Watermarking page 1 of 2…');
    expect(events).toContain('Watermarking page 2 of 2…');
    expect(events.at(-1)).toBe('Saving watermarked PDF…');
  });
});

describe('cropPdf', () => {
  it('shrinks the crop box', async () => {
    const pdf = await makeSamplePdf(1, [300, 400]);
    const out = await cropPdf(pdf, { left: 20, right: 20, top: 30, bottom: 30 });
    const doc = await load(out);
    const box = doc.getPage(0).getCropBox();
    expect(box.width).toBeCloseTo(260, 1);
    expect(box.height).toBeCloseTo(340, 1);
  });

  it('reports per-page crop progress', async () => {
    const pdf = await makeSamplePdf(2, [300, 400]);
    const events: string[] = [];
    await cropPdf(pdf, { left: 10 }, (progress) => events.push(progress.label));

    expect(events).toContain('Cropping page 1 of 2…');
    expect(events).toContain('Cropping page 2 of 2…');
    expect(events.at(-1)).toBe('Saving cropped PDF…');
  });

  it('rejects margins larger than the page', async () => {
    const pdf = await makeSamplePdf(1, [300, 400]);
    await expect(cropPdf(pdf, { left: 200, right: 200 })).rejects.toThrow();
  });
});

describe('metadata', () => {
  it('round-trips set/get', async () => {
    const pdf = await makeSamplePdf(1);
    const out = await setMetadata(pdf, {
      title: 'My Doc',
      author: 'Ada',
      keywords: ['a', 'b'],
    });
    const meta = await getMetadata(out);
    expect(meta.title).toBe('My Doc');
    expect(meta.author).toBe('Ada');
    expect(meta.keywords).toEqual(['a', 'b']);
    expect(meta.pageCount).toBe(1);
  });

  it('stripMetadata clears fields', async () => {
    const pdf = await setMetadata(await makeSamplePdf(1), { title: 'X', author: 'Y' });
    const meta = await getMetadata(await stripMetadata(pdf));
    expect(meta.title).toBeFalsy();
    expect(meta.author).toBeFalsy();
  });
});
