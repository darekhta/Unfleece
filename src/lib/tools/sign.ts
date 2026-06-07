import { wasmPageCount, wasmStampImages, type WasmImageStamp } from '../wasm/core.js';
import type { ImageType } from './imagesToPdf.js';

export interface StampOptions {
  image: Uint8Array;
  imageType: ImageType;
  /** 0-based page index; defaults to the last page. */
  pageIndex?: number;
  /** Position in points from the bottom-left of the page. */
  x: number;
  y: number;
  width: number;
  height?: number;
  opacity?: number;
}

/** Resolve TS-side defaults into the wire shape the Rust core expects. */
function toWasmStamp(s: StampPlacement): WasmImageStamp {
  return {
    image: s.image,
    imageType: s.imageType,
    pageIndex: s.pageIndex,
    x: s.x,
    y: s.y,
    width: s.width,
    height: s.height ?? 0, // 0 = derive height from the image aspect ratio
    opacity: s.opacity ?? 1,
  };
}

/**
 * Stamp an image (e.g. a drawn/typed signature) onto a page. This is an
 * *electronic* signature (a visual mark) — it carries no cryptographic proof.
 * Runs in the Rust core: JPEGs pass through byte-for-byte and PNG alpha
 * becomes an /SMask.
 */
export async function stampImage(bytes: Uint8Array, opts: StampOptions): Promise<Uint8Array> {
  const pageIndex = opts.pageIndex ?? (await wasmPageCount(bytes)) - 1;
  return wasmStampImages(bytes, [toWasmStamp({ ...opts, pageIndex })]);
}

export interface StampPlacement {
  image: Uint8Array;
  imageType: ImageType;
  /** 0-based page index. */
  pageIndex: number;
  /** Position in points from the bottom-left of the page. */
  x: number;
  y: number;
  width: number;
  height?: number;
  opacity?: number;
}

/**
 * Stamp several images in one pass (multi-page signing). The Rust core loads
 * the document once and dedupes identical image bytes to a single XObject.
 */
export async function stampImageMany(bytes: Uint8Array, stamps: StampPlacement[]): Promise<Uint8Array> {
  if (stamps.length === 0) throw new Error('No signatures placed');
  return wasmStampImages(bytes, stamps.map(toWasmStamp));
}
