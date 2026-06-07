import { PDFDocument } from '@cantoo/pdf-lib';
import { PAGE_SIZES } from '../util/pdf.js';
import { notifyProgress, type ProgressCallback } from '../progress.js';

export type ImageType = 'png' | 'jpg';
export interface InputImage {
  bytes: Uint8Array;
  type: ImageType;
}

export interface ImagesToPdfOptions {
  /** 'fit' => each page matches its image; otherwise a fixed page size. */
  pageSize?: 'fit' | 'a4' | 'letter';
  /** Margin in points when using a fixed page size. */
  margin?: number;
}

/** Build a PDF from a list of images, one image per page. */
export async function imagesToPdf(
  images: InputImage[],
  opts: ImagesToPdfOptions = {},
  onProgress?: ProgressCallback,
): Promise<Uint8Array> {
  if (images.length === 0) throw new Error('No images provided');
  const { pageSize = 'fit', margin = 24 } = opts;

  const doc = await PDFDocument.create();
  for (const [i, img] of images.entries()) {
    notifyProgress(onProgress, { phase: 'working', label: `Adding image ${i + 1} of ${images.length}…`, current: i, total: images.length });
    const embedded =
      img.type === 'png' ? await doc.embedPng(img.bytes) : await doc.embedJpg(img.bytes);

    if (pageSize === 'fit') {
      const page = doc.addPage([embedded.width, embedded.height]);
      page.drawImage(embedded, { x: 0, y: 0, width: embedded.width, height: embedded.height });
    } else {
      const [pw, ph] = PAGE_SIZES[pageSize];
      const page = doc.addPage([pw, ph]);
      const maxW = pw - margin * 2;
      const maxH = ph - margin * 2;
      const scale = Math.min(maxW / embedded.width, maxH / embedded.height, 1);
      const w = embedded.width * scale;
      const h = embedded.height * scale;
      page.drawImage(embedded, { x: (pw - w) / 2, y: (ph - h) / 2, width: w, height: h });
    }
    notifyProgress(onProgress, { phase: 'working', label: `Added image ${i + 1} of ${images.length}.`, current: i + 1, total: images.length });
  }
  return doc.save();
}
