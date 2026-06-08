import { loadPdfDocument } from './pdfjs.js';
import { reportProgress, throwIfAborted, type RunOptions } from '../progress.js';
import { wasmSetCropBoxes, type WasmCropBox } from '../wasm/core.js';

export interface AutoCropOptions {
  scale?: number;
  tolerance?: number;
  padding?: number;
}

export interface PixelBounds {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

export function findContentBounds(data: Uint8ClampedArray, width: number, height: number, tolerance = 12): PixelBounds | null {
  const cutoff = Math.max(0, Math.min(255, 255 - tolerance));
  let minX = width;
  let minY = height;
  let maxX = -1;
  let maxY = -1;

  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 4;
      const alpha = data[i + 3];
      if (alpha === 0) continue;
      if (data[i] < cutoff || data[i + 1] < cutoff || data[i + 2] < cutoff) {
        minX = Math.min(minX, x);
        minY = Math.min(minY, y);
        maxX = Math.max(maxX, x);
        maxY = Math.max(maxY, y);
      }
    }
  }

  return maxX >= 0 ? { minX, minY, maxX, maxY } : null;
}

/** Detect white margins from rendered pixels and set a per-page CropBox. */
export async function autoCropPdf(bytes: Uint8Array, opts: AutoCropOptions = {}, run: RunOptions = {}): Promise<Uint8Array> {
  const scale = Math.max(0.5, Math.min(3, Number(opts.scale ?? 1.5)));
  const tolerance = Math.max(0, Math.min(80, Number(opts.tolerance ?? 12)));
  const padding = Math.max(0, Math.min(144, Number(opts.padding ?? 6)));

  reportProgress(run, { phase: 'loading', label: 'Opening PDF…' });
  const renderDoc = await loadPdfDocument(bytes);
  const cropBoxes: WasmCropBox[] = [];

  try {
    for (let i = 1; i <= renderDoc.numPages; i++) {
      reportProgress(run, { phase: 'rendering', label: `Scanning page ${i} of ${renderDoc.numPages}…`, current: i - 1, total: renderDoc.numPages });
      const renderPage = await renderDoc.getPage(i);
      const viewport = renderPage.getViewport({ scale, rotation: 0 });
      const canvas = document.createElement('canvas');
      canvas.width = Math.ceil(viewport.width);
      canvas.height = Math.ceil(viewport.height);
      const ctx = canvas.getContext('2d', { willReadFrequently: true });
      if (!ctx) throw new Error('Canvas not supported');
      ctx.fillStyle = '#fff';
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      const task = renderPage.render({ canvas, canvasContext: ctx, viewport });
      await task.promise;
      throwIfAborted(run.signal);

      const image = ctx.getImageData(0, 0, canvas.width, canvas.height);
      const bounds = findContentBounds(image.data, canvas.width, canvas.height, tolerance);
      // The pdf.js viewBox is the page's crop box in PDF points: [x0, y0, x1, y1].
      const [vx0, vy0, vx1, vy1] = renderPage.view;
      const crop = { x: vx0, y: vy0, width: vx1 - vx0, height: vy1 - vy0 };
      if (bounds) {
        const sx = canvas.width / crop.width;
        const sy = canvas.height / crop.height;
        const contentLeft = bounds.minX / sx;
        const contentRight = (bounds.maxX + 1) / sx;
        const contentTop = crop.height - bounds.minY / sy;
        const contentBottom = crop.height - (bounds.maxY + 1) / sy;
        const newX = crop.x + Math.max(0, contentLeft - padding);
        const newY = crop.y + Math.max(0, contentBottom - padding);
        const newRight = crop.x + Math.min(crop.width, contentRight + padding);
        const newTop = crop.y + Math.min(crop.height, contentTop + padding);
        const newWidth = newRight - newX;
        const newHeight = newTop - newY;
        const reducesPage = newWidth < crop.width - 0.5 || newHeight < crop.height - 0.5;
        if (newWidth > 0 && newHeight > 0 && reducesPage) {
          cropBoxes.push({ pageIndex: i - 1, x: newX, y: newY, width: newWidth, height: newHeight });
        }
      }

      canvas.width = 0;
      canvas.height = 0;
      renderPage.cleanup();
      reportProgress(run, { phase: 'rendering', label: `Scanned page ${i} of ${renderDoc.numPages}.`, current: i, total: renderDoc.numPages });
    }
  } finally {
    await renderDoc.cleanup();
  }

  const cropped = cropBoxes.length;
  reportProgress(run, { phase: 'saving', label: cropped > 0 ? `Cropped ${cropped} page${cropped === 1 ? '' : 's'}. Saving…` : 'No white margins found. Saving clean copy…' });
  return wasmSetCropBoxes(bytes, cropBoxes);
}
