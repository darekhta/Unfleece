import { loadPdf } from '../util/pdf.js';
import { notifyProgress, type ProgressCallback } from '../progress.js';

export interface CropMargins {
  /** Points to trim from each edge. */
  top?: number;
  right?: number;
  bottom?: number;
  left?: number;
}

/**
 * Crop every page by trimming the given margins (in points) from each edge.
 * Adjusts the crop box relative to the current media box.
 */
export async function cropPdf(bytes: Uint8Array, margins: CropMargins, onProgress?: ProgressCallback): Promise<Uint8Array> {
  const { top = 0, right = 0, bottom = 0, left = 0 } = margins;
  if (top < 0 || right < 0 || bottom < 0 || left < 0) {
    throw new Error('Crop margins cannot be negative');
  }

  notifyProgress(onProgress, { phase: 'loading', label: 'Loading PDF…' });
  const doc = await loadPdf(bytes);
  const pages = doc.getPages();
  for (const [i, page] of pages.entries()) {
    notifyProgress(onProgress, { phase: 'working', label: `Cropping page ${i + 1} of ${pages.length}…`, current: i, total: pages.length });
    const media = page.getMediaBox();
    const newWidth = media.width - left - right;
    const newHeight = media.height - top - bottom;
    if (newWidth <= 0 || newHeight <= 0) {
      throw new Error('Crop margins exceed page size');
    }
    page.setCropBox(media.x + left, media.y + bottom, newWidth, newHeight);
  }
  notifyProgress(onProgress, { phase: 'saving', label: 'Saving cropped PDF…', current: pages.length, total: pages.length });
  return doc.save();
}
