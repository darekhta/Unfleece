// Browser bridge to the Rust→WASM `unfleece-core` engine. The wasm module is
// lazy-initialized on first use and cached. This is the start of Unfleece's
// owned "moat" (lopdf-based), distinct from the JS pdf-lib path.
import init, { optimize, page_count, pdfa_from_png_pages, rotate_all, select_pages } from './pkg/unfleece_core.js';
import wasmUrl from './pkg/unfleece_core_bg.wasm?url';

let ready: Promise<unknown> | null = null;
async function ensure(): Promise<void> {
  if (!ready) ready = init(wasmUrl);
  await ready;
}

/** Losslessly optimize a PDF (prune unused objects + flate-compress streams). */
export async function wasmOptimize(bytes: Uint8Array): Promise<Uint8Array> {
  await ensure();
  return optimize(bytes);
}

/** Rotate every page by `degrees`. */
export async function wasmRotateAll(bytes: Uint8Array, degrees: number): Promise<Uint8Array> {
  await ensure();
  return rotate_all(bytes, BigInt(degrees));
}

/** Count pages. */
export async function wasmPageCount(bytes: Uint8Array): Promise<number> {
  await ensure();
  return page_count(bytes);
}

/** Build a new PDF containing selected 0-based page indices, in order. */
export async function wasmSelectPages(bytes: Uint8Array, indices: number[] | Uint32Array): Promise<Uint8Array> {
  await ensure();
  return select_pages(bytes, indices instanceof Uint32Array ? indices : Uint32Array.from(indices));
}

/** Build a PDF/A-2b file from packed PNG page images. */
export async function wasmPdfAFromPngPages(pack: Uint8Array): Promise<Uint8Array> {
  await ensure();
  return pdfa_from_png_pages(pack);
}
