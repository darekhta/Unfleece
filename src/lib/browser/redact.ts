import { loadPdfDocument } from './pdfjs.js';
import { abortError, reportProgress, throwIfAborted, type RunOptions } from '../progress.js';
import { wasmImagePagesToPdf, type WasmImagePage } from '../wasm/core.js';

export interface RedactionRegion {
  pageIndex: number;
  /** Fractions of the rendered page, top-left origin. */
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface RedactOptions {
  redactions: RedactionRegion[];
  scale?: number;
  color?: 'black' | 'white';
}

export interface PixelRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);
const EPS = 1e-6;

export function normalizedRedaction(rect: RedactionRegion): RedactionRegion | null {
  const x1 = clamp(Math.min(rect.x, rect.x + rect.w), 0, 1);
  const y1 = clamp(Math.min(rect.y, rect.y + rect.h), 0, 1);
  const x2 = clamp(Math.max(rect.x, rect.x + rect.w), 0, 1);
  const y2 = clamp(Math.max(rect.y, rect.y + rect.h), 0, 1);
  if (x2 - x1 < 0.002 || y2 - y1 < 0.002) return null;
  return { pageIndex: Math.max(0, Math.floor(rect.pageIndex)), x: x1, y: y1, w: x2 - x1, h: y2 - y1 };
}

export function redactionToPixels(rect: RedactionRegion, width: number, height: number, expandPx = 1): PixelRect {
  const n = normalizedRedaction(rect);
  if (!n) return { x: 0, y: 0, w: 0, h: 0 };
  const x = Math.max(0, Math.floor(n.x * width + EPS) - expandPx);
  const y = Math.max(0, Math.floor(n.y * height + EPS) - expandPx);
  const right = Math.min(width, Math.ceil((n.x + n.w) * width - EPS) + expandPx);
  const bottom = Math.min(height, Math.ceil((n.y + n.h) * height - EPS) + expandPx);
  return { x, y, w: Math.max(0, right - x), h: Math.max(0, bottom - y) };
}

function pageRedactions(redactions: RedactionRegion[], pageIndex: number): RedactionRegion[] {
  return redactions
    .map(normalizedRedaction)
    .filter((rect): rect is RedactionRegion => rect !== null)
    .filter((rect) => rect.pageIndex === pageIndex);
}

function canvasBlob(canvas: HTMLCanvasElement, type = 'image/png', quality?: number): Promise<Blob> {
  return new Promise((resolve, reject) =>
    canvas.toBlob((blob) => (blob ? resolve(blob) : reject(new Error('Canvas export failed'))), type, quality),
  );
}

/** Burn redaction boxes into rendered pages, then rebuild an image-only PDF. */
export async function redactPdf(bytes: Uint8Array, opts: RedactOptions, run: RunOptions = {}): Promise<Uint8Array> {
  const redactions = opts.redactions.map(normalizedRedaction).filter((rect): rect is RedactionRegion => Boolean(rect));
  if (redactions.length === 0) throw new Error('Add at least one redaction box before running.');

  const scale = clamp(Number(opts.scale) || 2, 1, 3);
  const fill = opts.color === 'white' ? '#fff' : '#000';

  reportProgress(run, { phase: 'loading', label: 'Opening PDF…' });
  const source = await loadPdfDocument(bytes);
  const imagePages: WasmImagePage[] = [];

  try {
    for (let i = 1; i <= source.numPages; i++) {
      throwIfAborted(run.signal);
      reportProgress(run, { phase: 'rendering', label: `Rendering page ${i} of ${source.numPages}…`, current: i - 1, total: source.numPages });

      const page = await source.getPage(i);
      const base = page.getViewport({ scale: 1 });
      const viewport = page.getViewport({ scale });
      const canvas = document.createElement('canvas');
      canvas.width = Math.ceil(viewport.width);
      canvas.height = Math.ceil(viewport.height);
      const ctx = canvas.getContext('2d');
      if (!ctx) throw new Error('Canvas not supported');

      ctx.fillStyle = '#fff';
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      const task = page.render({ canvas, canvasContext: ctx, viewport });
      const onAbort = () => task.cancel();
      run.signal?.addEventListener('abort', onAbort, { once: true });
      try {
        await task.promise;
      } catch (error) {
        if (run.signal?.aborted) throw abortError();
        throw error;
      } finally {
        run.signal?.removeEventListener('abort', onAbort);
      }

      ctx.fillStyle = fill;
      for (const rect of pageRedactions(redactions, i - 1)) {
        const r = redactionToPixels(rect, canvas.width, canvas.height);
        ctx.fillRect(r.x, r.y, r.w, r.h);
      }

      const png = new Uint8Array(await (await canvasBlob(canvas)).arrayBuffer());
      imagePages.push({ widthPt: base.width, heightPt: base.height, bytes: png, type: 'png' });
      canvas.width = 0;
      canvas.height = 0;
      page.cleanup();
      reportProgress(run, { phase: 'rendering', label: `Redacted page ${i} of ${source.numPages}.`, current: i, total: source.numPages });
    }
  } finally {
    await source.cleanup();
  }

  reportProgress(run, { phase: 'saving', label: 'Saving redacted PDF…' });
  throwIfAborted(run.signal);
  return wasmImagePagesToPdf(imagePages);
}
