import { loadPdf } from '../util/pdf.js';
import type { ImageType } from './imagesToPdf.js';

export interface StampOptions {
  image: Uint8Array;
  imageType: ImageType;
  /** 0-based page index; defaults to the last page. */
  pageIndex?: number;
  /** Position in points from the bottom-left of the page. */
  x: number;
  y: number;
  width: number;
  height?: number;
  opacity?: number;
}

/**
 * Stamp an image (e.g. a drawn/typed signature) onto a page. This is an
 * *electronic* signature (a visual mark) — it carries no cryptographic proof.
 */
export async function stampImage(bytes: Uint8Array, opts: StampOptions): Promise<Uint8Array> {
  const doc = await loadPdf(bytes);
  const pages = doc.getPages();
  const idx = opts.pageIndex ?? pages.length - 1;
  const page = pages[idx];
  if (!page) throw new Error(`Page index ${idx} out of range`);

  const embedded =
    opts.imageType === 'png' ? await doc.embedPng(opts.image) : await doc.embedJpg(opts.image);

  const aspect = embedded.height / embedded.width;
  const width = opts.width;
  const height = opts.height ?? width * aspect;

  page.drawImage(embedded, {
    x: opts.x,
    y: opts.y,
    width,
    height,
    opacity: opts.opacity ?? 1,
  });
  return doc.save();
}

export interface StampPlacement {
  image: Uint8Array;
  imageType: ImageType;
  /** 0-based page index. */
  pageIndex: number;
  /** Position in points from the bottom-left of the page. */
  x: number;
  y: number;
  width: number;
  height?: number;
  opacity?: number;
}

/**
 * Stamp several images in one pass (multi-page signing). The document is loaded
 * once and identical image buffers are embedded only once.
 */
export async function stampImageMany(bytes: Uint8Array, stamps: StampPlacement[]): Promise<Uint8Array> {
  if (stamps.length === 0) throw new Error('No signatures placed');
  const doc = await loadPdf(bytes);
  const pages = doc.getPages();
  const cache = new Map<Uint8Array, Awaited<ReturnType<typeof doc.embedPng>>>();

  for (const s of stamps) {
    const page = pages[s.pageIndex];
    if (!page) throw new Error(`Page index ${s.pageIndex} out of range`);
    let embedded = cache.get(s.image);
    if (!embedded) {
      embedded = s.imageType === 'png' ? await doc.embedPng(s.image) : await doc.embedJpg(s.image);
      cache.set(s.image, embedded);
    }
    const aspect = embedded.height / embedded.width;
    const width = s.width;
    const height = s.height ?? width * aspect;
    page.drawImage(embedded, { x: s.x, y: s.y, width, height, opacity: s.opacity ?? 1 });
  }
  return doc.save();
}
