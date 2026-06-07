import { PDFDocument, degrees, type PDFEmbeddedPage, type PDFPage } from '@cantoo/pdf-lib';
import { loadPdf, PAGE_SIZES } from '../util/pdf.js';
import { complementIndices } from '../util/ranges.js';
import { notifyProgress, type ProgressCallback } from '../progress.js';
import { wasmPageCount, wasmSelectPages } from '../wasm/core.js';

async function pageCount(bytes: Uint8Array): Promise<number> {
  try {
    return await wasmPageCount(bytes);
  } catch {
    return (await loadPdf(bytes)).getPageCount();
  }
}

async function extractPagesWithPdfLib(bytes: Uint8Array, indices: number[], onProgress?: ProgressCallback): Promise<Uint8Array> {
  const src = await loadPdf(bytes);
  const out = await PDFDocument.create();
  notifyProgress(onProgress, { phase: 'working', label: `Copying ${indices.length} selected pages with pdf-lib…`, current: 0, total: indices.length });
  const pages = await out.copyPages(src, indices);
  for (const [i, page] of pages.entries()) {
    out.addPage(page);
    notifyProgress(onProgress, { phase: 'working', label: `Copied page ${i + 1} of ${indices.length}.`, current: i + 1, total: indices.length });
  }
  return out.save();
}

/** Build a new PDF containing only `indices` (0-based), in the given order. */
export async function extractPages(bytes: Uint8Array, indices: number[], onProgress?: ProgressCallback): Promise<Uint8Array> {
  if (indices.length === 0) throw new Error('No pages selected');
  notifyProgress(onProgress, { phase: 'working', label: `Copying ${indices.length} selected pages with Rust core…`, current: 0, total: indices.length });
  try {
    const out = await wasmSelectPages(bytes, indices);
    notifyProgress(onProgress, { phase: 'working', label: `Copied ${indices.length} selected pages.`, current: indices.length, total: indices.length });
    return out;
  } catch {
    notifyProgress(onProgress, { phase: 'working', label: 'Rust page copy failed; using pdf-lib fallback…' });
    return extractPagesWithPdfLib(bytes, indices, onProgress);
  }
}

/** Remove the given 0-based page indices, keeping the rest. */
export async function removePages(bytes: Uint8Array, indices: number[], onProgress?: ProgressCallback): Promise<Uint8Array> {
  const keep = complementIndices(indices, await pageCount(bytes));
  if (keep.length === 0) throw new Error('Cannot remove every page');
  return extractPages(bytes, keep, onProgress);
}

/** Reorder pages. `order` must be a permutation of all 0-based indices. */
export async function reorderPages(bytes: Uint8Array, order: number[], onProgress?: ProgressCallback): Promise<Uint8Array> {
  const n = await pageCount(bytes);
  const sorted = [...order].sort((a, b) => a - b);
  const isPermutation = sorted.length === n && sorted.every((v, i) => v === i);
  if (!isPermutation) throw new Error('Order must be a permutation of all pages');
  return extractPages(bytes, order, onProgress);
}

/** Split into one single-page PDF per page. */
export async function splitToPages(bytes: Uint8Array, onProgress?: ProgressCallback): Promise<Uint8Array[]> {
  const out: Uint8Array[] = [];
  const total = await pageCount(bytes);
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
 */
export async function rotatePdf(
  bytes: Uint8Array,
  angle: number,
  indices?: number[],
  onProgress?: ProgressCallback,
): Promise<Uint8Array> {
  if (angle % 90 !== 0) throw new Error('Rotation must be a multiple of 90°');
  const doc = await loadPdf(bytes);
  const pages = doc.getPages();
  const targets = indices ?? pages.map((_, i) => i);
  for (const [done, i] of targets.entries()) {
    notifyProgress(onProgress, { phase: 'working', label: `Rotating page ${done + 1} of ${targets.length}…`, current: done, total: targets.length });
    const page = pages[i];
    if (!page) throw new Error(`Page index ${i} out of range`);
    const current = page.getRotation().angle;
    page.setRotation(degrees((((current + angle) % 360) + 360) % 360));
    notifyProgress(onProgress, { phase: 'working', label: `Rotated page ${done + 1} of ${targets.length}.`, current: done + 1, total: targets.length });
  }
  return doc.save();
}

const GRID: Record<number, [number, number]> = {
  2: [1, 2],
  4: [2, 2],
  6: [2, 3],
  8: [2, 4],
  9: [3, 3],
  16: [4, 4],
};

/** Place N source pages onto each output A4 page (handout / booklet-style). */
export async function nUpPdf(bytes: Uint8Array, perSheet: number, onProgress?: ProgressCallback): Promise<Uint8Array> {
  const grid = GRID[perSheet];
  if (!grid) throw new Error(`Unsupported pages-per-sheet: ${perSheet}`);
  const [cols, rows] = grid;

  const out = await PDFDocument.create();
  const src = await loadPdf(bytes);
  const indices = src.getPageIndices();
  const embedded = await out.embedPdf(bytes, indices);

  const [sheetW, sheetH] = PAGE_SIZES.a4;
  const gap = 8;
  const cellW = (sheetW - gap * (cols + 1)) / cols;
  const cellH = (sheetH - gap * (rows + 1)) / rows;

  for (let i = 0; i < embedded.length; i += perSheet) {
    const page = out.addPage(PAGE_SIZES.a4);
    for (let j = 0; j < perSheet && i + j < embedded.length; j++) {
      notifyProgress(onProgress, { phase: 'working', label: `Placing page ${i + j + 1} of ${embedded.length}…`, current: i + j, total: embedded.length });
      const ep = embedded[i + j];
      const col = j % cols;
      const row = Math.floor(j / cols);
      const scale = Math.min(cellW / ep.width, cellH / ep.height);
      const w = ep.width * scale;
      const h = ep.height * scale;
      const x = gap + col * (cellW + gap) + (cellW - w) / 2;
      // rows fill top-to-bottom; PDF origin is bottom-left
      const y = sheetH - gap - (row + 1) * cellH - row * gap + (cellH - h) / 2;
      page.drawPage(ep, { x, y, width: w, height: h });
      notifyProgress(onProgress, { phase: 'working', label: `Placed page ${i + j + 1} of ${embedded.length}.`, current: i + j + 1, total: embedded.length });
    }
  }
  return out.save();
}

export interface BookletOptions {
  pageSize?: keyof typeof PAGE_SIZES;
  binding?: 'left' | 'right';
  margin?: number;
}

function drawFittedPage(sheet: PDFPage, ep: PDFEmbeddedPage, box: { x: number; y: number; width: number; height: number }) {
  const scale = Math.min(box.width / ep.width, box.height / ep.height);
  const w = ep.width * scale;
  const h = ep.height * scale;
  sheet.drawPage(ep, {
    x: box.x + (box.width - w) / 2,
    y: box.y + (box.height - h) / 2,
    width: w,
    height: h,
  });
}

function leftBoundSpreadOrder(paddedPageCount: number): [number, number][] {
  const spreads: [number, number][] = [];
  const sheetCount = paddedPageCount / 4;
  for (let sheet = 0; sheet < sheetCount; sheet++) {
    spreads.push([paddedPageCount - 1 - sheet * 2, sheet * 2]);
    spreads.push([sheet * 2 + 1, paddedPageCount - 2 - sheet * 2]);
  }
  return spreads;
}

/** Reorder pages into two-up booklet spreads on landscape sheets. */
export async function bookletPdf(bytes: Uint8Array, options: BookletOptions = {}, onProgress?: ProgressCallback): Promise<Uint8Array> {
  const src = await loadPdf(bytes);
  const sourceIndices = src.getPageIndices();
  if (sourceIndices.length === 0) throw new Error('No pages found');

  const out = await PDFDocument.create();
  const embedded = await out.embedPdf(bytes, sourceIndices);
  const pageSize = options.pageSize && options.pageSize in PAGE_SIZES ? options.pageSize : 'a4';
  const [portraitW, portraitH] = PAGE_SIZES[pageSize];
  const sheetW = portraitH;
  const sheetH = portraitW;
  const margin = Math.max(0, Number(options.margin ?? 18));
  const printableW = Math.max(1, sheetW - margin * 2);
  const printableH = Math.max(1, sheetH - margin * 2);
  const cellW = printableW / 2;
  const cellH = printableH;
  const paddedPageCount = Math.ceil(embedded.length / 4) * 4;
  const spreads = leftBoundSpreadOrder(paddedPageCount);
  const binding = options.binding === 'right' ? 'right' : 'left';

  for (const [spreadIndex, spread] of spreads.entries()) {
    notifyProgress(onProgress, { phase: 'working', label: `Creating booklet spread ${spreadIndex + 1} of ${spreads.length}…`, current: spreadIndex, total: spreads.length });
    const page = out.addPage([sheetW, sheetH]);
    const ordered = binding === 'right' ? ([spread[1], spread[0]] as [number, number]) : spread;
    const boxes = [
      { x: margin, y: margin, width: cellW, height: cellH },
      { x: margin + cellW, y: margin, width: cellW, height: cellH },
    ];
    for (const [slot, pageIndex] of ordered.entries()) {
      const ep = embedded[pageIndex];
      if (ep) drawFittedPage(page, ep, boxes[slot]);
    }
    notifyProgress(onProgress, { phase: 'working', label: `Created booklet spread ${spreadIndex + 1} of ${spreads.length}.`, current: spreadIndex + 1, total: spreads.length });
  }

  return out.save();
}
