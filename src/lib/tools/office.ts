// PDF text → Office documents (DOCX/XLSX/PPTX). The text is extracted from the
// PDF by pdf.js (browser), then the OOXML ZIP containers — including row/column
// reconstruction — are built by the Rust→WASM core. No JSZip.
import {
  wasmTextPagesToDocx,
  wasmTextPagesToXlsx,
  wasmTextPagesToPptx,
  type WasmTextPage,
} from '../wasm/core.js';

export interface ExtractedTextItem {
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ExtractedTextPage {
  pageNumber: number;
  text: string;
  items: ExtractedTextItem[];
  width?: number;
  height?: number;
}

const DOCX_MIME = 'application/vnd.openxmlformats-officedocument.wordprocessingml.document';
const XLSX_MIME = 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet';
const PPTX_MIME = 'application/vnd.openxmlformats-officedocument.presentationml.presentation';

export const OFFICE_MIME = {
  docx: DOCX_MIME,
  xlsx: XLSX_MIME,
  pptx: PPTX_MIME,
} as const;

/** Map the pdf.js-derived pages onto the Rust core's pack shape (no page text — Rust rebuilds it from items). */
function toWasmPages(pages: ExtractedTextPage[]): WasmTextPage[] {
  return pages.map((p) => ({ pageNumber: p.pageNumber, width: p.width, height: p.height, items: p.items }));
}

export async function textPagesToDocx(pages: ExtractedTextPage[]): Promise<Uint8Array> {
  return wasmTextPagesToDocx(toWasmPages(pages));
}

export async function textPagesToXlsx(pages: ExtractedTextPage[]): Promise<Uint8Array> {
  return wasmTextPagesToXlsx(toWasmPages(pages));
}

export async function textPagesToPptx(pages: ExtractedTextPage[]): Promise<Uint8Array> {
  return wasmTextPagesToPptx(toWasmPages(pages));
}
