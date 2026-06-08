import JSZip from 'jszip';
import * as Comlink from 'comlink';
import { getPdfWorker, terminatePdfWorker } from './worker/client.js';
import type { PdfWorkerApi } from './worker/pdf.worker.js';
import { renderToImages, extractText, extractTextPages } from './browser/render.js';
import { comparePdfs } from './browser/compare.js';
import { autoCropPdf } from './browser/autoCrop.js';
import { compressImage, convertImage, extensionFor, type ImageMime } from './browser/image.js';
import { compressPdf } from './browser/compress.js';
import { ocrSearchablePdf } from './browser/ocr.js';
import { exportPdfA } from './browser/pdfa.js';
import { OFFICE_MIME, textPagesToDocx, textPagesToPptx, textPagesToXlsx } from './tools/office.js';
import { EPUB_MIME, fixedPagesToEpub, textPagesToEpub } from './tools/epub.js';
import { textToPdf } from './tools/documentPdf.js';
import type { Tool } from './registry.js';
import { fileMatchesAccept } from './handoff.js';
import { abortError, reportProgress, throwIfAborted, type ProgressCallback, type RunOptions } from './progress.js';

export interface RunResult {
  files: { name: string; blob: Blob }[];
  text?: string;
}

const baseName = (name: string) => name.replace(/\.[^.]+$/, '');
const toBytes = async (f: File) => new Uint8Array(await f.arrayBuffer());
const pdfBlob = (bytes: Uint8Array) => new Blob([bytes as BlobPart], { type: 'application/pdf' });
const officeBlob = (bytes: Uint8Array, type: string) => new Blob([bytes as BlobPart], { type });
const epubBlob = (bytes: Uint8Array) => new Blob([bytes as BlobPart], { type: EPUB_MIME });

function numberOption(value: unknown, fallback: number, min: number, max: number): number {
  const n = Number(value);
  if (!Number.isFinite(n)) return fallback;
  return Math.min(max, Math.max(min, n));
}

function validateInputs(tool: Tool, inputs: File[]) {
  if (inputs.length === 0) throw new Error('Please add a file');
  if (!tool.multiple && inputs.length > 1) throw new Error(`${tool.name} accepts one file at a time`);
  const rejected = inputs.find((f) => !fileMatchesAccept(f, tool.accept));
  if (rejected) throw new Error(`${rejected.name} is not supported by ${tool.name}`);
}

/** Coerce raw form values into typed option values using the tool's schema. */
export function coerceOptions(tool: Tool, raw: Record<string, unknown>): Record<string, any> {
  const out: Record<string, any> = {};
  for (const f of tool.options ?? []) {
    let v = raw[f.name];
    if (v === undefined || v === '') v = 'default' in f ? (f.default as unknown) : v;
    if (f.type === 'number') out[f.name] = v === undefined || v === '' ? undefined : Number(v);
    else if (f.type === 'checkbox') out[f.name] = Boolean(v);
    else out[f.name] = v ?? '';
  }
  return out;
}

async function zipFiles(files: { name: string; blob: Blob }[], zipName: string, run?: RunOptions): Promise<RunResult> {
  const zip = new JSZip();
  for (const [i, f] of files.entries()) {
    reportProgress(run, { phase: 'zipping', label: `Preparing ${i + 1} of ${files.length} files…`, current: i, total: files.length });
    zip.file(f.name, await f.blob.arrayBuffer());
  }
  reportProgress(run, { phase: 'zipping', label: 'Creating ZIP…', current: 0, total: 100 });
  const blob = await zip.generateAsync({ type: 'blob', compression: 'DEFLATE' }, (metadata) => {
    reportProgress(run, { phase: 'zipping', label: `Creating ZIP… ${Math.round(metadata.percent)}%`, current: metadata.percent, total: 100 });
  });
  throwIfAborted(run?.signal);
  return { files: [{ name: zipName, blob }] };
}

async function withPdfWorker<T>(
  run: RunOptions,
  fn: (worker: Comlink.Remote<PdfWorkerApi>, onProgress?: ProgressCallback) => Promise<T>,
): Promise<T> {
  const onProgress = run.onProgress
    ? Comlink.proxy((progress: Parameters<ProgressCallback>[0]) => reportProgress(run, progress))
    : undefined;
  const onAbort = () => terminatePdfWorker();
  run.signal?.addEventListener('abort', onAbort, { once: true });
  try {
    const result = await fn(getPdfWorker(), onProgress);
    throwIfAborted(run.signal);
    return result;
  } catch (e) {
    if (run.signal?.aborted) throw abortError();
    throw e;
  } finally {
    run.signal?.removeEventListener('abort', onAbort);
  }
}

/**
 * Execute a tool against the given input files and options, returning
 * downloadable result blobs. `fill-form` and `sign` are handled by their own
 * interactive components and are not routed here.
 */
export async function runTool(tool: Tool, inputs: File[], rawOpts: Record<string, unknown> = {}, run: RunOptions = {}): Promise<RunResult> {
  if (tool.special) throw new Error(`${tool.name} uses a dedicated UI`);
  validateInputs(tool, inputs);
  reportProgress(run, { phase: 'loading', label: 'Preparing files…' });

  const o = coerceOptions(tool, rawOpts);
  const first = inputs[0];
  const base = baseName(first.name);

  switch (tool.id) {
    case 'merge': {
      reportProgress(run, { phase: 'working', label: 'Merging PDFs in a local worker…' });
      const out = await withPdfWorker(run, async (w, onProgress) => w.merge(await Promise.all(inputs.map(toBytes)), onProgress));
      return { files: [{ name: 'merged.pdf', blob: pdfBlob(out) }] };
    }
    case 'split': {
      const bytes = await toBytes(first);
      reportProgress(run, { phase: 'working', label: 'Splitting PDF in a local worker…' });
      const parts = await withPdfWorker(run, async (w, onProgress) =>
        o.mode === 'ranges' ? w.splitRanges(bytes, String(o.ranges ?? ''), onProgress) : w.splitEach(bytes, onProgress),
      );
      const files = parts.map((p, i) => ({ name: `${base}-${i + 1}.pdf`, blob: pdfBlob(p) }));
      return zipFiles(files, `${base}-split.zip`, run);
    }
    case 'extract-pages':
      reportProgress(run, { phase: 'working', label: 'Extracting pages in a local worker…' });
      {
        const out = await withPdfWorker(run, async (w, onProgress) => w.extractPages(await toBytes(first), String(o.pages ?? ''), onProgress));
        return { files: [{ name: `${base}-extracted.pdf`, blob: pdfBlob(out) }] };
      }
    case 'remove-pages':
      reportProgress(run, { phase: 'working', label: 'Removing pages in a local worker…' });
      {
        const out = await withPdfWorker(run, async (w, onProgress) => w.removePages(await toBytes(first), String(o.pages ?? ''), onProgress));
        return { files: [{ name: `${base}-trimmed.pdf`, blob: pdfBlob(out) }] };
      }
    case 'reorder':
      reportProgress(run, { phase: 'working', label: 'Reordering pages in a local worker…' });
      {
        const out = await withPdfWorker(run, async (w, onProgress) => w.reorder(await toBytes(first), String(o.order ?? ''), onProgress));
        return { files: [{ name: `${base}-reordered.pdf`, blob: pdfBlob(out) }] };
      }
    case 'rotate':
      reportProgress(run, { phase: 'working', label: 'Rotating PDF in a local worker…' });
      {
        const bytes = await toBytes(first);
        const pages = String(o.pages ?? '').trim();
        if (!pages) {
          reportProgress(run, { phase: 'working', label: 'Loading Rust rotator…' });
          const { wasmRotateAll } = await import('./wasm/core.js');
          reportProgress(run, { phase: 'working', label: 'Rotating every page locally…' });
          const out = await wasmRotateAll(bytes, Number(o.angle));
          return { files: [{ name: `${base}-rotated.pdf`, blob: pdfBlob(out) }] };
        }
        const out = await withPdfWorker(run, async (w, onProgress) => w.rotate(bytes, Number(o.angle), pages, onProgress));
        return { files: [{ name: `${base}-rotated.pdf`, blob: pdfBlob(out) }] };
      }
    case 'n-up':
      reportProgress(run, { phase: 'working', label: 'Laying out pages in a local worker…' });
      {
        const out = await withPdfWorker(run, async (w, onProgress) => w.nUp(await toBytes(first), Number(o.perSheet), onProgress));
        return { files: [{ name: `${base}-${o.perSheet}up.pdf`, blob: pdfBlob(out) }] };
      }
    case 'booklet':
      reportProgress(run, { phase: 'working', label: 'Imposing booklet spreads in a local worker…' });
      {
        const pageSize = o.pageSize === 'letter' ? 'letter' : 'a4';
        const binding = o.binding === 'right' ? 'right' : 'left';
        const out = await withPdfWorker(run, async (w, onProgress) => w.booklet(await toBytes(first), { pageSize, binding, margin: o.margin }, onProgress));
        return { files: [{ name: `${base}-booklet.pdf`, blob: pdfBlob(out) }] };
      }
    case 'compare-pdf': {
      if (inputs.length !== 2) throw new Error('Compare PDFs needs exactly two PDF files');
      const text = await comparePdfs(await toBytes(inputs[0]), await toBytes(inputs[1]), { scale: o.scale, threshold: o.threshold }, run);
      return { files: [{ name: `${base}-comparison.txt`, blob: new Blob([text], { type: 'text/plain' }) }], text };
    }

    case 'images-to-pdf': {
      const images = await Promise.all(
        inputs.map(async (f) => ({ bytes: await toBytes(f), type: (f.type.includes('png') ? 'png' : 'jpg') as 'png' | 'jpg' })),
      );
      reportProgress(run, { phase: 'working', label: 'Building PDF from images in a local worker…' });
      const out = await withPdfWorker(run, async (w, onProgress) => w.imagesToPdf(images, { pageSize: o.pageSize, margin: o.margin }, onProgress));
      return { files: [{ name: 'images.pdf', blob: pdfBlob(out) }] };
    }
    case 'pdf-to-jpg': {
      const pages = await renderToImages(await toBytes(first), { scale: o.scale, type: 'image/jpeg', quality: o.quality }, run);
      return zipFiles(pages, `${base}-jpg.zip`, run);
    }
    case 'pdf-to-png': {
      const pages = await renderToImages(await toBytes(first), { scale: o.scale, type: 'image/png' }, run);
      return zipFiles(pages, `${base}-png.zip`, run);
    }
    case 'pdf-to-text': {
      const text = await extractText(await toBytes(first), run);
      return { files: [{ name: `${base}.txt`, blob: new Blob([text], { type: 'text/plain' }) }], text };
    }
    case 'pdf-to-epub': {
      const bytes = await toBytes(first);
      const title = String(o.title ?? '').trim() || base;
      const author = String(o.author ?? '').trim();
      const language = String(o.language ?? 'en').trim() || 'en';
      const metadata = { title, author, language };

      if (o.mode === 'fixed') {
        const pages = await renderToImages(bytes, {
          scale: numberOption(o.imageScale, 1.5, 1, 3),
          type: 'image/jpeg',
          quality: numberOption(o.imageQuality, 0.82, 0.4, 0.95),
        }, run);
        reportProgress(run, { phase: 'saving', label: 'Writing fixed-layout EPUB…' });
        const out = await fixedPagesToEpub(pages.map((page, index) => ({ ...page, pageNumber: index + 1 })), metadata);
        return { files: [{ name: `${base}-fixed.epub`, blob: epubBlob(out) }] };
      }

      const pages = await extractTextPages(bytes, run);
      reportProgress(run, { phase: 'saving', label: 'Writing reflowable EPUB…' });
      const out = await textPagesToEpub(pages, {
        ...metadata,
        removeHeadersFooters: o.removeHeadersFooters,
        unwrapParagraphs: o.unwrapParagraphs,
        repairHyphenation: o.repairHyphenation,
      });
      return { files: [{ name: `${base}.epub`, blob: epubBlob(out) }] };
    }
    case 'pdf-to-docx': {
      const pages = await extractTextPages(await toBytes(first), run);
      reportProgress(run, { phase: 'saving', label: 'Writing Word document…' });
      const out = await textPagesToDocx(pages);
      return { files: [{ name: `${base}-extracted.docx`, blob: officeBlob(out, OFFICE_MIME.docx) }] };
    }
    case 'pdf-to-excel': {
      const pages = await extractTextPages(await toBytes(first), run);
      reportProgress(run, { phase: 'saving', label: 'Writing Excel workbook…' });
      const out = await textPagesToXlsx(pages);
      return { files: [{ name: `${base}-extracted.xlsx`, blob: officeBlob(out, OFFICE_MIME.xlsx) }] };
    }
    case 'pdf-to-pptx': {
      const pages = await extractTextPages(await toBytes(first), run);
      reportProgress(run, { phase: 'saving', label: 'Writing PowerPoint deck…' });
      const out = await textPagesToPptx(pages);
      return { files: [{ name: `${base}-extracted.pptx`, blob: officeBlob(out, OFFICE_MIME.pptx) }] };
    }
    case 'ocr-pdf': {
      const out = await ocrSearchablePdf(await toBytes(first), {
        scale: typeof o.scale === 'number' ? o.scale : undefined,
        minConfidence: typeof o.minConfidence === 'number' ? o.minConfidence : undefined,
        model: 'eng',
      }, run);
      return { files: [{ name: `${base}-ocr.pdf`, blob: pdfBlob(out.pdf) }], text: out.text };
    }
    case 'pdfa': {
      const out = await exportPdfA(await toBytes(first), {
        scale: typeof o.scale === 'number' ? o.scale : undefined,
      }, run);
      return { files: [{ name: `${base}-pdfa.pdf`, blob: pdfBlob(out) }] };
    }
    case 'image-convert': {
      reportProgress(run, { phase: 'working', label: 'Converting image…' });
      const format = (o.format as ImageMime) ?? 'image/webp';
      const blob = await convertImage(first, format, o.quality);
      return { files: [{ name: `${base}.${extensionFor(format)}`, blob }] };
    }
    case 'html-to-pdf': {
      reportProgress(run, { phase: 'working', label: 'Building PDF from text…' });
      const source = await first.text();
      const pageSize = o.pageSize === 'letter' ? 'letter' : 'a4';
      const title = String(o.title ?? '').trim() || undefined;
      const out = await textToPdf(source, first.name, { title, pageSize, fontSize: o.fontSize });
      return { files: [{ name: `${base}.pdf`, blob: pdfBlob(out) }] };
    }

    case 'page-numbers':
      {
        reportProgress(run, { phase: 'working', label: 'Adding page numbers in a local worker…' });
        const out = await withPdfWorker(run, async (w, onProgress) => w.pageNumbers(await toBytes(first), { format: o.format, position: o.position, fontSize: o.fontSize, margin: o.margin }, onProgress));
        return { files: [{ name: `${base}-numbered.pdf`, blob: pdfBlob(out) }] };
      }
    case 'bates':
      {
        reportProgress(run, { phase: 'working', label: 'Adding Bates numbers in a local worker…' });
        const prefix = String(o.prefix ?? '');
        const out = await withPdfWorker(run, async (w, onProgress) => w.pageNumbers(await toBytes(first), {
          format: `${prefix}{n}`,
          position: o.position,
          fontSize: o.fontSize,
          margin: o.margin,
          startAt: o.startAt,
          padTo: o.padTo,
        }, onProgress));
        return { files: [{ name: `${base}-bates.pdf`, blob: pdfBlob(out) }] };
      }
    case 'watermark':
      {
        reportProgress(run, { phase: 'working', label: 'Adding watermark in a local worker…' });
        const out = await withPdfWorker(run, async (w, onProgress) => w.watermark(await toBytes(first), { text: o.text, fontSize: o.fontSize, opacity: o.opacity, angle: o.angle }, onProgress));
        return { files: [{ name: `${base}-watermarked.pdf`, blob: pdfBlob(out) }] };
      }
    case 'crop':
      {
        reportProgress(run, { phase: 'working', label: 'Cropping PDF in a local worker…' });
        const out = await withPdfWorker(run, async (w, onProgress) => w.crop(await toBytes(first), { top: o.top, right: o.right, bottom: o.bottom, left: o.left }, onProgress));
        return { files: [{ name: `${base}-cropped.pdf`, blob: pdfBlob(out) }] };
      }
    case 'auto-crop':
      {
        const out = await autoCropPdf(await toBytes(first), { scale: o.scale, tolerance: o.tolerance, padding: o.padding }, run);
        return { files: [{ name: `${base}-autocropped.pdf`, blob: pdfBlob(out) }] };
      }
    case 'metadata':
      {
        const out = await withPdfWorker(run, async (w) => w.setMetadata(await toBytes(first), { title: o.title, author: o.author, subject: o.subject, keywords: o.keywords, creator: o.creator }));
        return { files: [{ name: `${base}.pdf`, blob: pdfBlob(out) }] };
      }

    case 'optimize': {
      // Rust→WASM engine; lazy-loaded so the wasm only downloads for this tool.
      reportProgress(run, { phase: 'optimizing', label: 'Loading Rust optimizer…' });
      const { wasmOptimize } = await import('./wasm/core.js');
      reportProgress(run, { phase: 'optimizing', label: 'Optimizing PDF locally…' });
      return { files: [{ name: `${base}-optimized.pdf`, blob: pdfBlob(await wasmOptimize(await toBytes(first))) }] };
    }
    case 'compress':
      return { files: [{ name: `${base}-compressed.pdf`, blob: pdfBlob(await compressPdf(await toBytes(first), { scale: o.scale, quality: o.quality }, run)) }] };
    case 'compress-image': {
      reportProgress(run, { phase: 'working', label: 'Compressing image…' });
      const format = (o.format as ImageMime) ?? 'image/webp';
      const blob = await compressImage(first, { format, quality: o.quality, maxDimension: Number(o.maxDimension) || undefined });
      return { files: [{ name: `${base}-compressed.${extensionFor(format)}`, blob }] };
    }

    case 'flatten':
      {
        const out = await withPdfWorker(run, async (w) => w.flatten(await toBytes(first)));
        return { files: [{ name: `${base}-flattened.pdf`, blob: pdfBlob(out) }] };
      }
    case 'remove-metadata':
      {
        const out = await withPdfWorker(run, async (w) => w.stripMetadata(await toBytes(first)));
        return { files: [{ name: `${base}-clean.pdf`, blob: pdfBlob(out) }] };
      }
    case 'sanitize':
      {
        reportProgress(run, { phase: 'working', label: 'Sanitizing PDF in a local worker…' });
        const out = await withPdfWorker(run, async (w, onProgress) => w.sanitize(await toBytes(first), { removeAnnotations: o.removeAnnotations, removeForms: o.removeForms }, onProgress));
        return { files: [{ name: `${base}-sanitized.pdf`, blob: pdfBlob(out) }] };
      }
    case 'protect':
      {
        reportProgress(run, { phase: 'working', label: 'Protecting PDF in a local worker…' });
        const out = await withPdfWorker(run, async (w, onProgress) => w.protect(await toBytes(first), {
          userPassword: o.userPassword,
          ownerPassword: o.ownerPassword,
          allowPrinting: o.allowPrinting,
          allowCopying: o.allowCopying,
          allowModifying: o.allowModifying,
        }, onProgress));
        return { files: [{ name: `${base}-protected.pdf`, blob: pdfBlob(out) }] };
      }
    case 'unlock':
      {
        reportProgress(run, { phase: 'working', label: 'Unlocking PDF in a local worker…' });
        const out = await withPdfWorker(run, async (w, onProgress) => w.unlock(await toBytes(first), String(o.password ?? ''), onProgress));
        return { files: [{ name: `${base}-unlocked.pdf`, blob: pdfBlob(out) }] };
      }

    default:
      throw new Error(`Unknown tool: ${tool.id}`);
  }
}
