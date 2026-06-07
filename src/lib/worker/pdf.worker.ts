/// <reference lib="webworker" />
import * as Comlink from 'comlink';
import { loadPdf } from '../util/pdf.js';
import { parsePageRanges } from '../util/ranges.js';
import { mergePdfs } from '../tools/merge.js';
import {
  bookletPdf, extractPages, removePages, reorderPages, splitToPages, splitByRanges, rotatePdf, nUpPdf, type BookletOptions,
} from '../tools/organize.js';
import { addPageNumbers, addTextWatermark, type PageNumberOptions, type WatermarkOptions } from '../tools/annotate.js';
import { cropPdf, type CropMargins } from '../tools/crop.js';
import { setMetadata, getMetadata, stripMetadata, type MetadataUpdate } from '../tools/metadata.js';
import { protectPdf, sanitizePdf, unlockPdf, type ProtectOptions, type SanitizeOptions } from '../tools/security.js';
import { imagesToPdf, type InputImage, type ImagesToPdfOptions } from '../tools/imagesToPdf.js';
import { listFormFields, fillForm, flattenForm, type FormValues } from '../tools/forms.js';
import type { ProgressCallback } from '../progress.js';
import { wasmPageCount } from '../wasm/core.js';

async function countPages(bytes: Uint8Array): Promise<number> {
  try {
    return await wasmPageCount(bytes);
  } catch {
    return (await loadPdf(bytes)).getPageCount();
  }
}

/** The off-main-thread API. Heavy pdf-lib work runs here so the UI stays smooth. */
const api = {
  getPageCount: (file: Uint8Array) => countPages(file),

  merge: (files: Uint8Array[], onProgress?: ProgressCallback) => mergePdfs(files, onProgress),

  splitEach: (file: Uint8Array, onProgress?: ProgressCallback) => splitToPages(file, onProgress),

  async splitRanges(file: Uint8Array, rangesStr: string, onProgress?: ProgressCallback) {
    const n = await countPages(file);
    const tokens = rangesStr.split(',').map((s) => s.trim()).filter(Boolean);
    if (tokens.length === 0) throw new Error('Enter at least one page range');
    const ranges = tokens.map((t) => parsePageRanges(t, n));
    return splitByRanges(file, ranges, onProgress);
  },

  async extractPages(file: Uint8Array, pagesStr: string, onProgress?: ProgressCallback) {
    return extractPages(file, parsePageRanges(pagesStr, await countPages(file)), onProgress);
  },

  async removePages(file: Uint8Array, pagesStr: string, onProgress?: ProgressCallback) {
    return removePages(file, parsePageRanges(pagesStr, await countPages(file)), onProgress);
  },

  async reorder(file: Uint8Array, orderStr: string, onProgress?: ProgressCallback) {
    const order = orderStr.split(',').map((s) => parseInt(s.trim(), 10) - 1);
    if (order.some((n) => Number.isNaN(n))) throw new Error('Order must be comma-separated page numbers');
    return reorderPages(file, order, onProgress);
  },

  async rotate(file: Uint8Array, angle: number, pagesStr?: string, onProgress?: ProgressCallback) {
    const indices = pagesStr && pagesStr.trim()
      ? parsePageRanges(pagesStr, await countPages(file))
      : undefined;
    return rotatePdf(file, Number(angle), indices, onProgress);
  },

  nUp: (file: Uint8Array, perSheet: number, onProgress?: ProgressCallback) => nUpPdf(file, Number(perSheet), onProgress),

  booklet: (file: Uint8Array, opts: BookletOptions, onProgress?: ProgressCallback) => bookletPdf(file, opts, onProgress),

  imagesToPdf: (images: InputImage[], opts: ImagesToPdfOptions, onProgress?: ProgressCallback) => imagesToPdf(images, opts, onProgress),

  pageNumbers: (file: Uint8Array, opts: PageNumberOptions, onProgress?: ProgressCallback) => addPageNumbers(file, opts, onProgress),

  watermark: (file: Uint8Array, opts: WatermarkOptions, onProgress?: ProgressCallback) => addTextWatermark(file, opts, onProgress),

  crop: (file: Uint8Array, margins: CropMargins, onProgress?: ProgressCallback) => cropPdf(file, margins, onProgress),

  getMetadata: (file: Uint8Array) => getMetadata(file),

  setMetadata: (file: Uint8Array, meta: MetadataUpdate & { keywords?: string }) => {
    const update: MetadataUpdate = { ...meta };
    if (typeof meta.keywords === 'string') {
      update.keywords = meta.keywords.split(',').map((s) => s.trim()).filter(Boolean);
    }
    return setMetadata(file, update);
  },

  stripMetadata: (file: Uint8Array) => stripMetadata(file),

  sanitize: (file: Uint8Array, opts: SanitizeOptions, onProgress?: ProgressCallback) => sanitizePdf(file, opts, onProgress),

  protect: (file: Uint8Array, opts: ProtectOptions, onProgress?: ProgressCallback) => protectPdf(file, opts, onProgress),

  unlock: (file: Uint8Array, password: string, onProgress?: ProgressCallback) => unlockPdf(file, password, onProgress),

  listFormFields: (file: Uint8Array) => listFormFields(file),
  fillForm: (file: Uint8Array, values: FormValues) => fillForm(file, values),
  flatten: (file: Uint8Array) => flattenForm(file),
};

export type PdfWorkerApi = typeof api;

Comlink.expose(api);
