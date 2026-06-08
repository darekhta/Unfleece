// Edge-case tests that drive the REAL Rust→WASM path for merge + organize.
// Page identity is made observable through distinct page widths; booklet slot
// placement is verified by decoding the sheet content stream's `cm` operator.
import { describe, it, expect } from 'vitest';
import { inflateSync } from 'node:zlib';
import { PDFDocument, PDFName, PDFRawStream } from '@cantoo/pdf-lib';
import { mergePdfs } from '@lib/tools/merge.js';
import {
  extractPages,
  removePages,
  rotatePdf,
  splitToPages,
  splitByRanges,
  nUpPdf,
  bookletPdf,
} from '@lib/tools/organize.js';
import { makeSamplePdf, pageCount, load, isPdf } from './_fixtures.js';

/** A PDF whose page i is widths[i] × 500pt, so page order is observable. */
async function makeVariedPdf(widths: number[]): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  for (const w of widths) doc.addPage([w, 500]);
  return doc.save();
}

/** A truly zero-page PDF (pdf-lib's save() adds a default page otherwise). */
async function makeEmptyPdf(): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  return doc.save({ addDefaultPage: false });
}

async function pageWidths(bytes: Uint8Array): Promise<number[]> {
  const doc = await load(bytes);
  return doc.getPages().map((p) => p.getWidth());
}

/** Decoded content-stream text of output page n (inflates Flate streams). */
async function pageContentText(bytes: Uint8Array, n: number): Promise<string> {
  const doc = await load(bytes);
  const contents = doc.getPage(n).node.Contents();
  if (!(contents instanceof PDFRawStream)) throw new Error('expected a single raw content stream');
  const raw = contents.getContents();
  const data = contents.dict.has(PDFName.of('Filter')) ? inflateSync(raw) : raw;
  return Buffer.from(data).toString('latin1');
}

/** Translate-x of the first `cm` (placement matrix) operator on a page. */
function firstCmX(content: string): number {
  const m = content.match(/(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s+cm/);
  if (!m) throw new Error(`no cm operator in: ${content}`);
  return Number(m[5]);
}

describe('mergePdfs (Rust core)', () => {
  it('merges three documents in input order, observable via page sizes', async () => {
    const out = await mergePdfs([
      await makeVariedPdf([300]),
      await makeVariedPdf([400, 410]),
      await makeVariedPdf([500]),
    ]);
    expect(isPdf(out)).toBe(true);
    expect(await pageWidths(out)).toEqual([300, 400, 410, 500]);
  });

  it('merged output can be merged again', async () => {
    const once = await mergePdfs([await makeVariedPdf([310]), await makeVariedPdf([320])]);
    const twice = await mergePdfs([once, await makeVariedPdf([330])]);
    expect(await pageWidths(twice)).toEqual([310, 320, 330]);
  });

  it('rejects invalid PDF bytes with a friendly message', async () => {
    const good = await makeSamplePdf(1);
    const bad = new Uint8Array([1, 2, 3, 4]);
    await expect(mergePdfs([good, bad])).rejects.toThrow(/Invalid PDF document/);
  });

  it('rejects an empty file list with the TS wording', async () => {
    await expect(mergePdfs([])).rejects.toThrow('No files to merge');
  });

  it('errors when every input has zero pages', async () => {
    const empty = await makeEmptyPdf();
    await expect(mergePdfs([empty])).rejects.toThrow('Merge produced zero pages');
  });
});

describe('extract/remove/split (Rust core)', () => {
  it('extractPages keeps the requested order', async () => {
    const pdf = await makeVariedPdf([301, 302, 303, 304, 305]);
    expect(await pageWidths(await extractPages(pdf, [3, 0]))).toEqual([304, 301]);
  });

  it('extractPages surfaces the Rust 1-based out-of-range message', async () => {
    const pdf = await makeSamplePdf(3);
    await expect(extractPages(pdf, [5])).rejects.toThrow('Page number 6 could not be found');
  });

  it('extractPages rejects duplicate indices before reaching wasm', async () => {
    const pdf = await makeSamplePdf(3);
    await expect(extractPages(pdf, [1, 1])).rejects.toThrow('Duplicate pages selected');
  });

  it('removePages keeps the complement in order', async () => {
    const pdf = await makeVariedPdf([311, 312, 313, 314]);
    expect(await pageWidths(await removePages(pdf, [0, 2]))).toEqual([312, 314]);
  });

  it('splitByRanges produces real per-range documents', async () => {
    const pdf = await makeVariedPdf([321, 322, 323, 324, 325]);
    const parts = await splitByRanges(pdf, [[0, 1], [2, 3, 4]]);
    expect(parts).toHaveLength(2);
    expect(await pageWidths(parts[0])).toEqual([321, 322]);
    expect(await pageWidths(parts[1])).toEqual([323, 324, 325]);
  });

  it('splitToPages preserves odd fractional page sizes', async () => {
    const pdf = await makeVariedPdf([123.45, 678.9]);
    const parts = await splitToPages(pdf);
    expect(parts).toHaveLength(2);
    expect((await pageWidths(parts[0]))[0]).toBeCloseTo(123.45, 1);
    expect((await pageWidths(parts[1]))[0]).toBeCloseTo(678.9, 1);
  });
});

describe('rotatePdf (Rust core)', () => {
  it('rotates only the selected pages', async () => {
    const pdf = await makeSamplePdf(3);
    const doc = await load(await rotatePdf(pdf, 90, [0, 2]));
    expect(doc.getPage(0).getRotation().angle).toBe(90);
    expect(doc.getPage(1).getRotation().angle).toBe(0);
    expect(doc.getPage(2).getRotation().angle).toBe(90);
  });

  it('accumulates rotation across calls and wraps at 360', async () => {
    const pdf = await makeSamplePdf(1);
    const once = await rotatePdf(pdf, 270);
    const twice = await rotatePdf(once, 180); // 270 + 180 = 450 -> 90
    expect((await load(twice)).getPage(0).getRotation().angle).toBe(90);
  });

  it('normalizes negative angles', async () => {
    const pdf = await makeSamplePdf(1);
    expect((await load(await rotatePdf(pdf, -90))).getPage(0).getRotation().angle).toBe(270);
  });

  it('surfaces the Rust 1-based message for an out-of-range page', async () => {
    const pdf = await makeSamplePdf(2);
    await expect(rotatePdf(pdf, 90, [5])).rejects.toThrow('Page number 6 could not be found');
  });

  it('rejects non-multiples of 90 before reaching wasm', async () => {
    const pdf = await makeSamplePdf(1);
    await expect(rotatePdf(pdf, 45)).rejects.toThrow('Rotation must be a multiple of 90°');
  });
});

describe('nUpPdf (Rust core)', () => {
  it('lays out a partial last sheet on A4', async () => {
    const pdf = await makeSamplePdf(7);
    const out = await nUpPdf(pdf, 4);
    const doc = await load(out);
    expect(doc.getPageCount()).toBe(2); // 4 + 3 (partial last sheet)
    expect(doc.getPage(1).getWidth()).toBeCloseTo(595.28, 1);
    expect(doc.getPage(1).getHeight()).toBeCloseTo(841.89, 1);
  });

  it('supports the 8, 9 and 16 grids', async () => {
    expect(await pageCount(await nUpPdf(await makeSamplePdf(8), 8))).toBe(1);
    expect(await pageCount(await nUpPdf(await makeSamplePdf(9), 9))).toBe(1);
    expect(await pageCount(await nUpPdf(await makeSamplePdf(16), 16))).toBe(1);
  });

  it('rejects unsupported per-sheet counts in TS', async () => {
    const pdf = await makeSamplePdf(2);
    await expect(nUpPdf(pdf, 3)).rejects.toThrow('Unsupported pages-per-sheet: 3');
  });
});

describe('bookletPdf (Rust core)', () => {
  // A lone page lands in the RIGHT slot of spread [3, 0] when left-bound, and
  // in the LEFT slot when right-bound. A4 landscape sheet is 841.89pt wide,
  // so the slot is identified by which half of the sheet the placement hits.
  it('right binding swaps the slots on each spread', async () => {
    const pdf = await makeVariedPdf([300]);
    const left = await bookletPdf(pdf, { binding: 'left' });
    const right = await bookletPdf(pdf, { binding: 'right' });
    expect(await pageCount(left)).toBe(2);
    expect(await pageCount(right)).toBe(2);
    expect(firstCmX(await pageContentText(left, 0))).toBeGreaterThan(420);
    expect(firstCmX(await pageContentText(right, 0))).toBeLessThan(420);
  });

  it('passes margin through (zero margin reaches the sheet edge)', async () => {
    // A square 300×300 page is width-limited in the 420.9×595.3 cell, so it
    // fills the cell width and sits flush at x = margin = 0.
    const doc = await PDFDocument.create();
    doc.addPage([300, 300]);
    const out = await bookletPdf(await doc.save(), { binding: 'right', margin: 0 });
    expect(firstCmX(await pageContentText(out, 0))).toBeCloseTo(0, 1);
  });

  it('pads to a multiple of four on landscape A4 by default', async () => {
    const pdf = await makeSamplePdf(6); // padded to 8 -> 4 sheets
    const doc = await load(await bookletPdf(pdf));
    expect(doc.getPageCount()).toBe(4);
    expect(doc.getPage(0).getWidth()).toBeCloseTo(841.89, 1);
    expect(doc.getPage(0).getHeight()).toBeCloseTo(595.28, 1);
  });

  it('errors on an empty document', async () => {
    const empty = await makeEmptyPdf();
    await expect(bookletPdf(empty)).rejects.toThrow('No pages found');
  });
});
