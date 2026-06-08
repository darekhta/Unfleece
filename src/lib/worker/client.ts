import * as Comlink from 'comlink';
import type { PdfWorkerApi } from './pdf.worker.js';
import type { ProgressCallback } from '../progress.js';

let remote: Comlink.Remote<PdfWorkerApi> | null = null;
let worker: Worker | null = null;

/** Lazily create the singleton PDF worker and return its Comlink-wrapped API. */
export function getPdfWorker(): Comlink.Remote<PdfWorkerApi> {
  if (!remote) {
    worker = new Worker(new URL('./pdf.worker.ts', import.meta.url), {
      type: 'module',
      name: 'unfleece-pdf',
    });
    remote = Comlink.wrap<PdfWorkerApi>(worker);
  }
  return remote;
}

/** Abort the current PDF worker. The next call to `getPdfWorker` creates a fresh one. */
export function terminatePdfWorker(): void {
  worker?.terminate();
  worker = null;
  remote = null;
}

export function createProgressProxy(callback: ProgressCallback): ProgressCallback {
  return Comlink.proxy(callback);
}
