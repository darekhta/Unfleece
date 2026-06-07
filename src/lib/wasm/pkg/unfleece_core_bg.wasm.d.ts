/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const add_page_numbers: (a: number, b: number, c: number, d: number) => [number, number, number, number];
export const add_watermark: (a: number, b: number, c: number, d: number) => [number, number, number, number];
export const booklet: (a: number, b: number, c: number, d: number) => [number, number, number, number];
export const crop_margins: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
export const images_to_pdf: (a: number, b: number, c: number, d: number) => [number, number, number, number];
export const merge_pdfs: (a: number, b: number) => [number, number, number, number];
export const n_up: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
export const optimize: (a: number, b: number) => [number, number, number, number];
export const page_count: (a: number, b: number) => [number, number, number];
export const pdfa_from_png_pages: (a: number, b: number) => [number, number, number, number];
export const read_metadata: (a: number, b: number) => [number, number, number, number];
export const rotate_all: (a: number, b: number, c: bigint) => [number, number, number, number];
export const rotate_pages: (a: number, b: number, c: number, d: number, e: bigint) => [number, number, number, number];
export const sanitize: (a: number, b: number, c: number, d: number) => [number, number, number, number];
export const sanitize_report: (a: number, b: number) => [number, number, number, number];
export const select_pages: (a: number, b: number, c: number, d: number) => [number, number, number, number];
export const set_metadata: (a: number, b: number, c: number, d: number) => [number, number, number, number];
export const stamp_images: (a: number, b: number, c: number, d: number) => [number, number, number, number];
export const strip_metadata: (a: number, b: number) => [number, number, number, number];
export const __wbindgen_externrefs: WebAssembly.Table;
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __externref_table_dealloc: (a: number) => void;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_start: () => void;
