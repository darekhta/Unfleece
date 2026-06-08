import { pdfjsLib } from './pdfjs.js';
import { reportProgress, throwIfAborted, type RunOptions } from '../progress.js';

export interface CompareOptions {
  scale?: number;
  threshold?: number;
}

function canvasData(width: number, height: number): { canvas: HTMLCanvasElement; ctx: CanvasRenderingContext2D } {
  const canvas = document.createElement('canvas');
  canvas.width = Math.max(1, Math.ceil(width));
  canvas.height = Math.max(1, Math.ceil(height));
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  if (!ctx) throw new Error('Canvas not supported');
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  return { canvas, ctx };
}

async function renderPage(doc: Awaited<ReturnType<typeof pdfjsLib.getDocument>['promise']>, pageNumber: number, scale: number) {
  const page = await doc.getPage(pageNumber);
  const viewport = page.getViewport({ scale });
  const { canvas, ctx } = canvasData(viewport.width, viewport.height);
  await page.render({ canvas, canvasContext: ctx, viewport }).promise;
  const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
  page.cleanup();
  return { width: canvas.width, height: canvas.height, data };
}

function diffImages(
  a: { width: number; height: number; data: ImageData },
  b: { width: number; height: number; data: ImageData },
  threshold: number,
): { changed: number; total: number; percent: number } {
  const width = Math.min(a.width, b.width);
  const height = Math.min(a.height, b.height);
  const total = width * height;
  if (total === 0 || a.width !== b.width || a.height !== b.height) {
    return { changed: Math.max(total, 1), total: Math.max(total, 1), percent: 100 };
  }

  let changed = 0;
  const ad = a.data.data;
  const bd = b.data.data;
  for (let i = 0; i < ad.length; i += 4) {
    const delta =
      Math.abs(ad[i] - bd[i])
      + Math.abs(ad[i + 1] - bd[i + 1])
      + Math.abs(ad[i + 2] - bd[i + 2])
      + Math.abs(ad[i + 3] - bd[i + 3]);
    if (delta > threshold) changed += 1;
  }
  return { changed, total, percent: (changed / total) * 100 };
}

export async function comparePdfs(
  first: Uint8Array,
  second: Uint8Array,
  opts: CompareOptions = {},
  run: RunOptions = {},
): Promise<string> {
  const { scale = 0.75, threshold = 24 } = opts;
  reportProgress(run, { phase: 'loading', label: 'Opening PDFs…' });
  const a = await pdfjsLib.getDocument({ data: first }).promise;
  const b = await pdfjsLib.getDocument({ data: second }).promise;
  const lines: string[] = ['PDF visual comparison', ''];
  let compared = 0;
  let pagesWithDifferences = 0;

  try {
    lines.push(`First PDF pages: ${a.numPages}`);
    lines.push(`Second PDF pages: ${b.numPages}`);
    if (a.numPages !== b.numPages) lines.push(`Page-count difference: ${Math.abs(a.numPages - b.numPages)} page(s).`);
    lines.push('');

    const shared = Math.min(a.numPages, b.numPages);
    for (let i = 1; i <= shared; i++) {
      throwIfAborted(run.signal);
      reportProgress(run, { phase: 'rendering', label: `Comparing page ${i} of ${shared}…`, current: i - 1, total: shared });
      const [ra, rb] = await Promise.all([renderPage(a, i, scale), renderPage(b, i, scale)]);
      const diff = diffImages(ra, rb, threshold);
      compared += 1;
      if (diff.changed > 0) pagesWithDifferences += 1;
      lines.push(
        diff.changed === 0
          ? `Page ${i}: no visual pixel differences at ${scale}x render scale.`
          : `Page ${i}: ${diff.changed.toLocaleString()} of ${diff.total.toLocaleString()} sampled pixels differ (${diff.percent.toFixed(2)}%).`,
      );
      reportProgress(run, { phase: 'rendering', label: `Compared page ${i} of ${shared}.`, current: i, total: shared });
    }
  } finally {
    await Promise.allSettled([a.cleanup(), b.cleanup()]);
  }

  lines.push('');
  if (compared === 0) {
    lines.push('No shared pages to compare.');
  } else if (pagesWithDifferences === 0 && a.numPages === b.numPages) {
    lines.push('Summary: no visual differences found at the sampled render scale.');
  } else {
    lines.push(`Summary: ${pagesWithDifferences} of ${compared} compared page(s) had visual differences.`);
  }
  lines.push('This is a browser-rendered visual comparison, not a cryptographic file hash.');
  return lines.join('\n');
}
