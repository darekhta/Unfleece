import type { PAGE_SIZES } from '../util/pdf.js';
import { complementIndices } from '../util/ranges.js';
import { notifyProgress, type ProgressCallback } from '../progress.js';
import {
  wasmBooklet,
  wasmNUp,
  wasmPageCount,
  wasmRotateAll,
  wasmRotatePages,
  wasmSelectPages,
} from '../wasm/core.js';

/**
 * Build a new PDF containing only `indices` (0-based), in the given order.
 *
 * The Rust core's select_pages rewrites the page tree in place, so duplicate
 * indices are rejected here (upstream parsers — parsePageRanges, the
 * permutation check — never produce them).
 */
export async function extractPages(bytes: Uint8Array, indices: number[], onProgress?: ProgressCallback): Promise<Uint8Array> {
  if (indices.length === 0) throw new Error('No pages selected');
  if (new Set(indices).size !== indices.length) throw new Error('Duplicate pages selected');
  notifyProgress(onProgress, { phase: 'working', label: `Copying ${indices.length} selected pages…`, current: 0, total: indices.length });
  const out = await wasmSelectPages(bytes, indices);
  notifyProgress(onProgress, { phase: 'working', label: `Copied ${indices.length} selected pages.`, current: indices.length, total: indices.length });
  return out;
}

/** Remove the given 0-based page indices, keeping the rest. */
export async function removePages(bytes: Uint8Array, indices: number[], onProgress?: ProgressCallback): Promise<Uint8Array> {
  const keep = complementIndices(indices, await wasmPageCount(bytes));
  if (keep.length === 0) throw new Error('Cannot remove every page');
  return extractPages(bytes, keep, onProgress);
}

/** Reorder pages. `order` must be a permutation of all 0-based indices. */
export async function reorderPages(bytes: Uint8Array, order: number[], onProgress?: ProgressCallback): Promise<Uint8Array> {
  const n = await wasmPageCount(bytes);
  const sorted = [...order].sort((a, b) => a - b);
  const isPermutation = sorted.length === n && sorted.every((v, i) => v === i);
  if (!isPermutation) throw new Error('Order must be a permutation of all pages');
  return extractPages(bytes, order, onProgress);
}

/** Split into one single-page PDF per page. */
export async function splitToPages(bytes: Uint8Array, onProgress?: ProgressCallback): Promise<Uint8Array[]> {
  const out: Uint8Array[] = [];
  const total = await wasmPageCount(bytes);
  for (let i = 0; i < total; i++) {
    notifyProgress(onProgress, { phase: 'working', label: `Splitting page ${i + 1} of ${total}…`, current: i, total });
    out.push(await extractPages(bytes, [i]));
    notifyProgress(onProgress, { phase: 'working', label: `Split page ${i + 1} of ${total}.`, current: i + 1, total });
  }
  return out;
}

/** Split into one PDF per range. Each range is a list of 0-based indices. */
export async function splitByRanges(bytes: Uint8Array, ranges: number[][], onProgress?: ProgressCallback): Promise<Uint8Array[]> {
  if (ranges.length === 0) throw new Error('No ranges specified');
  const out: Uint8Array[] = [];
  for (const [i, range] of ranges.entries()) {
    notifyProgress(onProgress, { phase: 'working', label: `Creating range ${i + 1} of ${ranges.length}…`, current: i, total: ranges.length });
    out.push(await extractPages(bytes, range));
    notifyProgress(onProgress, { phase: 'working', label: `Created range ${i + 1} of ${ranges.length}.`, current: i + 1, total: ranges.length });
  }
  return out;
}

/**
 * Rotate pages by `angle` (multiple of 90), relative to current rotation.
 * `indices` omitted => all pages. Lossless (sets the /Rotate flag).
 *
 * Out-of-range indices surface the Rust core's 1-based message:
 * "Page number N could not be found".
 */
export async function rotatePdf(
  bytes: Uint8Array,
  angle: number,
  indices?: number[],
  onProgress?: ProgressCallback,
): Promise<Uint8Array> {
  if (angle % 90 !== 0) throw new Error('Rotation must be a multiple of 90°');
  const total = indices ? indices.length : await wasmPageCount(bytes);
  notifyProgress(onProgress, { phase: 'working', label: `Rotating ${total} page${total === 1 ? '' : 's'}…`, current: 0, total });
  const out = indices
    ? await wasmRotatePages(bytes, indices, angle)
    : await wasmRotateAll(bytes, angle);
  notifyProgress(onProgress, { phase: 'working', label: `Rotated ${total} page${total === 1 ? '' : 's'}.`, current: total, total });
  return out;
}

/** Grid layouts the Rust core supports (cols × rows per A4 sheet). */
const N_UP_PER_SHEET = new Set([2, 4, 6, 8, 9, 16]);

/** Place N source pages onto each output A4 page (handout / booklet-style). */
export async function nUpPdf(bytes: Uint8Array, perSheet: number, onProgress?: ProgressCallback): Promise<Uint8Array> {
  if (!N_UP_PER_SHEET.has(perSheet)) throw new Error(`Unsupported pages-per-sheet: ${perSheet}`);
  notifyProgress(onProgress, { phase: 'working', label: `Placing ${perSheet} pages per sheet…`, current: 0, total: 1 });
  const out = await wasmNUp(bytes, perSheet);
  notifyProgress(onProgress, { phase: 'working', label: 'Created N-up sheets.', current: 1, total: 1 });
  return out;
}

export interface BookletOptions {
  pageSize?: keyof typeof PAGE_SIZES;
  binding?: 'left' | 'right';
  margin?: number;
}

/**
 * Reorder pages into two-up booklet spreads on landscape sheets.
 * Defaults (applied in the Rust core): a4 sheets, left binding, 18pt margin;
 * the page count is padded up to a multiple of 4 with blank slots.
 */
export async function bookletPdf(bytes: Uint8Array, options: BookletOptions = {}, onProgress?: ProgressCallback): Promise<Uint8Array> {
  const pages = await wasmPageCount(bytes);
  if (pages === 0) throw new Error('No pages found');
  const spreads = Math.ceil(pages / 4) * 2;
  notifyProgress(onProgress, { phase: 'working', label: `Creating ${spreads} booklet spreads…`, current: 0, total: spreads });
  const opts: Record<string, unknown> = {};
  if (options.pageSize !== undefined) opts.pageSize = options.pageSize;
  if (options.binding !== undefined) opts.binding = options.binding;
  if (options.margin !== undefined) opts.margin = options.margin;
  const out = await wasmBooklet(bytes, opts);
  notifyProgress(onProgress, { phase: 'working', label: `Created ${spreads} booklet spreads.`, current: spreads, total: spreads });
  return out;
}
