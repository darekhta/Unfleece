// Browser bridge to the Rust→WASM `unfleece-core` engine. The wasm module is
// lazy-initialized on first use and cached. This is Unfleece's owned engine
// (lopdf/krilla-based): every PDF operation that doesn't need page *rendering*
// runs here, not in a JS library.
import init, {
  add_page_numbers,
  add_watermark,
  booklet,
  crop_margins,
  images_to_pdf,
  merge_pdfs,
  n_up,
  optimize,
  page_count,
  pdfa_from_png_pages,
  read_metadata,
  rotate_all,
  rotate_pages,
  sanitize,
  sanitize_report,
  select_pages,
  set_metadata,
  stamp_images,
  strip_metadata,
} from './pkg/unfleece_core.js';
import wasmUrl from './pkg/unfleece_core_bg.wasm?url';
import { PackBuilder } from './pack.js';

let ready: Promise<unknown> | null = null;
async function ensure(): Promise<void> {
  if (!ready) {
    ready = (async () => {
      // Node (vitest): fetch() can't load file paths — read the wasm bytes directly.
      // This branch is dead code in the browser bundle.
      if (typeof window === 'undefined' && typeof process !== 'undefined' && process.versions?.node) {
        const fs = await import(/* @vite-ignore */ 'node' + ':fs/promises');
        const bytes = await fs.readFile(new URL('./pkg/unfleece_core_bg.wasm', import.meta.url));
        return init({ module_or_path: bytes });
      }
      return init({ module_or_path: wasmUrl });
    })();
  }
  await ready;
}

/** Losslessly optimize a PDF (prune unused objects + flate-compress streams). */
export async function wasmOptimize(bytes: Uint8Array): Promise<Uint8Array> {
  await ensure();
  return optimize(bytes);
}

/** Rotate every page by `degrees`. */
export async function wasmRotateAll(bytes: Uint8Array, degrees: number): Promise<Uint8Array> {
  await ensure();
  return rotate_all(bytes, BigInt(degrees));
}

/** Count pages. */
export async function wasmPageCount(bytes: Uint8Array): Promise<number> {
  await ensure();
  return page_count(bytes);
}

/** Build a new PDF containing selected 0-based page indices, in order. */
export async function wasmSelectPages(bytes: Uint8Array, indices: number[] | Uint32Array): Promise<Uint8Array> {
  await ensure();
  return select_pages(bytes, indices instanceof Uint32Array ? indices : Uint32Array.from(indices));
}

/** Build a PDF/A-2b file from packed PNG page images. */
export async function wasmPdfAFromPngPages(pack: Uint8Array): Promise<Uint8Array> {
  await ensure();
  return pdfa_from_png_pages(pack);
}

/** Rotate only the given 0-based pages by `degrees` (relative). */
export async function wasmRotatePages(bytes: Uint8Array, indices: number[] | Uint32Array, degrees: number): Promise<Uint8Array> {
  await ensure();
  return rotate_pages(bytes, indices instanceof Uint32Array ? indices : Uint32Array.from(indices), BigInt(degrees));
}

/** Merge PDFs into one document, in order. */
export async function wasmMergePdfs(files: Uint8Array[]): Promise<Uint8Array> {
  await ensure();
  const pack = new PackBuilder('UFMG').u32(files.length);
  for (const f of files) pack.bytes(f);
  return merge_pdfs(pack.finish());
}

/** Trim margins (points) from every page by setting the page CropBox. */
export async function wasmCropMargins(bytes: Uint8Array, top: number, right: number, bottom: number, left: number): Promise<Uint8Array> {
  await ensure();
  return crop_margins(bytes, top, right, bottom, left);
}

export interface WasmMetadata {
  title?: string;
  author?: string;
  subject?: string;
  keywords?: string[];
  creator?: string;
  producer?: string;
  /** Verbatim PDF date string, e.g. "D:20260607120000Z". */
  creationDate?: string;
  modificationDate?: string;
  pageCount?: number;
}

/** Read document metadata (dates are verbatim PDF date strings). */
export async function wasmReadMetadata(bytes: Uint8Array): Promise<WasmMetadata> {
  await ensure();
  return JSON.parse(read_metadata(bytes)) as WasmMetadata;
}

/** Set the metadata fields present in `meta`; an empty string clears a field. */
export async function wasmSetMetadata(bytes: Uint8Array, meta: Record<string, unknown>): Promise<Uint8Array> {
  await ensure();
  return set_metadata(bytes, JSON.stringify(meta));
}

/** Remove the Info dictionary and XMP metadata stream. */
export async function wasmStripMetadata(bytes: Uint8Array): Promise<Uint8Array> {
  await ensure();
  return strip_metadata(bytes);
}

/** Strip scripts/attachments/actions; opts: {removeAnnotations?, removeForms?}. */
export async function wasmSanitize(bytes: Uint8Array, opts: Record<string, unknown>): Promise<Uint8Array> {
  await ensure();
  return sanitize(bytes, JSON.stringify(opts));
}

/** JSON report of what a default sanitize pass would remove. */
export async function wasmSanitizeReport(bytes: Uint8Array): Promise<Record<string, number>> {
  await ensure();
  return JSON.parse(sanitize_report(bytes)) as Record<string, number>;
}

/** Stamp page numbers (also powers Bates via format "PREFIX{n}" + padTo). */
export async function wasmAddPageNumbers(bytes: Uint8Array, opts: Record<string, unknown>): Promise<Uint8Array> {
  await ensure();
  return add_page_numbers(bytes, JSON.stringify(opts));
}

/** Draw a rotated, semi-transparent text watermark centered on every page. */
export async function wasmAddWatermark(bytes: Uint8Array, opts: Record<string, unknown>): Promise<Uint8Array> {
  await ensure();
  return add_watermark(bytes, JSON.stringify(opts));
}

export interface WasmImageStamp {
  image: Uint8Array;
  imageType: 'png' | 'jpg';
  pageIndex: number;
  x: number;
  y: number;
  width: number;
  /** 0 = derive from the image aspect ratio. */
  height?: number;
  opacity?: number;
}

/** Stamp PNG/JPEG images (signatures) onto pages in one pass. */
export async function wasmStampImages(bytes: Uint8Array, stamps: WasmImageStamp[]): Promise<Uint8Array> {
  await ensure();
  const pack = new PackBuilder('UFST').u32(stamps.length);
  for (const s of stamps) {
    pack
      .u8(s.imageType === 'png' ? 0 : 1)
      .u32(s.pageIndex)
      .f32(s.x)
      .f32(s.y)
      .f32(s.width)
      .f32(s.height ?? 0)
      .f32(s.opacity ?? 1)
      .bytes(s.image);
  }
  return stamp_images(bytes, pack.finish());
}

/** Build a PDF from images, one page per image. opts: {pageSize, margin}. */
export async function wasmImagesToPdf(
  images: { bytes: Uint8Array; type: 'png' | 'jpg' }[],
  opts: Record<string, unknown>,
): Promise<Uint8Array> {
  await ensure();
  const pack = new PackBuilder('UFIP').u32(images.length);
  for (const img of images) {
    pack.u8(img.type === 'png' ? 0 : 1).bytes(img.bytes);
  }
  return images_to_pdf(pack.finish(), JSON.stringify(opts));
}

/** Place `perSheet` source pages onto each A4 sheet (handout N-up). */
export async function wasmNUp(bytes: Uint8Array, perSheet: number): Promise<Uint8Array> {
  await ensure();
  return n_up(bytes, perSheet, '{}');
}

/** Reorder pages into two-up booklet spreads on landscape sheets. */
export async function wasmBooklet(bytes: Uint8Array, opts: Record<string, unknown>): Promise<Uint8Array> {
  await ensure();
  return booklet(bytes, JSON.stringify(opts));
}
