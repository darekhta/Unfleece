import { StandardFonts, rgb, degrees } from '@cantoo/pdf-lib';
import { notifyProgress, type ProgressCallback } from '../progress.js';
import { loadPdf } from '../util/pdf.js';

export type NumberPosition =
  | 'bottom-center'
  | 'bottom-right'
  | 'bottom-left'
  | 'top-center'
  | 'top-right'
  | 'top-left';

export interface PageNumberOptions {
  /** Template; {n} = page number, {total} = page count. Default '{n}'. */
  format?: string;
  position?: NumberPosition;
  fontSize?: number;
  margin?: number;
  startAt?: number;
  padTo?: number;
}

/** Stamp page numbers onto every page. */
export async function addPageNumbers(
  bytes: Uint8Array,
  opts: PageNumberOptions = {},
  onProgress?: ProgressCallback,
): Promise<Uint8Array> {
  const { format = '{n}', position = 'bottom-center', fontSize = 10, margin = 24, startAt = 1, padTo = 0 } = opts;
  notifyProgress(onProgress, { phase: 'loading', label: 'Loading PDF…' });
  const doc = await loadPdf(bytes);
  const font = await doc.embedFont(StandardFonts.Helvetica);
  const pages = doc.getPages();

  pages.forEach((page, i) => {
    notifyProgress(onProgress, { phase: 'working', label: `Adding page number ${i + 1} of ${pages.length}…`, current: i, total: pages.length });
    const n = String(i + startAt).padStart(Math.max(0, Math.floor(padTo)), '0');
    const text = format
      .replace(/\{n\}/g, n)
      .replace(/\{total\}/g, String(pages.length));
    const { width, height } = page.getSize();
    const textWidth = font.widthOfTextAtSize(text, fontSize);

    let x: number;
    if (position.endsWith('center')) x = (width - textWidth) / 2;
    else if (position.endsWith('right')) x = width - margin - textWidth;
    else x = margin;

    const y = position.startsWith('top') ? height - margin - fontSize : margin;

    page.drawText(text, { x, y, size: fontSize, font, color: rgb(0.1, 0.1, 0.1) });
  });

  notifyProgress(onProgress, { phase: 'saving', label: 'Saving numbered PDF…', current: pages.length, total: pages.length });
  return doc.save();
}

export interface WatermarkOptions {
  text: string;
  fontSize?: number;
  opacity?: number;
  angle?: number;
  color?: { r: number; g: number; b: number };
}

/** Draw a diagonal text watermark centered on every page. */
export async function addTextWatermark(
  bytes: Uint8Array,
  opts: WatermarkOptions,
  onProgress?: ProgressCallback,
): Promise<Uint8Array> {
  if (!opts.text) throw new Error('Watermark text is required');
  const { text, fontSize = 50, opacity = 0.25, angle = 45, color = { r: 0.6, g: 0.6, b: 0.6 } } = opts;

  notifyProgress(onProgress, { phase: 'loading', label: 'Loading PDF…' });
  const doc = await loadPdf(bytes);
  const font = await doc.embedFont(StandardFonts.HelveticaBold);
  const rad = (angle * Math.PI) / 180;
  const pages = doc.getPages();

  for (const [i, page] of pages.entries()) {
    notifyProgress(onProgress, { phase: 'working', label: `Watermarking page ${i + 1} of ${pages.length}…`, current: i, total: pages.length });
    const { width, height } = page.getSize();
    const textWidth = font.widthOfTextAtSize(text, fontSize);
    // Anchor so the text's center sits at the page center.
    const x = width / 2 - (textWidth / 2) * Math.cos(rad);
    const y = height / 2 - (textWidth / 2) * Math.sin(rad);
    page.drawText(text, {
      x,
      y,
      size: fontSize,
      font,
      color: rgb(color.r, color.g, color.b),
      opacity,
      rotate: degrees(angle),
    });
  }

  notifyProgress(onProgress, { phase: 'saving', label: 'Saving watermarked PDF…', current: pages.length, total: pages.length });
  return doc.save();
}
