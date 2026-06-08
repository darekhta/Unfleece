import { notifyProgress, type ProgressCallback } from '../progress.js';
import { wasmCropMargins } from '../wasm/core.js';

export interface CropMargins {
  /** Points to trim from each edge. */
  top?: number;
  right?: number;
  bottom?: number;
  left?: number;
}

/**
 * Crop every page by trimming the given margins (in points) from each edge.
 * Runs in the Rust core: only the CropBox is set (the MediaBox is untouched).
 * Rejects negative margins ('Crop margins cannot be negative'), non-finite
 * margins ('Crop margins must be finite numbers') and margins that consume a
 * page ('Crop margins exceed page size').
 */
export async function cropPdf(bytes: Uint8Array, margins: CropMargins, onProgress?: ProgressCallback): Promise<Uint8Array> {
  const { top = 0, right = 0, bottom = 0, left = 0 } = margins;
  notifyProgress(onProgress, { phase: 'working', label: 'Cropping pages in Rust core…' });
  const out = await wasmCropMargins(bytes, top, right, bottom, left);
  notifyProgress(onProgress, { phase: 'saving', label: 'Saving cropped PDF…' });
  return out;
}
