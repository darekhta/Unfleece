// pdf.js based rendering. Runs on the main thread; pdf.js offloads parsing to
// its own worker so the UI stays responsive. Browser-only (uses <canvas>),
// so this is covered by Playwright E2E rather than Node unit tests.
import { pdfjsLib } from './pdfjs.js';
import { abortError, reportProgress, throwIfAborted, type RunOptions } from '../progress.js';
import type { ExtractedTextItem, ExtractedTextPage } from '../tools/office.js';

export interface RenderedPage {
  name: string;
  blob: Blob;
}

export interface RenderOptions {
  scale?: number;
  type?: 'image/png' | 'image/jpeg';
  quality?: number;
}

/** Render every page to an image blob. */
export async function renderToImages(bytes: Uint8Array, opts: RenderOptions = {}, run: RunOptions = {}): Promise<RenderedPage[]> {
  const { scale = 2, type = 'image/png', quality = 0.85 } = opts;
  const ext = type === 'image/png' ? 'png' : 'jpg';
  reportProgress(run, { phase: 'loading', label: 'Opening PDF…' });
  const doc = await pdfjsLib.getDocument({ data: bytes }).promise;
  const out: RenderedPage[] = [];

  try {
    for (let i = 1; i <= doc.numPages; i++) {
      reportProgress(run, { phase: 'rendering', label: `Rendering page ${i} of ${doc.numPages}…`, current: i - 1, total: doc.numPages });
      const page = await doc.getPage(i);
      const viewport = page.getViewport({ scale });
      const canvas = document.createElement('canvas');
      canvas.width = Math.ceil(viewport.width);
      canvas.height = Math.ceil(viewport.height);
      const ctx = canvas.getContext('2d');
      if (!ctx) throw new Error('Canvas not supported');
      const task = page.render({ canvas, canvasContext: ctx, viewport });
      const onAbort = () => task.cancel();
      run.signal?.addEventListener('abort', onAbort, { once: true });
      try {
        await task.promise;
      } catch (e) {
        if (run.signal?.aborted) throw abortError();
        throw e;
      } finally {
        run.signal?.removeEventListener('abort', onAbort);
      }
      throwIfAborted(run.signal);
      const blob = await new Promise<Blob>((resolve, reject) =>
        canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('toBlob failed'))), type, quality),
      );
      out.push({ name: `page-${String(i).padStart(3, '0')}.${ext}`, blob });
      canvas.width = 0;
      canvas.height = 0;
      page.cleanup();
      reportProgress(run, { phase: 'rendering', label: `Rendered page ${i} of ${doc.numPages}.`, current: i, total: doc.numPages });
    }
  } finally {
    await doc.cleanup();
  }
  return out;
}

function textItemShape(item: unknown): item is { str: string; transform: number[]; width?: number; height?: number } {
  return Boolean(
    item
      && typeof item === 'object'
      && 'str' in item
      && typeof (item as { str?: unknown }).str === 'string'
      && 'transform' in item
      && Array.isArray((item as { transform?: unknown }).transform),
  );
}

function lineText(items: ExtractedTextItem[]): string {
  if (items.length === 0) return '';
  const sorted = [...items].sort((a, b) => (Math.abs(b.y - a.y) > 3 ? b.y - a.y : a.x - b.x));
  const lines: ExtractedTextItem[][] = [];
  for (const item of sorted) {
    const last = lines.at(-1);
    if (!last || Math.abs(last[0].y - item.y) > Math.max(3, item.height * 0.6)) lines.push([item]);
    else last.push(item);
  }
  return lines
    .map((line) => line.sort((a, b) => a.x - b.x).map((item) => item.text).join(' ').replace(/\s+/g, ' ').trim())
    .filter(Boolean)
    .join('\n');
}

/** Extract selectable text and coarse text positions from every page. */
export async function extractTextPages(bytes: Uint8Array, run: RunOptions = {}): Promise<ExtractedTextPage[]> {
  reportProgress(run, { phase: 'loading', label: 'Opening PDF…' });
  const doc = await pdfjsLib.getDocument({ data: bytes }).promise;
  const pages: ExtractedTextPage[] = [];
  try {
    for (let i = 1; i <= doc.numPages; i++) {
      reportProgress(run, { phase: 'extracting', label: `Extracting text from page ${i} of ${doc.numPages}…`, current: i - 1, total: doc.numPages });
      const page = await doc.getPage(i);
      const content = await page.getTextContent();
      const items = (content.items as unknown[]).filter(textItemShape).map((item): ExtractedTextItem => {
        const [, , , , x = 0, y = 0] = item.transform;
        return {
          text: item.str,
          x,
          y,
          width: item.width ?? 0,
          height: item.height ?? 0,
        };
      });
      pages.push({ pageNumber: i, text: lineText(items), items });
      page.cleanup();
      reportProgress(run, { phase: 'extracting', label: `Extracted page ${i} of ${doc.numPages}.`, current: i, total: doc.numPages });
    }
  } finally {
    await doc.cleanup();
  }
  return pages;
}

/** Extract selectable text from every page. */
export async function extractText(bytes: Uint8Array, run: RunOptions = {}): Promise<string> {
  const pages = await extractTextPages(bytes, run);
  return pages.map((page) => page.text).join('\n\n').trim();
}

/** Page sizes in PDF points (for the compress tool to preserve dimensions). */
export async function pagePointSizes(bytes: Uint8Array): Promise<{ width: number; height: number }[]> {
  const doc = await pdfjsLib.getDocument({ data: bytes }).promise;
  const sizes: { width: number; height: number }[] = [];
  try {
    for (let i = 1; i <= doc.numPages; i++) {
      const page = await doc.getPage(i);
      const vp = page.getViewport({ scale: 1 });
      sizes.push({ width: vp.width, height: vp.height });
    }
  } finally {
    await doc.cleanup();
  }
  return sizes;
}
