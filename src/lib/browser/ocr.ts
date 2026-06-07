import { PDFDocument, StandardFonts, rgb } from '@cantoo/pdf-lib';
import { OCRClient, supportsFastBuild, type TextItem } from 'tesseract-wasm';
import tesseractWorkerUrl from '../../../node_modules/tesseract-wasm/dist/tesseract-worker.js?url';
import tesseractCoreUrl from '../../../node_modules/tesseract-wasm/dist/tesseract-core.wasm?url';
import tesseractFallbackCoreUrl from '../../../node_modules/tesseract-wasm/dist/tesseract-core-fallback.wasm?url';
import { abortError, reportProgress, throwIfAborted, type RunOptions } from '../progress.js';
import { pdfjsLib } from './pdfjs.js';

export interface OcrPdfOptions {
  scale?: number;
  minConfidence?: number;
  model?: 'eng';
}

export interface OcrPdfResult {
  pdf: Uint8Array;
  text: string;
}

let wasmBinaryPromise: Promise<ArrayBuffer> | null = null;
const modelUrlByName: Record<NonNullable<OcrPdfOptions['model']>, string> = {
  eng: '/ocr/eng.traineddata',
};

function wasmUrl(): string {
  return supportsFastBuild() ? tesseractCoreUrl : tesseractFallbackCoreUrl;
}

async function loadWasmBinary(): Promise<ArrayBuffer> {
  wasmBinaryPromise ??= fetch(wasmUrl()).then((response) => {
    if (!response.ok) throw new Error('OCR engine failed to load');
    return response.arrayBuffer();
  });
  return wasmBinaryPromise;
}

function safePdfText(text: string): string {
  return text
    .normalize('NFKD')
    .replace(/[^\t\n\r -~]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

function lineText(items: TextItem[]): string {
  const words = items
    .map((item) => ({ ...item, text: safePdfText(item.text) }))
    .filter((item) => item.text.length > 0)
    .sort((a, b) => {
      const ay = (a.rect.top + a.rect.bottom) / 2;
      const by = (b.rect.top + b.rect.bottom) / 2;
      return Math.abs(ay - by) > 8 ? ay - by : a.rect.left - b.rect.left;
    });
  const lines: TextItem[][] = [];
  for (const word of words) {
    const last = lines.at(-1);
    const h = word.rect.bottom - word.rect.top;
    const center = (word.rect.top + word.rect.bottom) / 2;
    const lastCenter = last ? (last[0].rect.top + last[0].rect.bottom) / 2 : 0;
    if (!last || Math.abs(center - lastCenter) > Math.max(8, h * 0.7)) lines.push([word]);
    else last.push(word);
  }
  return lines
    .map((line) => line.sort((a, b) => a.rect.left - b.rect.left).map((item) => item.text).join(' '))
    .filter(Boolean)
    .join('\n');
}

function drawInvisibleTextLayer(
  page: ReturnType<PDFDocument['getPage']>,
  font: Awaited<ReturnType<PDFDocument['embedFont']>>,
  boxes: TextItem[],
  canvasWidth: number,
  canvasHeight: number,
  minConfidence: number,
) {
  const pageWidth = page.getWidth();
  const pageHeight = page.getHeight();
  const scaleX = pageWidth / canvasWidth;
  const scaleY = pageHeight / canvasHeight;

  for (const box of boxes) {
    if (box.confidence < minConfidence) continue;
    const text = safePdfText(box.text);
    if (!text) continue;

    const w = Math.max(1, (box.rect.right - box.rect.left) * scaleX);
    const h = Math.max(1, (box.rect.bottom - box.rect.top) * scaleY);
    let fontSize = Math.max(2, h * 0.92);
    const measured = font.widthOfTextAtSize(text, fontSize);
    if (measured > w && measured > 0) fontSize = Math.max(2, fontSize * (w / measured));

    page.drawText(text, {
      x: box.rect.left * scaleX,
      y: pageHeight - box.rect.bottom * scaleY + Math.max(0, (h - fontSize) * 0.25),
      size: fontSize,
      font,
      color: rgb(0, 0, 0),
      opacity: 0,
    });
  }
}

function createOcrClient(wasmBinary: ArrayBuffer): OCRClient {
  return new OCRClient({
    wasmBinary,
    workerURL: tesseractWorkerUrl,
    createWorker: (url) => new Worker(url, { name: 'unfleece-ocr' }),
  });
}

export async function ocrSearchablePdf(bytes: Uint8Array, opts: OcrPdfOptions = {}, run: RunOptions = {}): Promise<OcrPdfResult> {
  const { scale = 2.5, minConfidence = 0.35, model = 'eng' } = opts;
  reportProgress(run, { phase: 'loading', label: 'Loading OCR engine…' });
  const [wasmBinary, doc, out] = await Promise.all([
    loadWasmBinary(),
    pdfjsLib.getDocument({ data: bytes }).promise,
    PDFDocument.load(bytes, { ignoreEncryption: true }),
  ]);
  const font = await out.embedFont(StandardFonts.Helvetica);
  const ocr = createOcrClient(wasmBinary);
  const onAbort = () => {
    void ocr.destroy();
  };
  run.signal?.addEventListener('abort', onAbort, { once: true });

  const pageTexts: string[] = [];
  try {
    throwIfAborted(run.signal);
    reportProgress(run, { phase: 'loading', label: 'Loading English OCR model…' });
    await ocr.loadModel(modelUrlByName[model]);

    for (let i = 1; i <= doc.numPages; i++) {
      throwIfAborted(run.signal);
      reportProgress(run, { phase: 'rendering', label: `Rendering page ${i} of ${doc.numPages} for OCR…`, current: i - 1, total: doc.numPages });
      const srcPage = await doc.getPage(i);
      const viewport = srcPage.getViewport({ scale });
      const canvas = document.createElement('canvas');
      canvas.width = Math.ceil(viewport.width);
      canvas.height = Math.ceil(viewport.height);
      const ctx = canvas.getContext('2d', { willReadFrequently: true });
      if (!ctx) throw new Error('Canvas not supported');
      ctx.fillStyle = '#ffffff';
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      const task = srcPage.render({ canvas, canvasContext: ctx, viewport });
      const cancelRender = () => task.cancel();
      run.signal?.addEventListener('abort', cancelRender, { once: true });
      try {
        await task.promise;
      } catch (error) {
        if (run.signal?.aborted) throw abortError();
        throw error;
      } finally {
        run.signal?.removeEventListener('abort', cancelRender);
      }
      throwIfAborted(run.signal);

      reportProgress(run, { phase: 'extracting', label: `Recognizing text on page ${i} of ${doc.numPages}…`, current: i - 1, total: doc.numPages });
      const image = ctx.getImageData(0, 0, canvas.width, canvas.height);
      await ocr.loadImage(image);
      const boxes = await ocr.getTextBoxes('word', (progress) => {
        reportProgress(run, {
          phase: 'extracting',
          label: `Recognizing text on page ${i} of ${doc.numPages}… ${Math.round(progress * 100)}%`,
          current: i - 1 + progress,
          total: doc.numPages,
        });
      });
      throwIfAborted(run.signal);
      await ocr.clearImage();

      const accepted = boxes.filter((box) => box.confidence >= minConfidence);
      drawInvisibleTextLayer(out.getPage(i - 1), font, accepted, canvas.width, canvas.height, minConfidence);
      pageTexts.push(lineText(accepted));

      canvas.width = 0;
      canvas.height = 0;
      srcPage.cleanup();
      reportProgress(run, { phase: 'extracting', label: `Finished OCR page ${i} of ${doc.numPages}.`, current: i, total: doc.numPages });
    }
  } catch (error) {
    if (run.signal?.aborted) throw abortError();
    throw error;
  } finally {
    run.signal?.removeEventListener('abort', onAbort);
    await Promise.allSettled([ocr.destroy(), doc.cleanup()]);
  }

  reportProgress(run, { phase: 'saving', label: 'Writing searchable PDF…' });
  const pdf = await out.save();
  return { pdf, text: pageTexts.join('\n\n').trim() };
}
