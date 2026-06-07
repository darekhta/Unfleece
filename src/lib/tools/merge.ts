import { PDFDocument } from '@cantoo/pdf-lib';
import { loadPdf } from '../util/pdf.js';
import { notifyProgress, type ProgressCallback } from '../progress.js';

/**
 * Merge multiple PDFs into one, in the given order. Pure: bytes in, bytes out.
 * Runs fully client-side.
 */
export async function mergePdfs(files: Uint8Array[], onProgress?: ProgressCallback): Promise<Uint8Array> {
  if (files.length === 0) throw new Error('No files to merge');

  const out = await PDFDocument.create();
  for (const [i, bytes] of files.entries()) {
    notifyProgress(onProgress, { phase: 'working', label: `Merging file ${i + 1} of ${files.length}…`, current: i, total: files.length });
    const src = await loadPdf(bytes);
    const pages = await out.copyPages(src, src.getPageIndices());
    for (const page of pages) out.addPage(page);
    notifyProgress(onProgress, { phase: 'working', label: `Merged file ${i + 1} of ${files.length}.`, current: i + 1, total: files.length });
  }
  return out.save();
}
