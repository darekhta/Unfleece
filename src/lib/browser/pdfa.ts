import { loadPdfDocument } from './pdfjs.js';
import { abortError, reportProgress, throwIfAborted, type RunOptions } from '../progress.js';

interface RenderedPngPage {
  width: number;
  height: number;
  bytes: Uint8Array;
}

export interface PdfAOptions {
  scale?: number;
}

const MAGIC = [85, 70, 80, 65]; // UFPA

async function canvasToPngBytes(canvas: HTMLCanvasElement): Promise<Uint8Array> {
  const blob = await new Promise<Blob>((resolve, reject) =>
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('toBlob failed'))), 'image/png'),
  );
  return new Uint8Array(await blob.arrayBuffer());
}

function packPages(pages: RenderedPngPage[], date: Date): Uint8Array {
  const byteLength = 4 + 4 + 7 + pages.reduce((sum, page) => sum + 4 + 4 + 4 + page.bytes.length, 0);
  const pack = new Uint8Array(byteLength);
  const view = new DataView(pack.buffer);
  let pos = 0;
  pack.set(MAGIC, pos);
  pos += 4;
  view.setUint32(pos, pages.length, true);
  pos += 4;
  view.setUint16(pos, date.getUTCFullYear(), true);
  pos += 2;
  pack[pos++] = date.getUTCMonth() + 1;
  pack[pos++] = date.getUTCDate();
  pack[pos++] = date.getUTCHours();
  pack[pos++] = date.getUTCMinutes();
  pack[pos++] = date.getUTCSeconds();

  for (const page of pages) {
    view.setFloat32(pos, page.width, true);
    pos += 4;
    view.setFloat32(pos, page.height, true);
    pos += 4;
    view.setUint32(pos, page.bytes.length, true);
    pos += 4;
    pack.set(page.bytes, pos);
    pos += page.bytes.length;
  }
  return pack;
}

export async function exportPdfA(bytes: Uint8Array, opts: PdfAOptions = {}, run: RunOptions = {}): Promise<Uint8Array> {
  const { scale = 1.5 } = opts;
  reportProgress(run, { phase: 'loading', label: 'Opening PDF…' });
  const src = await loadPdfDocument(bytes);
  const pages: RenderedPngPage[] = [];

  try {
    for (let i = 1; i <= src.numPages; i++) {
      reportProgress(run, { phase: 'rendering', label: `Rendering page ${i} of ${src.numPages} for PDF/A…`, current: i - 1, total: src.numPages });
      const page = await src.getPage(i);
      const pts = page.getViewport({ scale: 1 });
      const viewport = page.getViewport({ scale });
      const canvas = document.createElement('canvas');
      canvas.width = Math.ceil(viewport.width);
      canvas.height = Math.ceil(viewport.height);
      const ctx = canvas.getContext('2d');
      if (!ctx) throw new Error('Canvas not supported');
      ctx.fillStyle = '#ffffff';
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
      throwIfAborted(run.signal);

      pages.push({ width: pts.width, height: pts.height, bytes: await canvasToPngBytes(canvas) });
      canvas.width = 0;
      canvas.height = 0;
      page.cleanup();
      reportProgress(run, { phase: 'rendering', label: `Rendered page ${i} of ${src.numPages}.`, current: i, total: src.numPages });
    }
  } finally {
    await src.cleanup();
  }

  throwIfAborted(run.signal);
  reportProgress(run, { phase: 'saving', label: 'Writing PDF/A-2b with Rust…' });
  const { wasmPdfAFromPngPages } = await import('../wasm/core.js');
  return wasmPdfAFromPngPages(packPages(pages, new Date()));
}
