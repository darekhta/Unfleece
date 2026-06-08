import { describe, it, expect } from 'vitest';
import {
  PDFDocument,
  PDFName,
  PDFDict,
  PDFNumber,
  PDFArray,
  PDFRawStream,
  decodePDFRawStream,
} from '@cantoo/pdf-lib';
import { addPageNumbers, addTextWatermark, type NumberPosition } from '@lib/tools/annotate.js';
import { cropPdf } from '@lib/tools/crop.js';
import { makeSamplePdf, pageCount, isPdf } from './_fixtures.js';

// ---------------------------------------------------------------------------
// Helpers: decode the (flate-compressed) content streams of a wasm-produced
// PDF so tests can assert on the actual operators the Rust core emitted.
// ---------------------------------------------------------------------------

/** All decoded content streams of one page, concatenated (latin1). */
async function contentOf(bytes: Uint8Array, pageIndex = 0): Promise<string> {
  const doc = await PDFDocument.load(bytes);
  const page = doc.getPage(pageIndex);
  const contents = page.node.lookup(PDFName.of('Contents'));
  const streams: PDFRawStream[] = [];
  if (contents instanceof PDFRawStream) streams.push(contents);
  else if (contents instanceof PDFArray) {
    for (let i = 0; i < contents.size(); i++) {
      const obj = page.node.context.lookup(contents.get(i));
      if (obj instanceof PDFRawStream) streams.push(obj);
    }
  }
  return streams
    .map((s) => Buffer.from(decodePDFRawStream(s).decode()).toString('latin1'))
    .join('\n');
}

/** Operands of the LAST `Tm` operator (the stamp is always appended last). */
function lastTm(content: string): number[] {
  const matches = [...content.matchAll(/(-?[\d.]+) (-?[\d.]+) (-?[\d.]+) (-?[\d.]+) (-?[\d.]+) (-?[\d.]+) Tm/g)];
  expect(matches.length).toBeGreaterThan(0);
  return matches[matches.length - 1].slice(1, 7).map(Number);
}

/** Operands of the LAST `rg` (fill colour) operator. */
function lastRg(content: string): number[] {
  const matches = [...content.matchAll(/(-?[\d.]+) (-?[\d.]+) (-?[\d.]+) rg/g)];
  expect(matches.length).toBeGreaterThan(0);
  return matches[matches.length - 1].slice(1, 4).map(Number);
}

/** `/ca` alpha of the first ExtGState reachable from a page's Resources. */
async function extGStateAlpha(bytes: Uint8Array, pageIndex = 0): Promise<number> {
  const doc = await PDFDocument.load(bytes);
  const page = doc.getPage(pageIndex);
  const res = page.node.Resources();
  const gs = res?.lookup(PDFName.of('ExtGState'), PDFDict);
  expect(gs).toBeDefined();
  const [, first] = gs!.entries()[0];
  const gd = page.node.context.lookup(first, PDFDict);
  return gd.lookup(PDFName.of('ca'), PDFNumber).asNumber();
}

/** A PDF whose pages have different sizes (pdf-lib fixture builder). */
async function makeMixedSizePdf(sizes: [number, number][]): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  for (const size of sizes) doc.addPage(size);
  return doc.save();
}

// ---------------------------------------------------------------------------
// addPageNumbers — real Rust wasm path
// ---------------------------------------------------------------------------

describe('addPageNumbers (Rust core)', () => {
  it('places the number correctly in all six zones', async () => {
    // Page 300x400; '1' at 10 pt Helvetica = 5.56 pt wide; margin 24.
    const cases: [NumberPosition, number, number][] = [
      ['bottom-center', 147.22, 24],
      ['bottom-right', 270.44, 24],
      ['bottom-left', 24, 24],
      ['top-center', 147.22, 366],
      ['top-right', 270.44, 366],
      ['top-left', 24, 366],
    ];
    const pdf = await makeSamplePdf(1);
    for (const [position, x, y] of cases) {
      const out = await addPageNumbers(pdf, { position });
      const tm = lastTm(await contentOf(out));
      expect(tm[4], `${position} x`).toBeCloseTo(x, 1);
      expect(tm[5], `${position} y`).toBeCloseTo(y, 1);
    }
  });

  it('substitutes {n} and {total} in the format template', async () => {
    const out = await addPageNumbers(await makeSamplePdf(3), { format: 'Page {n} of {total}' });
    expect(await contentOf(out, 1)).toContain('(Page 2 of 3)');
  });

  it('drives Bates numbering via prefix format, startAt and padTo', async () => {
    const out = await addPageNumbers(await makeSamplePdf(2), {
      format: 'CASE-{n}',
      position: 'bottom-right',
      startAt: 100,
      padTo: 6,
    });
    expect(await contentOf(out, 0)).toContain('(CASE-000100)');
    expect(await contentOf(out, 1)).toContain('(CASE-000101)');
  });

  it('floors fractional padTo like JS Math.floor', async () => {
    const out = await addPageNumbers(await makeSamplePdf(1), { padTo: 3.9 });
    expect(await contentOf(out, 0)).toContain('(001)');
  });

  it('degrades non-WinAnsi characters to ? and maps WinAnsi extras', async () => {
    // U+2116 (№) is outside WinAnsi -> '?'.
    const out = await addPageNumbers(await makeSamplePdf(1), { format: '№{n}' });
    expect(await contentOf(out, 0)).toContain('(?1)');
    // U+2013 (en dash) maps to WinAnsi byte 0x96 (raw or octal-escaped).
    const dashed = await addPageNumbers(await makeSamplePdf(1), { format: '–{n}' });
    expect(await contentOf(dashed, 0)).toMatch(/\((\x96|\\226)1\)/);
  });

  it('rejects an invalid position instead of silently falling back', async () => {
    const pdf = await makeSamplePdf(1);
    await expect(
      addPageNumbers(pdf, { position: 'middle' as NumberPosition }),
    ).rejects.toThrow(/Invalid position 'middle'/);
  });

  it('honors fontSize and margin in the placement math', async () => {
    const out = await addPageNumbers(await makeSamplePdf(1), { position: 'top-left', fontSize: 8, margin: 10 });
    const tm = lastTm(await contentOf(out));
    expect(tm[4]).toBeCloseTo(10, 1); // x = margin
    expect(tm[5]).toBeCloseTo(400 - 10 - 8, 1); // y = height - margin - fontSize
  });

  it('stamps every page of a 5-page document with its own number', async () => {
    const out = await addPageNumbers(await makeSamplePdf(5));
    expect(isPdf(out)).toBe(true);
    expect(await pageCount(out)).toBe(5);
    for (let i = 0; i < 5; i++) {
      expect(await contentOf(out, i)).toContain(`(${i + 1})`);
    }
  });

  it('centers correctly on odd page sizes', async () => {
    const out = await addPageNumbers(await makeSamplePdf(1, [123.45, 678.9]));
    const tm = lastTm(await contentOf(out));
    expect(tm[4]).toBeCloseTo((123.45 - 5.56) / 2, 1);
    expect(tm[5]).toBeCloseTo(24, 1);
  });
});

// ---------------------------------------------------------------------------
// addTextWatermark — real Rust wasm path
// ---------------------------------------------------------------------------

describe('addTextWatermark (Rust core)', () => {
  it('rejects empty or missing text with the exact message', async () => {
    const pdf = await makeSamplePdf(1);
    await expect(addTextWatermark(pdf, { text: '' })).rejects.toThrow('Watermark text is required');
    // @ts-expect-error intentionally missing text
    await expect(addTextWatermark(pdf, {})).rejects.toThrow('Watermark text is required');
  });

  it('writes a 0.25-opacity ExtGState by default and draws the text', async () => {
    const out = await addTextWatermark(await makeSamplePdf(1), { text: 'DRAFT' });
    expect(await extGStateAlpha(out)).toBeCloseTo(0.25, 2);
    const content = await contentOf(out);
    expect(content).toContain('(DRAFT)');
    expect(content).toMatch(/\/UFgs\d* gs/); // opacity applied via gs op
  });

  it('clamps opacity into [0, 1]', async () => {
    const pdf = await makeSamplePdf(1);
    const over = await addTextWatermark(pdf, { text: 'X', opacity: 5 });
    expect(await extGStateAlpha(over)).toBe(1);
    const under = await addTextWatermark(pdf, { text: 'X', opacity: -3 });
    expect(await extGStateAlpha(under)).toBe(0);
  });

  it('anchors an angle-0 watermark at the page center via Tm', async () => {
    // 'WM' at 50 pt Helvetica-Bold = (944 + 833) / 1000 * 50 = 88.85 pt wide.
    const out = await addTextWatermark(await makeSamplePdf(1), { text: 'WM', angle: 0 });
    const tm = lastTm(await contentOf(out));
    // Note: -sin(0) serializes as '-0', so compare numerically.
    for (const [i, v] of [1, 0, 0, 1].entries()) expect(tm[i]).toBeCloseTo(v, 5);
    expect(tm[4]).toBeCloseTo(150 - 88.85 / 2, 1);
    expect(tm[5]).toBeCloseTo(200, 1);
  });

  it('rotates by the default 45° through the text matrix', async () => {
    const out = await addTextWatermark(await makeSamplePdf(1), { text: 'WM' });
    const tm = lastTm(await contentOf(out));
    const r = Math.SQRT1_2;
    expect(tm[0]).toBeCloseTo(r, 3);
    expect(tm[1]).toBeCloseTo(r, 3);
    expect(tm[2]).toBeCloseTo(-r, 3);
    expect(tm[3]).toBeCloseTo(r, 3);
    expect(tm[4]).toBeCloseTo(150 - (88.85 / 2) * r, 1);
    expect(tm[5]).toBeCloseTo(200 - (88.85 / 2) * r, 1);
  });

  it('clamps colour channels in the rg operator', async () => {
    const out = await addTextWatermark(await makeSamplePdf(1), {
      text: 'X',
      color: { r: 5, g: -1, b: 0.5 },
    });
    expect(lastRg(await contentOf(out))).toEqual([1, 0, 0.5]);
  });

  it('degrades unicode text to ? but keeps WinAnsi accents', async () => {
    const pdf = await makeSamplePdf(1);
    const cyrillic = await addTextWatermark(pdf, { text: 'Привіт' });
    expect(await contentOf(cyrillic)).toContain('(??????)');
    const accented = await addTextWatermark(pdf, { text: 'Café' });
    expect(await contentOf(accented)).toMatch(/\(Caf(é|\\351)\)/);
  });

  it('stamps every page and keeps the PDF valid', async () => {
    const out = await addTextWatermark(await makeSamplePdf(3), { text: 'SECRET' });
    expect(isPdf(out)).toBe(true);
    expect(await pageCount(out)).toBe(3);
    for (let i = 0; i < 3; i++) {
      expect(await contentOf(out, i)).toContain('(SECRET)');
    }
  });
});

// ---------------------------------------------------------------------------
// cropPdf — real Rust wasm path
// ---------------------------------------------------------------------------

describe('cropPdf (Rust core)', () => {
  it('sets the CropBox and leaves the MediaBox untouched', async () => {
    const out = await cropPdf(await makeSamplePdf(1, [300, 400]), { left: 20, right: 20, top: 30, bottom: 30 });
    const page = (await PDFDocument.load(out)).getPage(0);
    const media = page.getMediaBox();
    expect(media.width).toBeCloseTo(300, 1);
    expect(media.height).toBeCloseTo(400, 1);
    const crop = page.getCropBox();
    expect(crop.x).toBeCloseTo(20, 1);
    expect(crop.y).toBeCloseTo(30, 1);
    expect(crop.width).toBeCloseTo(260, 1);
    expect(crop.height).toBeCloseTo(340, 1);
  });

  it('defaults missing margins to zero (CropBox equals MediaBox)', async () => {
    const out = await cropPdf(await makeSamplePdf(1, [300, 400]), {});
    const page = (await PDFDocument.load(out)).getPage(0);
    const crop = page.getCropBox();
    expect(crop).toEqual({ x: 0, y: 0, width: 300, height: 400 });
  });

  it('rejects negative margins with the exact message', async () => {
    const pdf = await makeSamplePdf(1);
    await expect(cropPdf(pdf, { left: -1 })).rejects.toThrow('Crop margins cannot be negative');
    await expect(cropPdf(pdf, { top: -0.001 })).rejects.toThrow('Crop margins cannot be negative');
  });

  it('rejects non-finite margins', async () => {
    const pdf = await makeSamplePdf(1);
    await expect(cropPdf(pdf, { top: Number.NaN })).rejects.toThrow('Crop margins must be finite numbers');
    await expect(cropPdf(pdf, { right: Number.POSITIVE_INFINITY })).rejects.toThrow('Crop margins must be finite numbers');
  });

  it('rejects margins that exceed or exactly consume the page', async () => {
    const pdf = await makeSamplePdf(1, [300, 400]);
    await expect(cropPdf(pdf, { left: 200, right: 200 })).rejects.toThrow('Crop margins exceed page size');
    // 150 + 150 == 300 -> a zero-width page is also rejected.
    await expect(cropPdf(pdf, { left: 150, right: 150 })).rejects.toThrow('Crop margins exceed page size');
  });

  it('crops every page of a mixed-size document independently', async () => {
    const pdf = await makeMixedSizePdf([[300, 400], [200, 500]]);
    const out = await cropPdf(pdf, { left: 10, top: 10 });
    const doc = await PDFDocument.load(out);
    const crop0 = doc.getPage(0).getCropBox();
    expect(crop0.x).toBeCloseTo(10, 1);
    expect(crop0.width).toBeCloseTo(290, 1);
    expect(crop0.height).toBeCloseTo(390, 1);
    const crop1 = doc.getPage(1).getCropBox();
    expect(crop1.width).toBeCloseTo(190, 1);
    expect(crop1.height).toBeCloseTo(490, 1);
    // MediaBoxes untouched on both pages.
    expect(doc.getPage(0).getMediaBox().width).toBeCloseTo(300, 1);
    expect(doc.getPage(1).getMediaBox().width).toBeCloseTo(200, 1);
  });

  it('fails when any page of a mixed-size document is too small', async () => {
    const pdf = await makeMixedSizePdf([[300, 400], [100, 100]]);
    await expect(cropPdf(pdf, { left: 150 })).rejects.toThrow('Crop margins exceed page size');
  });
});
