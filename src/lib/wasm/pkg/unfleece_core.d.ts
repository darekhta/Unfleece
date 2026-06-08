/* tslint:disable */
/* eslint-disable */

/**
 * Stamp page numbers onto every page (JSON options: format with `{n}`/`{total}`,
 * 6-zone position, fontSize, margin, startAt, padTo — also powers Bates numbering
 * via `format: "PREFIX{n}"`).
 */
export function add_page_numbers(data: Uint8Array, opts_json: string): Uint8Array;

/**
 * Append an invisible Helvetica text layer onto existing pages (OCR).
 */
export function add_text_layer(data: Uint8Array, pack: Uint8Array): Uint8Array;

/**
 * Draw a rotated, semi-transparent text watermark centered on every page
 * (JSON options: text, fontSize, opacity, angle, color).
 */
export function add_watermark(data: Uint8Array, opts_json: string): Uint8Array;

/**
 * Reorder pages into two-up booklet spreads on landscape sheets.
 */
export function booklet(data: Uint8Array, opts_json: string): Uint8Array;

/**
 * Trim margins (in points) from every page by setting the page CropBox.
 */
export function crop_margins(data: Uint8Array, top: number, right: number, bottom: number, left: number): Uint8Array;

/**
 * Fill form fields from a `{fieldName: string|bool}` JSON object,
 * regenerating appearances and clearing NeedAppearances.
 */
export function fill_form(data: Uint8Array, values_json: string): Uint8Array;

/**
 * Build a fixed-layout EPUB from a `UFXP` page-image pack. `opts_json` mirrors
 * FixedLayoutEpubOptions (`{title?, author?, language?, identifier?, modified?}`).
 */
export function fixed_pages_to_epub(pack: Uint8Array, opts_json: string): Uint8Array;

/**
 * Flatten the form: bake appearances into page content and drop widgets + AcroForm.
 */
export function flatten_form(data: Uint8Array): Uint8Array;

/**
 * Build an image-only PDF (one page per packed image) — redact / raster compress.
 */
export function image_pages_to_pdf(pack: Uint8Array): Uint8Array;

/**
 * Build a PDF from a packed list of PNG/JPEG images, one page per image.
 * `opts_json` mirrors imagesToPdf.ts: `{"pageSize":"fit"|"a4"|"letter","margin":24}`.
 */
export function images_to_pdf(pack: Uint8Array, opts_json: string): Uint8Array;

/**
 * Report whether the PDF carries an `/Encrypt` dictionary (password peek).
 */
export function is_encrypted(data: Uint8Array): boolean;

/**
 * List AcroForm fields as a JSON array of `{name, type, options?, value?}`.
 */
export function list_form_fields(data: Uint8Array): string;

/**
 * Merge multiple PDFs (packed as `UFMG`) into one document, in pack order.
 */
export function merge_pdfs(pack: Uint8Array): Uint8Array;

/**
 * Place `per_sheet` source pages onto each A4 sheet (handout-style N-up).
 */
export function n_up(data: Uint8Array, per_sheet: number, opts_json: string): Uint8Array;

/**
 * Losslessly optimize the PDF. Returns the new PDF bytes.
 */
export function optimize(data: Uint8Array): Uint8Array;

/**
 * Count pages.
 */
export function page_count(data: Uint8Array): number;

/**
 * Build a PDF/A-2b file from a packed list of PNG page images.
 */
export function pdfa_from_png_pages(pack: Uint8Array): Uint8Array;

/**
 * Encrypt with AESV2 Standard Security. `opts_json` mirrors
 * `ProtectOptions` (`userPassword`, `ownerPassword?`, `allowPrinting?`,
 * `allowCopying?`, `allowModifying?`).
 */
export function protect(data: Uint8Array, opts_json: string): Uint8Array;

/**
 * Read document metadata as a JSON string
 * (`{title?, author?, subject?, keywords?, creator?, producer?, creationDate?, modificationDate?, pageCount}`).
 */
export function read_metadata(data: Uint8Array): string;

/**
 * Rotate every page by `degrees`. Returns the new PDF bytes.
 */
export function rotate_all(data: Uint8Array, degrees: bigint): Uint8Array;

/**
 * Rotate only the given 0-based pages by `degrees`.
 */
export function rotate_pages(data: Uint8Array, indices: Uint32Array, degrees: bigint): Uint8Array;

/**
 * Strip metadata, scripts, embedded files and other risky structures.
 * `opts_json` mirrors the sanitize-pdf tool options (`removeAnnotations`,
 * `removeForms`; both default to true).
 */
export function sanitize(data: Uint8Array, opts_json: string): Uint8Array;

/**
 * JSON report of what a default sanitize pass would remove, without
 * producing output bytes.
 */
export function sanitize_report(data: Uint8Array): string;

/**
 * Build a new PDF containing the selected 0-based page indices, in order.
 */
export function select_pages(data: Uint8Array, indices: Uint32Array): Uint8Array;

/**
 * Set a per-page CropBox on listed pages (auto-crop).
 */
export function set_crop_boxes(data: Uint8Array, pack: Uint8Array): Uint8Array;

/**
 * Overwrite the metadata fields present in the options JSON; an empty string clears a field.
 */
export function set_metadata(data: Uint8Array, json: string): Uint8Array;

/**
 * Stamp PNG/JPEG images (e.g. signatures) onto pages, per the `UFST` pack.
 */
export function stamp_images(data: Uint8Array, pack: Uint8Array): Uint8Array;

/**
 * Remove the Info dictionary and the XMP metadata stream (privacy hygiene).
 */
export function strip_metadata(data: Uint8Array): Uint8Array;

/**
 * Build a DOCX from extracted PDF text pages (the `UFTP` pack).
 */
export function text_pages_to_docx(pack: Uint8Array): Uint8Array;

/**
 * Build a reflowable EPUB from a `UFTP` text-pages pack. `opts_json` mirrors
 * ReflowableEpubOptions (`{title?, author?, language?, identifier?, modified?,
 * removeHeadersFooters?, unwrapParagraphs?, repairHyphenation?}`).
 */
export function text_pages_to_epub(pack: Uint8Array, opts_json: string): Uint8Array;

/**
 * Build a PPTX from extracted PDF text pages (the `UFTP` pack).
 */
export function text_pages_to_pptx(pack: Uint8Array): Uint8Array;

/**
 * Build an XLSX from extracted PDF text pages (the `UFTP` pack).
 */
export function text_pages_to_xlsx(pack: Uint8Array): Uint8Array;

/**
 * Lay out preprocessed plain text into a paged PDF.
 * `opts_json` mirrors documentPdf.ts: `{title?, pageSize?: "a4"|"letter", fontSize?, margin?}`.
 */
export function text_to_pdf(text: string, opts_json: string): Uint8Array;

/**
 * Decrypt with a user or owner password; returns the unencrypted PDF.
 */
export function unlock(data: Uint8Array, password: string): Uint8Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly add_page_numbers: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly add_text_layer: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly add_watermark: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly booklet: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly crop_margins: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly fill_form: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly fixed_pages_to_epub: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly flatten_form: (a: number, b: number) => [number, number, number, number];
    readonly image_pages_to_pdf: (a: number, b: number) => [number, number, number, number];
    readonly images_to_pdf: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly is_encrypted: (a: number, b: number) => [number, number, number];
    readonly list_form_fields: (a: number, b: number) => [number, number, number, number];
    readonly merge_pdfs: (a: number, b: number) => [number, number, number, number];
    readonly n_up: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly optimize: (a: number, b: number) => [number, number, number, number];
    readonly page_count: (a: number, b: number) => [number, number, number];
    readonly pdfa_from_png_pages: (a: number, b: number) => [number, number, number, number];
    readonly protect: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly read_metadata: (a: number, b: number) => [number, number, number, number];
    readonly rotate_all: (a: number, b: number, c: bigint) => [number, number, number, number];
    readonly rotate_pages: (a: number, b: number, c: number, d: number, e: bigint) => [number, number, number, number];
    readonly sanitize: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly sanitize_report: (a: number, b: number) => [number, number, number, number];
    readonly select_pages: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly set_crop_boxes: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly set_metadata: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly stamp_images: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly strip_metadata: (a: number, b: number) => [number, number, number, number];
    readonly text_pages_to_docx: (a: number, b: number) => [number, number, number, number];
    readonly text_pages_to_epub: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly text_pages_to_pptx: (a: number, b: number) => [number, number, number, number];
    readonly text_pages_to_xlsx: (a: number, b: number) => [number, number, number, number];
    readonly text_to_pdf: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly unlock: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
