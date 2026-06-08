import { describe, expect, it } from 'vitest';
import { dataForPdfjs } from '@lib/browser/pdfjs.js';

// The engine seam (docs/01): the boundary between the Rust→WASM object/generate
// engine and the pdf.js+Canvas render engine. pdf.js DETACHES the ArrayBuffer it
// receives, so the seam must hand it a *clone* — otherwise the caller's bytes are
// emptied and any later use (e.g. the Rust OCR text layer) gets nothing. That was
// a real production bug; this test pins the invariant that fixes it structurally.
describe('engine seam: dataForPdfjs', () => {
  it('returns a distinct buffer with identical contents', () => {
    const src = new Uint8Array([0x25, 0x50, 0x44, 0x46, 0x2d, 1, 2, 3]);
    const forPdfjs = dataForPdfjs(src);
    expect(forPdfjs).not.toBe(src);
    expect(forPdfjs.buffer).not.toBe(src.buffer);
    expect(Array.from(forPdfjs)).toEqual(Array.from(src));
  });

  it("detaching the clone (as pdf.js does) leaves the caller's bytes intact", () => {
    const src = new Uint8Array([10, 20, 30, 40]);
    const forPdfjs = dataForPdfjs(src);
    // structuredClone with transfer detaches forPdfjs.buffer — simulating pdf.js.
    structuredClone(forPdfjs.buffer, { transfer: [forPdfjs.buffer] });
    expect(forPdfjs.byteLength).toBe(0); // the clone was detached…
    expect(Array.from(src)).toEqual([10, 20, 30, 40]); // …but the original survives
  });
});
