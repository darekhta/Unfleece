import { notifyProgress, type ProgressCallback } from '../progress.js';
import { wasmAddPageNumbers, wasmAddWatermark } from '../wasm/core.js';

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

/** Stamp page numbers onto every page (runs in the Rust core). */
export async function addPageNumbers(
  bytes: Uint8Array,
  opts: PageNumberOptions = {},
  onProgress?: ProgressCallback,
): Promise<Uint8Array> {
  notifyProgress(onProgress, { phase: 'working', label: 'Adding page numbers…' });
  const out = await wasmAddPageNumbers(bytes, { ...opts });
  notifyProgress(onProgress, { phase: 'saving', label: 'Saving numbered PDF…' });
  return out;
}

export interface WatermarkOptions {
  text: string;
  fontSize?: number;
  opacity?: number;
  angle?: number;
  color?: { r: number; g: number; b: number };
}

/** Draw a diagonal text watermark centered on every page (runs in the Rust core). */
export async function addTextWatermark(
  bytes: Uint8Array,
  opts: WatermarkOptions,
  onProgress?: ProgressCallback,
): Promise<Uint8Array> {
  // The Rust core rejects empty/missing text with 'Watermark text is required'.
  notifyProgress(onProgress, { phase: 'working', label: 'Adding watermark…' });
  const out = await wasmAddWatermark(bytes, { ...opts });
  notifyProgress(onProgress, { phase: 'saving', label: 'Saving watermarked PDF…' });
  return out;
}
