// Exercises the REAL Rust→WASM path behind stampImage / stampImageMany /
// imagesToPdf: byte passthrough, SMask emission, XObject dedupe, page-size
// math and the exact error wording the UI relies on.
import { describe, it, expect, vi } from 'vitest';
import { deflateSync } from 'node:zlib';
import { PDFDocument, PDFName, PDFDict } from '@cantoo/pdf-lib';
import { imagesToPdf } from '@lib/tools/imagesToPdf.js';
import { stampImage, stampImageMany } from '@lib/tools/sign.js';
import { makeSamplePdf, pageCount, isPdf, PNG_1x1 } from './_fixtures.js';

// ---------------------------------------------------------------------------
// Fixtures: hand-built PNGs (any size, optional alpha, seeded pixel noise so
// IDAT payloads are large and distinct) and a minimal baseline 1x1 JPEG.

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(bytes: Uint8Array): number {
  let c = 0xffffffff;
  for (const b of bytes) c = CRC_TABLE[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function pngChunk(type: string, data: Uint8Array): Uint8Array {
  const out = new Uint8Array(12 + data.length);
  const dv = new DataView(out.buffer);
  dv.setUint32(0, data.length, false);
  for (let i = 0; i < 4; i++) out[4 + i] = type.charCodeAt(i);
  out.set(data, 8);
  dv.setUint32(8 + data.length, crc32(out.subarray(4, 8 + data.length)), false);
  return out;
}

/** A valid 8-bit PNG (RGB or RGBA) filled with seeded pseudo-random pixels. */
function makePng(width: number, height: number, { alpha = false, seed = 1 } = {}): Uint8Array {
  const bpp = alpha ? 4 : 3;
  const raw = new Uint8Array(height * (1 + width * bpp));
  let state = seed >>> 0;
  // High LCG bits: the low bits are periodic and would deflate to nothing.
  const rand = () => ((state = (Math.imul(state, 1664525) + 1013904223) >>> 0), state >>> 24);
  let pos = 0;
  for (let y = 0; y < height; y++) {
    raw[pos++] = 0; // filter: None
    for (let x = 0; x < width; x++) {
      raw[pos++] = rand();
      raw[pos++] = rand();
      raw[pos++] = rand();
      if (alpha) raw[pos++] = 128; // semi-transparent => alpha is meaningful
    }
  }
  const ihdr = new Uint8Array(13);
  const dv = new DataView(ihdr.buffer);
  dv.setUint32(0, width, false);
  dv.setUint32(4, height, false);
  ihdr[8] = 8; // bit depth
  ihdr[9] = alpha ? 6 : 2; // color type: RGBA / RGB
  const sig = Uint8Array.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const idat = pngChunk('IDAT', new Uint8Array(deflateSync(raw)));
  const parts = [sig, pngChunk('IHDR', ihdr), idat, pngChunk('IEND', new Uint8Array(0))];
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let off = 0;
  for (const p of parts) {
    out.set(p, off);
    off += p.length;
  }
  return out;
}

/** A minimal fully-valid 1x1 baseline JPEG (both DQTs + all Huffman tables). */
const JPEG_1x1: Uint8Array = Uint8Array.from(
  atob(
    '/9j/4AAQSkZJRgABAQEAYABgAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/2wBDAQkJCQwLDBgNDRgyIRwhMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjL/wAARCAABAAEDASIAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/9oADAMBAAIRAxEAPwD3+iiigD//2Q==',
  ),
  (c) => c.charCodeAt(0),
);

/** Find a byte subsequence (naive search; inputs are small). */
function indexOfBytes(haystack: Uint8Array, needle: Uint8Array): number {
  outer: for (let i = 0; i <= haystack.length - needle.length; i++) {
    for (let j = 0; j < needle.length; j++) {
      if (haystack[i + j] !== needle[j]) continue outer;
    }
    return i;
  }
  return -1;
}

function containsText(bytes: Uint8Array, text: string): boolean {
  return indexOfBytes(bytes, new TextEncoder().encode(text)) !== -1;
}

/** True when the page's /Resources has a non-empty /XObject dict. */
async function pageHasXObject(bytes: Uint8Array, pageIdx: number): Promise<boolean> {
  const doc = await PDFDocument.load(bytes, { ignoreEncryption: true });
  const res = doc.getPage(pageIdx).node.Resources();
  if (!res) return false;
  const xo = res.lookup(PDFName.of('XObject'));
  return xo instanceof PDFDict && xo.keys().length > 0;
}

async function pageSize(bytes: Uint8Array, pageIdx: number): Promise<{ width: number; height: number }> {
  const doc = await PDFDocument.load(bytes, { ignoreEncryption: true });
  return doc.getPage(pageIdx).getSize();
}

// ---------------------------------------------------------------------------

describe('stampImage (rust core)', () => {
  it('embeds JPEGs byte-for-byte with DCTDecode (no re-encoding)', async () => {
    const out = await stampImage(await makeSamplePdf(1), {
      image: JPEG_1x1,
      imageType: 'jpg',
      x: 20,
      y: 20,
      width: 80,
    });
    expect(isPdf(out)).toBe(true);
    expect(containsText(out, 'DCTDecode')).toBe(true);
    expect(indexOfBytes(out, JPEG_1x1)).toBeGreaterThan(-1);
  });

  it('splits PNG alpha into an /SMask', async () => {
    const out = await stampImage(await makeSamplePdf(1), {
      image: makePng(8, 8, { alpha: true }),
      imageType: 'png',
      x: 10,
      y: 10,
      width: 40,
    });
    expect(containsText(out, 'SMask')).toBe(true);
  });

  it('opaque RGB PNGs do not produce an /SMask', async () => {
    const out = await stampImage(await makeSamplePdf(1), {
      image: makePng(8, 8),
      imageType: 'png',
      x: 10,
      y: 10,
      width: 40,
    });
    expect(containsText(out, 'SMask')).toBe(false);
  });

  it('defaults to the LAST page when pageIndex is omitted', async () => {
    const out = await stampImage(await makeSamplePdf(3), {
      image: PNG_1x1,
      imageType: 'png',
      x: 50,
      y: 50,
      width: 100,
    });
    expect(await pageCount(out)).toBe(3);
    expect(await pageHasXObject(out, 0)).toBe(false);
    expect(await pageHasXObject(out, 1)).toBe(false);
    expect(await pageHasXObject(out, 2)).toBe(true);
  });

  it('honours an explicit pageIndex', async () => {
    const out = await stampImage(await makeSamplePdf(3), {
      image: PNG_1x1,
      imageType: 'png',
      pageIndex: 0,
      x: 50,
      y: 50,
      width: 100,
    });
    expect(await pageHasXObject(out, 0)).toBe(true);
    expect(await pageHasXObject(out, 2)).toBe(false);
  });

  it('rejects an out-of-range page with the exact TS wording', async () => {
    await expect(
      stampImage(await makeSamplePdf(2), { image: PNG_1x1, imageType: 'png', pageIndex: 5, x: 0, y: 0, width: 10 }),
    ).rejects.toThrow('Page index 5 out of range');
  });

  it('preserves odd page sizes', async () => {
    const out = await stampImage(await makeSamplePdf(2, [123.45, 678.9]), {
      image: PNG_1x1,
      imageType: 'png',
      x: 5,
      y: 5,
      width: 30,
    });
    const { width, height } = await pageSize(out, 1);
    expect(width).toBeCloseTo(123.45, 1);
    expect(height).toBeCloseTo(678.9, 1);
  });
});

describe('stampImageMany (rust core)', () => {
  it('dedupes identical image bytes to a single XObject', async () => {
    const pdf = await makeSamplePdf(2);
    const png = makePng(50, 50, { seed: 7 }); // ~7.5KB of incompressible IDAT
    const single = await stampImageMany(pdf, [
      { image: png, imageType: 'png', pageIndex: 0, x: 10, y: 10, width: 50 },
    ]);
    const double = await stampImageMany(pdf, [
      { image: png, imageType: 'png', pageIndex: 0, x: 10, y: 10, width: 50 },
      { image: png, imageType: 'png', pageIndex: 1, x: 90, y: 90, width: 50 },
    ]);
    // Same payload twice => one embedded image; only a tiny draw-op delta.
    expect(double.length - single.length).toBeLessThan(400);
  });

  it('embeds DISTINCT image bytes separately', async () => {
    const pdf = await makeSamplePdf(2);
    const a = makePng(50, 50, { seed: 7 });
    const b = makePng(50, 50, { seed: 99 });
    const single = await stampImageMany(pdf, [
      { image: a, imageType: 'png', pageIndex: 0, x: 10, y: 10, width: 50 },
    ]);
    const distinct = await stampImageMany(pdf, [
      { image: a, imageType: 'png', pageIndex: 0, x: 10, y: 10, width: 50 },
      { image: b, imageType: 'png', pageIndex: 1, x: 90, y: 90, width: 50 },
    ]);
    // The second payload (~7.5KB raw) must actually be present.
    expect(distinct.length - single.length).toBeGreaterThan(3000);
  });

  it('stamps across multiple pages in one pass', async () => {
    const out = await stampImageMany(await makeSamplePdf(3), [
      { image: PNG_1x1, imageType: 'png', pageIndex: 0, x: 40, y: 40, width: 80 },
      { image: PNG_1x1, imageType: 'png', pageIndex: 2, x: 120, y: 200, width: 60 },
    ]);
    expect(await pageHasXObject(out, 0)).toBe(true);
    expect(await pageHasXObject(out, 1)).toBe(false);
    expect(await pageHasXObject(out, 2)).toBe(true);
  });

  it('rejects an empty placement list with the exact wording', async () => {
    await expect(stampImageMany(await makeSamplePdf(1), [])).rejects.toThrow('No signatures placed');
  });

  it('rejects out-of-range placements with the exact wording (Rust-side)', async () => {
    await expect(
      stampImageMany(await makeSamplePdf(2), [
        { image: PNG_1x1, imageType: 'png', pageIndex: 9, x: 0, y: 0, width: 10 },
      ]),
    ).rejects.toThrow('Page index 9 out of range');
  });
});

describe('imagesToPdf (rust core)', () => {
  it('fit: page size equals the image pixel size (px as pt)', async () => {
    const out = await imagesToPdf([{ bytes: makePng(10, 20), type: 'png' }], { pageSize: 'fit' });
    const { width, height } = await pageSize(out, 0);
    expect(width).toBeCloseTo(10, 1);
    expect(height).toBeCloseTo(20, 1);
  });

  it('a4: fixed 595.28 x 841.89 pt pages regardless of image size', async () => {
    const out = await imagesToPdf(
      [
        { bytes: makePng(10, 20), type: 'png' },
        { bytes: makePng(2000, 3000, { seed: 3 }), type: 'png' },
      ],
      { pageSize: 'a4' },
    );
    expect(await pageCount(out)).toBe(2);
    for (const i of [0, 1]) {
      const { width, height } = await pageSize(out, i);
      expect(width).toBeCloseTo(595.28, 1);
      expect(height).toBeCloseTo(841.89, 1);
    }
  });

  it('letter: fixed 612 x 792 pt pages', async () => {
    const out = await imagesToPdf([{ bytes: makePng(30, 40), type: 'png' }], { pageSize: 'letter' });
    const { width, height } = await pageSize(out, 0);
    expect(width).toBeCloseTo(612, 1);
    expect(height).toBeCloseTo(792, 1);
  });

  it('clamps absurd margins instead of failing', async () => {
    const huge = await imagesToPdf([{ bytes: makePng(30, 40), type: 'png' }], { pageSize: 'a4', margin: 10000 });
    expect(isPdf(huge)).toBe(true);
    expect(await pageCount(huge)).toBe(1);
    const negative = await imagesToPdf([{ bytes: makePng(30, 40), type: 'png' }], { pageSize: 'a4', margin: -50 });
    expect(isPdf(negative)).toBe(true);
  });

  it('mixed PNG + JPEG input; the JPEG passes through byte-for-byte', async () => {
    const out = await imagesToPdf([
      { bytes: makePng(5, 5, { alpha: true }), type: 'png' },
      { bytes: JPEG_1x1, type: 'jpg' },
    ]);
    expect(await pageCount(out)).toBe(2);
    expect(indexOfBytes(out, JPEG_1x1)).toBeGreaterThan(-1);
    expect(containsText(out, 'SMask')).toBe(true);
  });

  it('rejects empty input with the exact wording', async () => {
    await expect(imagesToPdf([])).rejects.toThrow('No images provided');
  });

  it('reports a Rust-core start label and a completion tick', async () => {
    const onProgress = vi.fn();
    await imagesToPdf([{ bytes: PNG_1x1, type: 'png' }], {}, onProgress);
    expect(onProgress).toHaveBeenCalledWith(
      expect.objectContaining({ label: 'Building a 1-page PDF…', current: 0, total: 1 }),
    );
    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ current: 1, total: 1 }));
  });
});
