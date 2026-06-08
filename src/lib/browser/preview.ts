// Shared PDF preview rendering for the interactive editors (sign/crop/page-numbers/
// watermark). Renders a single page to a <canvas> at a responsive CSS width with
// devicePixelRatio for crisp output, and exposes the scale needed to map between
// on-screen CSS pixels and PDF user-space points.
import { pdfjsLib, loadPdfDocument } from './pdfjs.js';

export type PdfDoc = Awaited<ReturnType<typeof pdfjsLib.getDocument>['promise']>;

export async function loadPdfDoc(bytes: Uint8Array): Promise<PdfDoc> {
  // Clone the bytes: pdf.js transfers/detaches the buffer, but callers reuse it.
  return loadPdfDocument(bytes);
}

export interface PageRender {
  canvas: HTMLCanvasElement;
  /** Display (CSS) size in px. */
  cssWidth: number;
  cssHeight: number;
  /** Page size in PDF points (1/72"). */
  widthPt: number;
  heightPt: number;
  /** Display px per PDF point. */
  scale: number;
}

/** Render `pageNum` (1-based) to a canvas sized to `cssWidth` display px. */
export async function renderPage(doc: PdfDoc, pageNum: number, cssWidth: number): Promise<PageRender> {
  const page = await doc.getPage(pageNum);
  const base = page.getViewport({ scale: 1 }); // width/height in points
  const scale = cssWidth / base.width;
  const viewport = page.getViewport({ scale });
  const dpr = Math.min(window.devicePixelRatio || 1, 2);

  const canvas = document.createElement('canvas');
  canvas.width = Math.round(viewport.width * dpr);
  canvas.height = Math.round(viewport.height * dpr);
  canvas.style.width = `${Math.round(viewport.width)}px`;
  canvas.style.height = `${Math.round(viewport.height)}px`;
  canvas.style.display = 'block';

  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('Canvas not supported');
  ctx.scale(dpr, dpr);
  await page.render({ canvas, canvasContext: ctx, viewport }).promise;
  page.cleanup();

  return {
    canvas,
    cssWidth: viewport.width,
    cssHeight: viewport.height,
    widthPt: base.width,
    heightPt: base.height,
    scale,
  };
}

/**
 * Map an on-screen overlay point (CSS px from the canvas top-left) to a PDF point
 * (origin bottom-left). Assumes page rotation 0 (the overwhelming common case).
 */
export function screenToPdf(xCss: number, yCss: number, r: PageRender): { x: number; y: number } {
  return { x: xCss / r.scale, y: r.heightPt - yCss / r.scale };
}

/** CSS px length -> PDF points. */
export const pxToPt = (px: number, r: PageRender): number => px / r.scale;
/** PDF points -> CSS px length. */
export const ptToPx = (pt: number, r: PageRender): number => pt * r.scale;
