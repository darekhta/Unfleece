import { notifyProgress, type ProgressCallback } from '../progress.js';
import { wasmMergePdfs } from '../wasm/core.js';

/**
 * Merge multiple PDFs into one, in the given order. Pure: bytes in, bytes out.
 * Runs fully client-side in the Rust core (lopdf), which materializes inherited
 * page attributes and drops document-level structures (outlines, forms, name
 * trees) exactly like pdf-lib's copyPages did.
 */
export async function mergePdfs(files: Uint8Array[], onProgress?: ProgressCallback): Promise<Uint8Array> {
  if (files.length === 0) throw new Error('No files to merge');
  notifyProgress(onProgress, { phase: 'working', label: `Merging ${files.length} file${files.length === 1 ? '' : 's'} in Rust core…`, current: 0, total: files.length });
  const out = await wasmMergePdfs(files);
  notifyProgress(onProgress, { phase: 'working', label: `Merged ${files.length} file${files.length === 1 ? '' : 's'}.`, current: files.length, total: files.length });
  return out;
}
