// Cloudflare Pages Function: the content-blind error beacon sink.
//
// This is the project's ONE deliberate server touchpoint (see ADR-016). It never
// receives, stores, or can leak file data — only an allowlisted, fixed-shape
// count of {tool, errorClass, engine, browser, locale}. Anything not on the
// allowlist is dropped. It records one Analytics Engine data point when the
// `ERRORS` binding is configured, and always returns 204 so it can never affect
// the user's flow. No request bodies are logged.
interface Env {
  ERRORS?: AnalyticsEngineDataset;
}
interface AnalyticsEngineDataset {
  writeDataPoint(event: { blobs?: string[]; doubles?: number[]; indexes?: string[] }): void;
}

const TOOLS = new Set([
  'merge', 'split', 'extract-pages', 'remove-pages', 'reorder', 'rotate', 'n-up', 'booklet', 'compare-pdf',
  'images-to-pdf', 'pdf-to-jpg', 'pdf-to-png', 'pdf-to-text', 'pdf-to-epub', 'pdf-to-docx', 'pdf-to-excel',
  'pdf-to-pptx', 'ocr-pdf', 'pdfa', 'image-convert', 'html-to-pdf', 'page-numbers', 'bates', 'watermark',
  'crop', 'auto-crop', 'metadata', 'sign', 'optimize', 'compress', 'compress-image', 'fill-form', 'flatten',
  'remove-metadata', 'protect', 'unlock', 'sanitize', 'redact',
]);
const CLASSES = new Set(['cancelled', 'password', 'invalid-pdf', 'oom', 'render', 'selection', 'unsupported', 'other']);
const ENGINES = new Set(['rust', 'pdfjs', 'canvas', 'ghostscript', 'tesseract', 'unknown']);
const BROWSERS = new Set(['chrome', 'firefox', 'safari', 'edge', 'other']);

const allow = (set: Set<string>, v: unknown, fallback: string) =>
  typeof v === 'string' && set.has(v) ? v : fallback;

export const onRequest: (ctx: { request: Request; env: Env }) => Promise<Response> = async ({ request, env }) => {
  if (request.method !== 'POST') return new Response('Method Not Allowed', { status: 405 });
  try {
    const raw = (await request.json()) as Record<string, unknown>;
    const tool = allow(TOOLS, raw.t, 'other');
    const cls = allow(CLASSES, raw.e, 'other');
    const engine = allow(ENGINES, raw.g, 'unknown');
    const browser = allow(BROWSERS, raw.b, 'other');
    const cleanLocale = typeof raw.l === 'string' ? raw.l.slice(0, 5).replace(/[^a-z-]/gi, '') : '';
    const locale = cleanLocale || 'en';

    env.ERRORS?.writeDataPoint({
      // Only the allowlisted tokens — no message, no content, no identifiers.
      blobs: [tool, cls, engine, browser, locale],
      indexes: [tool],
      doubles: [1],
    });
  } catch {
    /* malformed body — ignore; never log it */
  }
  return new Response(null, { status: 204 });
};
