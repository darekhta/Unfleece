// "Compress" defaults to Ghostscript/ghostpdl-wasm `pdfwrite`, then keeps the
// older raster path as an explicit fallback for scanned PDFs or runtime errors.
import loadGhostscript, { type GhostscriptModule } from '@okathira/ghostpdl-wasm';
import ghostscriptWasmUrl from '@okathira/ghostpdl-wasm/gs.wasm?url';
import { pdfjsLib } from './pdfjs.js';
import { abortError, reportProgress, throwIfAborted, type RunOptions } from '../progress.js';
import { wasmImagePagesToPdf, type WasmImagePage } from '../wasm/core.js';

export interface CompressOptions {
  engine?: 'ghostscript' | 'raster';
  preset?: 'screen' | 'ebook' | 'printer' | 'prepress' | 'default';
  scale?: number;
  quality?: number;
}

let ghostscriptReady: Promise<GhostscriptModule> | null = null;
let jobSeq = 0;

function loadGhostscriptModule(): Promise<GhostscriptModule> {
  const ready = ghostscriptReady ?? loadGhostscript({
      locateFile: (file: string) => (file.endsWith('.wasm') ? ghostscriptWasmUrl : file),
      noExitRuntime: true,
      print: () => {},
      printErr: () => {},
    } as Parameters<typeof loadGhostscript>[0]);
  ghostscriptReady = ready;
  return ready;
}

function unlinkQuietly(module: GhostscriptModule, path: string) {
  try {
    module.FS.unlink(path);
  } catch {
    // Emscripten FS throws for missing files; cleanup should be best-effort.
  }
}

function presetForOptions(opts: CompressOptions): NonNullable<CompressOptions['preset']> {
  if (opts.preset) return opts.preset;
  const quality = typeof opts.quality === 'number' ? opts.quality : 0.6;
  if (quality <= 0.35) return 'screen';
  if (quality <= 0.7) return 'ebook';
  if (quality <= 0.85) return 'printer';
  return 'prepress';
}

async function ghostscriptCompressPdf(bytes: Uint8Array, opts: CompressOptions = {}, run: RunOptions = {}): Promise<Uint8Array> {
  const preset = presetForOptions(opts);
  reportProgress(run, { phase: 'loading', label: 'Loading Ghostscript WASM…' });
  const module = await loadGhostscriptModule();
  throwIfAborted(run.signal);

  const id = ++jobSeq;
  const input = `/unfleece-${id}.pdf`;
  const output = `/unfleece-${id}-compressed.pdf`;

  unlinkQuietly(module, input);
  unlinkQuietly(module, output);
  module.FS.writeFile(input, bytes);

  reportProgress(run, { phase: 'compressing', label: `Compressing with Ghostscript /${preset}…` });
  const status = module.callMain([
    '-q',
    '-dSAFER',
    '-dBATCH',
    '-dNOPAUSE',
    '-sDEVICE=pdfwrite',
    '-dCompatibilityLevel=1.7',
    `-dPDFSETTINGS=/${preset}`,
    '-dDetectDuplicateImages=true',
    '-dCompressFonts=true',
    '-dSubsetFonts=true',
    '-dAutoRotatePages=/None',
    `-sOutputFile=${output}`,
    input,
  ]);
  throwIfAborted(run.signal);

  try {
    if (status !== 0) throw new Error(`Ghostscript exited with status ${status}`);
    const out = module.FS.readFile(output, { encoding: 'binary' });
    if (!out?.length || String.fromCharCode(...out.slice(0, 5)) !== '%PDF-') {
      throw new Error('Ghostscript did not produce a valid PDF');
    }
    return out;
  } finally {
    unlinkQuietly(module, input);
    unlinkQuietly(module, output);
  }
}

async function rasterCompressPdf(bytes: Uint8Array, opts: CompressOptions = {}, run: RunOptions = {}): Promise<Uint8Array> {
  const { scale = 1.5, quality = 0.6 } = opts;
  reportProgress(run, { phase: 'loading', label: 'Opening PDF…' });
  const src = await pdfjsLib.getDocument({ data: bytes }).promise;
  const imagePages: WasmImagePage[] = [];

  try {
    for (let i = 1; i <= src.numPages; i++) {
      reportProgress(run, { phase: 'compressing', label: `Rasterizing page ${i} of ${src.numPages}…`, current: i - 1, total: src.numPages });
      const page = await src.getPage(i);
      const pts = page.getViewport({ scale: 1 });
      const viewport = page.getViewport({ scale });

      const canvas = document.createElement('canvas');
      canvas.width = Math.ceil(viewport.width);
      canvas.height = Math.ceil(viewport.height);
      const ctx = canvas.getContext('2d');
      if (!ctx) throw new Error('Canvas not supported');
      ctx.fillStyle = '#ffffff';
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      const task = page.render({ canvas, canvasContext: ctx, viewport });
      const onAbort = () => task.cancel();
      run.signal?.addEventListener('abort', onAbort, { once: true });
      try {
        await task.promise;
      } catch (e) {
        if (run.signal?.aborted) throw abortError();
        throw e;
      } finally {
        run.signal?.removeEventListener('abort', onAbort);
      }
      throwIfAborted(run.signal);

      const blob = await new Promise<Blob>((resolve, reject) =>
        canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('toBlob failed'))), 'image/jpeg', quality),
      );
      const jpeg = new Uint8Array(await blob.arrayBuffer());
      imagePages.push({ widthPt: pts.width, heightPt: pts.height, bytes: jpeg, type: 'jpg' });

      canvas.width = 0;
      canvas.height = 0;
      page.cleanup();
      reportProgress(run, { phase: 'compressing', label: `Compressed page ${i} of ${src.numPages}.`, current: i, total: src.numPages });
    }
  } finally {
    await src.cleanup();
  }
  reportProgress(run, { phase: 'compressing', label: 'Writing compressed PDF…' });
  return wasmImagePagesToPdf(imagePages);
}

export async function compressPdf(bytes: Uint8Array, opts: CompressOptions = {}, run: RunOptions = {}): Promise<Uint8Array> {
  if (opts.engine === 'raster') return rasterCompressPdf(bytes, opts, run);
  try {
    return await ghostscriptCompressPdf(bytes, opts, run);
  } catch (error) {
    if (run.signal?.aborted) throw abortError();
    reportProgress(run, { phase: 'compressing', label: 'Ghostscript failed; using raster fallback…' });
    return rasterCompressPdf(bytes, opts, run);
  }
}
