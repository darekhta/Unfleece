// Canvas/jSquash-based image conversion. Browser-only; covered by Playwright E2E.
import magickWasmUrl from '@imagemagick/magick-wasm/magick.wasm?url';

export type ImageMime = 'image/png' | 'image/jpeg' | 'image/webp' | 'image/avif' | 'image/jxl';

export interface ImageTransformOptions {
  format?: ImageMime;
  quality?: number;
  /** Longest output edge in CSS pixels. 0/undefined keeps the original size. */
  maxDimension?: number;
}

const CODEC_MIMES = new Set<ImageMime>(['image/avif', 'image/jxl']);
const MAGICK_INPUT_MIMES = new Set([
  'image/tiff',
  'image/bmp',
  'image/x-bmp',
  'image/gif',
  'image/x-icon',
  'image/vnd.microsoft.icon',
  'image/vnd.adobe.photoshop',
  'image/x-photoshop',
  'image/heic',
  'image/heif',
  'image/jp2',
]);
const MAGICK_INPUT_EXTENSIONS = [
  '.tif',
  '.tiff',
  '.psd',
  '.psb',
  '.bmp',
  '.gif',
  '.ico',
  '.cur',
  '.heic',
  '.heif',
  '.jp2',
  '.j2k',
  '.tga',
  '.pcx',
];
let magickReady: Promise<typeof import('@imagemagick/magick-wasm')> | null = null;

function mimeFromBlob(input: Blob): ImageMime | undefined {
  if (input.type && ['image/png', 'image/jpeg', 'image/webp', 'image/avif', 'image/jxl'].includes(input.type)) {
    return input.type as ImageMime;
  }
  const name = input instanceof File ? input.name.toLowerCase() : '';
  if (name.endsWith('.avif')) return 'image/avif';
  if (name.endsWith('.jxl')) return 'image/jxl';
  return undefined;
}

function shouldUseMagick(input: Blob): boolean {
  const mime = input.type.toLowerCase();
  const name = input instanceof File ? input.name.toLowerCase() : '';
  return MAGICK_INPUT_MIMES.has(mime) || MAGICK_INPUT_EXTENSIONS.some((ext) => name.endsWith(ext));
}

async function loadMagick(): Promise<typeof import('@imagemagick/magick-wasm')> {
  if (!magickReady) {
    magickReady = (async () => {
      const mod = await import('@imagemagick/magick-wasm');
      await mod.initializeImageMagick(new URL(magickWasmUrl, window.location.href));
      return mod;
    })();
  }
  return magickReady;
}

function targetSize(width: number, height: number, maxDimension = 0): { width: number; height: number } {
  const longest = Math.max(width, height);
  const scale = maxDimension && maxDimension > 0 && longest > maxDimension ? maxDimension / longest : 1;
  return {
    width: Math.max(1, Math.round(width * scale)),
    height: Math.max(1, Math.round(height * scale)),
  };
}

function drawImageData(data: ImageData, width: number, height: number): HTMLCanvasElement {
  const source = document.createElement('canvas');
  source.width = data.width;
  source.height = data.height;
  const sourceCtx = source.getContext('2d');
  if (!sourceCtx) throw new Error('Canvas not supported');
  sourceCtx.putImageData(data, 0, 0);

  if (width === data.width && height === data.height) return source;
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('Canvas not supported');
  ctx.drawImage(source, 0, 0, width, height);
  source.width = 0;
  source.height = 0;
  return canvas;
}

async function inputToCanvas(input: Blob, maxDimension = 0, fillWhite = false): Promise<HTMLCanvasElement> {
  const inputMime = mimeFromBlob(input);
  if (inputMime === 'image/avif') {
    const { decode } = await import('@jsquash/avif');
    const data = await decode(await input.arrayBuffer());
    if (!data) throw new Error('Could not decode this AVIF image');
    const size = targetSize(data.width, data.height, maxDimension);
    const canvas = drawImageData(data, size.width, size.height);
    if (fillWhite) {
      const ctx = canvas.getContext('2d');
      if (!ctx) throw new Error('Canvas not supported');
      ctx.globalCompositeOperation = 'destination-over';
      ctx.fillStyle = '#fff';
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      ctx.globalCompositeOperation = 'source-over';
    }
    return canvas;
  }
  if (inputMime === 'image/jxl') {
    const { decode } = await import('@jsquash/jxl');
    const data = await decode(await input.arrayBuffer());
    const size = targetSize(data.width, data.height, maxDimension);
    const canvas = drawImageData(data, size.width, size.height);
    if (fillWhite) {
      const ctx = canvas.getContext('2d');
      if (!ctx) throw new Error('Canvas not supported');
      ctx.globalCompositeOperation = 'destination-over';
      ctx.fillStyle = '#fff';
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      ctx.globalCompositeOperation = 'source-over';
    }
    return canvas;
  }

  if (shouldUseMagick(input)) {
    const { ImageMagick, MagickFormat } = await loadMagick();
    const png = await ImageMagick.read(new Uint8Array(await input.arrayBuffer()), async (image) => {
      image.autoOrient();
      image.format = MagickFormat.Png32;
      return image.write(MagickFormat.Png32, (data) => blobFromBytes(data, 'image/png'));
    });
    return inputToCanvas(png, maxDimension, fillWhite);
  }

  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(input);
  } catch (error) {
    const { ImageMagick, MagickFormat } = await loadMagick();
    const png = await ImageMagick.read(new Uint8Array(await input.arrayBuffer()), async (image) => {
      image.autoOrient();
      image.format = MagickFormat.Png32;
      return image.write(MagickFormat.Png32, (data) => blobFromBytes(data, 'image/png'));
    });
    return inputToCanvas(png, maxDimension, fillWhite);
  }
  const size = targetSize(bitmap.width, bitmap.height, maxDimension);
  const canvas = document.createElement('canvas');
  canvas.width = size.width;
  canvas.height = size.height;
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('Canvas not supported');
  if (fillWhite) {
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
  }
  ctx.drawImage(bitmap, 0, 0, size.width, size.height);
  bitmap.close();
  return canvas;
}

function blobFromArrayBuffer(buffer: ArrayBuffer, type: ImageMime): Blob {
  return new Blob([buffer], { type });
}

function blobFromBytes(data: Uint8Array, type: string): Blob {
  const bytes = new Uint8Array(data.byteLength);
  bytes.set(data);
  return new Blob([bytes.buffer], { type });
}

async function encodeWithCodec(canvas: HTMLCanvasElement, format: ImageMime, quality: number): Promise<Blob> {
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('Canvas not supported');
  const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
  const q = Math.round(Math.min(Math.max(quality, 0.01), 1) * 100);
  if (format === 'image/avif') {
    const { encode } = await import('@jsquash/avif');
    return blobFromArrayBuffer(await encode(data, { quality: q, qualityAlpha: q, speed: 6 }), format);
  }
  if (format === 'image/jxl') {
    const { encode } = await import('@jsquash/jxl');
    return blobFromArrayBuffer(await encode(data, { quality: q, effort: 7 }), format);
  }
  throw new Error(`Unsupported codec: ${format}`);
}

async function renderImageToBlob(
  input: Blob,
  { format = 'image/webp', quality = 0.85, maxDimension = 0 }: ImageTransformOptions = {},
): Promise<Blob> {
  const canvas = await inputToCanvas(input, maxDimension, format === 'image/jpeg');
  if (CODEC_MIMES.has(format)) {
    const blob = await encodeWithCodec(canvas, format, quality);
    canvas.width = 0;
    canvas.height = 0;
    return blob;
  }

  const blob = await new Promise<Blob>((resolve, reject) =>
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('Conversion failed'))), format, quality),
  );
  canvas.width = 0;
  canvas.height = 0;
  return blob;
}

/** Convert an image Blob to another format/quality using the Canvas API. */
export async function convertImage(
  input: Blob,
  format: ImageMime = 'image/webp',
  quality = 0.85,
): Promise<Blob> {
  return renderImageToBlob(input, { format, quality });
}

/** Re-encode and optionally downscale an image for smaller output. */
export async function compressImage(input: Blob, opts: ImageTransformOptions = {}): Promise<Blob> {
  return renderImageToBlob(input, opts);
}

export function extensionFor(mime: ImageMime): string {
  return mime === 'image/png'
    ? 'png'
    : mime === 'image/jpeg'
      ? 'jpg'
      : mime === 'image/avif'
        ? 'avif'
        : mime === 'image/jxl'
          ? 'jxl'
          : 'webp';
}
