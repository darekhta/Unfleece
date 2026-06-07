// Browser bridge to the Rust→WASM `unfleece-core` engine. The wasm module is
// lazy-initialized on first use and cached. This is Unfleece's owned engine
// (lopdf/krilla-based): every PDF operation that doesn't need page *rendering*
// runs here, not in a JS library.
import init, {
  add_page_numbers,
  add_watermark,
  booklet,
  crop_margins,
  fill_form,
  fixed_pages_to_epub,
  flatten_form,
  images_to_pdf,
  is_encrypted,
  list_form_fields,
  merge_pdfs,
  n_up,
  optimize,
  page_count,
  pdfa_from_png_pages,
  protect,
  read_metadata,
  rotate_all,
  rotate_pages,
  sanitize,
  sanitize_report,
  select_pages,
  set_metadata,
  stamp_images,
  strip_metadata,
  image_pages_to_pdf,
  add_text_layer,
  set_crop_boxes,
  text_pages_to_docx,
  text_pages_to_epub,
  text_pages_to_pptx,
  text_pages_to_xlsx,
  text_to_pdf,
  unlock,
} from './pkg/unfleece_core.js';
import wasmUrl from './pkg/unfleece_core_bg.wasm?url';
import { PackBuilder } from './pack.js';

let ready: Promise<unknown> | null = null;
async function initCore(): Promise<unknown> {
  // Node (vitest): fetch() can't load file paths — read the wasm bytes directly.
  // This branch is dead code in the browser bundle.
  if (typeof window === 'undefined' && typeof process !== 'undefined' && process.versions?.node) {
    const fsModule = 'node:fs/promises';
    const fs = await import(/* @vite-ignore */ fsModule);
    const bytes = await fs.readFile(new URL('./pkg/unfleece_core_bg.wasm', import.meta.url));
    return init({ module_or_path: bytes });
  }
  return init({ module_or_path: wasmUrl });
}

async function ensure(): Promise<void> {
  if (!ready) {
    ready = initCore().catch((error) => {
      ready = null;
      throw error;
    });
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

// ---- AcroForm fields (fill/flatten with appearance streams) ----
export interface WasmFormField {
  name: string;
  type: 'text' | 'checkbox' | 'radio' | 'dropdown' | 'optionlist' | 'button' | 'signature' | 'unknown';
  options?: string[];
  value?: string | boolean;
}

/** List every AcroForm field with its type, options and current value. */
export async function wasmListFormFields(bytes: Uint8Array): Promise<WasmFormField[]> {
  await ensure();
  return JSON.parse(list_form_fields(bytes)) as WasmFormField[];
}

/** Fill form fields and regenerate appearance streams. Unknown names are ignored. */
export async function wasmFillForm(bytes: Uint8Array, values: Record<string, string | boolean>): Promise<Uint8Array> {
  await ensure();
  return fill_form(bytes, JSON.stringify(values));
}

/** Bake field appearances into the page and drop the AcroForm + widgets. */
export async function wasmFlattenForm(bytes: Uint8Array): Promise<Uint8Array> {
  await ensure();
  return flatten_form(bytes);
}

// ---- Standard Security (protect / unlock) ----
/** Does the PDF carry an /Encrypt dictionary? (no password needed) */
export async function wasmIsEncrypted(bytes: Uint8Array): Promise<boolean> {
  await ensure();
  return is_encrypted(bytes);
}

/** Decrypt with a user OR owner password; returns unlocked PDF bytes. */
export async function wasmUnlock(bytes: Uint8Array, password: string): Promise<Uint8Array> {
  await ensure();
  return unlock(bytes, password);
}

export interface WasmProtectOptions {
  userPassword: string;
  ownerPassword?: string;
  allowPrinting?: boolean;
  allowCopying?: boolean;
  allowModifying?: boolean;
}

/** Encrypt with AESV2 Standard Security (V4/R4). */
export async function wasmProtect(bytes: Uint8Array, opts: WasmProtectOptions): Promise<Uint8Array> {
  await ensure();
  return protect(bytes, JSON.stringify(opts));
}

// ---- text → PDF ----
/** Lay out preprocessed plain text into a PDF. opts: {title?, pageSize?, fontSize?, margin?}. */
export async function wasmTextToPdf(text: string, opts: Record<string, unknown>): Promise<Uint8Array> {
  await ensure();
  return text_to_pdf(text, JSON.stringify(opts));
}

// ---- office / EPUB containers from extracted text pages ----
export interface WasmTextItem {
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface WasmTextPage {
  pageNumber: number;
  items: WasmTextItem[];
  width?: number;
  height?: number;
}

function packTextPages(pages: WasmTextPage[]): Uint8Array {
  const pack = new PackBuilder('UFTP').u32(pages.length);
  for (const p of pages) {
    pack.u32(p.pageNumber >>> 0).f32(p.width ?? 0).f32(p.height ?? 0).u32(p.items.length);
    for (const it of p.items) pack.f32(it.x).f32(it.y).f32(it.width).f32(it.height).str(it.text);
  }
  return pack.finish();
}

/** Build a DOCX from extracted PDF text pages. */
export async function wasmTextPagesToDocx(pages: WasmTextPage[]): Promise<Uint8Array> {
  await ensure();
  return text_pages_to_docx(packTextPages(pages));
}
/** Build an XLSX from extracted PDF text pages. */
export async function wasmTextPagesToXlsx(pages: WasmTextPage[]): Promise<Uint8Array> {
  await ensure();
  return text_pages_to_xlsx(packTextPages(pages));
}
/** Build a PPTX from extracted PDF text pages. */
export async function wasmTextPagesToPptx(pages: WasmTextPage[]): Promise<Uint8Array> {
  await ensure();
  return text_pages_to_pptx(packTextPages(pages));
}

export interface WasmReflowableEpubOptions {
  title?: string;
  author?: string;
  language?: string;
  identifier?: string;
  modified?: string;
  removeHeadersFooters?: boolean;
  unwrapParagraphs?: boolean;
  repairHyphenation?: boolean;
}

/** Reflowable EPUB from extracted text pages. */
export async function wasmTextPagesToEpub(pages: WasmTextPage[], opts: WasmReflowableEpubOptions = {}): Promise<Uint8Array> {
  await ensure();
  return text_pages_to_epub(packTextPages(pages), JSON.stringify({ modified: new Date().toISOString(), ...opts }));
}

export interface WasmFixedEpubPage {
  pageNumber: number;
  width: number;
  height: number;
  bytes: Uint8Array;
  type: 'png' | 'jpg';
}
export interface WasmFixedEpubOptions {
  title?: string;
  author?: string;
  language?: string;
  identifier?: string;
  modified?: string;
}

/** Fixed-layout EPUB from rendered page images. */
export async function wasmFixedPagesToEpub(pages: WasmFixedEpubPage[], opts: WasmFixedEpubOptions = {}): Promise<Uint8Array> {
  await ensure();
  const pack = new PackBuilder('UFXP').u32(pages.length);
  for (const p of pages) pack.u32(p.pageNumber >>> 0).f32(p.width).f32(p.height).u8(p.type === 'png' ? 0 : 1).bytes(p.bytes);
  return fixed_pages_to_epub(pack.finish(), JSON.stringify({ modified: new Date().toISOString(), ...opts }));
}

// ---- image-page assembly (redact / raster compress / OCR / auto-crop) ----
export interface WasmImagePage {
  widthPt: number;
  heightPt: number;
  bytes: Uint8Array;
  type: 'png' | 'jpg';
}

/** Build an image-only PDF: one page per image, each filling its point-size page. */
export async function wasmImagePagesToPdf(pages: WasmImagePage[]): Promise<Uint8Array> {
  await ensure();
  const pack = new PackBuilder('UFAP').u32(pages.length);
  for (const p of pages) {
    pack.f32(p.widthPt).f32(p.heightPt).u8(p.type === 'png' ? 0 : 1).bytes(p.bytes);
  }
  return image_pages_to_pdf(pack.finish());
}

export interface WasmTextSpan {
  x: number;
  y: number;
  fontSize: number;
  text: string;
}
export interface WasmTextLayerPage {
  pageIndex: number;
  spans: WasmTextSpan[];
}

/** Append an invisible Helvetica text layer (PDF points, bottom-left origin) for OCR. */
export async function wasmAddTextLayer(bytes: Uint8Array, pages: WasmTextLayerPage[]): Promise<Uint8Array> {
  await ensure();
  const pack = new PackBuilder('UFTL').u32(pages.length);
  for (const p of pages) {
    pack.u32(p.pageIndex >>> 0).u32(p.spans.length);
    for (const s of p.spans) pack.f32(s.x).f32(s.y).f32(s.fontSize).str(s.text);
  }
  return add_text_layer(bytes, pack.finish());
}

export interface WasmCropBox {
  pageIndex: number;
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Set a per-page CropBox (absolute PDF coords) on listed pages. */
export async function wasmSetCropBoxes(bytes: Uint8Array, boxes: WasmCropBox[]): Promise<Uint8Array> {
  await ensure();
  const pack = new PackBuilder('UFCB').u32(boxes.length);
  for (const b of boxes) pack.u32(b.pageIndex >>> 0).f32(b.x).f32(b.y).f32(b.width).f32(b.height);
  return set_crop_boxes(bytes, pack.finish());
}
