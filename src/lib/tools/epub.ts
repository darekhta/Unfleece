// PDF → EPUB. Text (reflowable) or rendered page images (fixed-layout) come
// from pdf.js (browser); the EPUB ZIP container — and, for reflowable, the
// paragraph/column/header-footer reconstruction — is built by the Rust→WASM
// core. No JSZip.
import type { RenderedPage } from '../browser/render.js';
import type { ExtractedTextPage } from './office.js';
import {
  wasmTextPagesToEpub,
  wasmFixedPagesToEpub,
  type WasmReflowableEpubOptions,
  type WasmFixedEpubPage,
} from '../wasm/core.js';

export const EPUB_MIME = 'application/epub+zip';

export interface EpubMetadataOptions {
  title?: string;
  author?: string;
  language?: string;
  identifier?: string;
  modified?: string | Date;
}

export interface ReflowableEpubOptions extends EpubMetadataOptions {
  removeHeadersFooters?: boolean;
  unwrapParagraphs?: boolean;
  repairHyphenation?: boolean;
}

export interface FixedLayoutEpubOptions extends EpubMetadataOptions {}

export interface FixedLayoutPage extends RenderedPage {
  pageNumber?: number;
  width: number;
  height: number;
}

function isoModified(modified?: string | Date): string | undefined {
  if (modified === undefined) return undefined;
  return modified instanceof Date ? modified.toISOString() : modified;
}

export async function textPagesToEpub(pages: ExtractedTextPage[], options: ReflowableEpubOptions = {}): Promise<Uint8Array> {
  const opts: WasmReflowableEpubOptions = {
    title: options.title,
    author: options.author,
    language: options.language,
    identifier: options.identifier,
    modified: isoModified(options.modified),
    removeHeadersFooters: options.removeHeadersFooters,
    unwrapParagraphs: options.unwrapParagraphs,
    repairHyphenation: options.repairHyphenation,
  };
  return wasmTextPagesToEpub(
    pages.map((p) => ({ pageNumber: p.pageNumber, width: p.width, height: p.height, items: p.items })),
    opts,
  );
}

export async function fixedPagesToEpub(pages: FixedLayoutPage[], options: FixedLayoutEpubOptions = {}): Promise<Uint8Array> {
  const packed: WasmFixedEpubPage[] = [];
  for (const [i, page] of pages.entries()) {
    const bytes = new Uint8Array(await page.blob.arrayBuffer());
    packed.push({
      pageNumber: page.pageNumber ?? i + 1,
      width: page.width,
      height: page.height,
      bytes,
      type: page.blob.type === 'image/png' ? 'png' : 'jpg',
    });
  }
  return wasmFixedPagesToEpub(packed, {
    title: options.title,
    author: options.author,
    language: options.language,
    identifier: options.identifier,
    modified: isoModified(options.modified),
  });
}
