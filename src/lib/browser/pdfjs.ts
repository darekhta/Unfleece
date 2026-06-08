import * as pdfjsLib from 'pdfjs-dist/legacy/build/pdf.mjs';
import workerUrl from 'pdfjs-dist/legacy/build/pdf.worker.min.mjs?url';

pdfjsLib.GlobalWorkerOptions.workerSrc = workerUrl;

export { pdfjsLib };

/**
 * Clone bytes before they cross into the raster engine. pdf.js transfers and
 * detaches the ArrayBuffer it is handed, which empties the caller's view — so
 * anything that reuses those bytes afterward (e.g. handing them to the Rust
 * core for an OCR text layer) would get an empty buffer. This is THE engine
 * seam (see docs/01-architecture.md): the boundary between the object/generate
 * engine (Rust→WASM) and the render/recognize engine (pdf.js + Canvas). Clone
 * here, once, so no call site can ever detach a buffer it doesn't own.
 */
export function dataForPdfjs(bytes: Uint8Array): Uint8Array {
  return bytes.slice();
}

/** The single doorway to load a PDF into the raster engine (always clone-safe). */
export function loadPdfDocument(bytes: Uint8Array) {
  return pdfjsLib.getDocument({ data: dataForPdfjs(bytes) }).promise;
}
