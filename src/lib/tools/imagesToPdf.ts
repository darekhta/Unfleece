import { notifyProgress, type ProgressCallback } from '../progress.js';
import { wasmImagesToPdf } from '../wasm/core.js';

export type ImageType = 'png' | 'jpg';
export interface InputImage {
  bytes: Uint8Array;
  type: ImageType;
}

export interface ImagesToPdfOptions {
  /** 'fit' => each page matches its image (pixels as points); otherwise a fixed page size. */
  pageSize?: 'fit' | 'a4' | 'letter';
  /** Margin in points when using a fixed page size (the core clamps it to 0..200). */
  margin?: number;
}

/**
 * Build a PDF from a list of images, one image per page. Runs in the Rust
 * core: JPEGs are embedded byte-for-byte (DCTDecode) and PNG alpha becomes
 * an /SMask. Fixed page sizes center the image at scale min(maxW/w, maxH/h, 1).
 */
export async function imagesToPdf(
  images: InputImage[],
  opts: ImagesToPdfOptions = {},
  onProgress?: ProgressCallback,
): Promise<Uint8Array> {
  if (images.length === 0) throw new Error('No images provided');
  const { pageSize = 'fit', margin = 24 } = opts;
  notifyProgress(onProgress, { phase: 'working', label: `Building a ${images.length}-page PDF in Rust core…`, current: 0, total: images.length });
  const out = await wasmImagesToPdf(images, { pageSize, margin });
  notifyProgress(onProgress, { phase: 'working', label: `Added ${images.length} image${images.length === 1 ? '' : 's'}.`, current: images.length, total: images.length });
  return out;
}
