// Content-blind error beacon.
//
// THE DELIBERATE DECISION (see docs/08-risks.md + the About privacy section): Unfleece
// processes files entirely on-device, so production bugs are otherwise invisible
// until a user reports them (cf. the WebKit IndexedDB-Blob loss and the OCR
// detached-buffer bug). We therefore report errors — and ONLY errors — with a
// strictly allowlisted, content-free payload:
//
//   { tool, errorClass, engine, browser, locale, version }
//
// It can NEVER carry file names, file bytes, error messages, stack traces, URLs,
// cookies, or any per-user identifier. It honors Do-Not-Track and a one-click
// opt-out, sends at most one fire-and-forget `navigator.sendBeacon`, and fails
// silent. No success pings, no analytics, no fingerprinting.
import { errorClass, type ErrorClass } from './errors.js';

/** Which engine layer was running when the error happened. */
export type EngineLayer = 'rust' | 'pdfjs' | 'canvas' | 'ghostscript' | 'tesseract' | 'unknown';

const OPT_OUT_KEY = 'uf-telemetry';
const ENDPOINT = '/api/err';
const APP_VERSION = '1';

/** True unless the user opted out or the browser signals Do-Not-Track. */
export function telemetryEnabled(): boolean {
  if (typeof navigator !== 'undefined') {
    const dnt = navigator.doNotTrack ?? (globalThis as { doNotTrack?: string }).doNotTrack;
    if (dnt === '1' || dnt === 'yes') return false;
  }
  try {
    return localStorage.getItem(OPT_OUT_KEY) !== 'off';
  } catch {
    return false; // no storage access → don't send
  }
}

export function setTelemetry(on: boolean): void {
  try {
    localStorage.setItem(OPT_OUT_KEY, on ? 'on' : 'off');
  } catch {
    /* ignore */
  }
}

/** Coarse browser family — a category, not a fingerprint. */
function browserFamily(): string {
  if (typeof navigator === 'undefined') return 'other';
  const ua = navigator.userAgent;
  if (/Edg\//.test(ua)) return 'edge';
  if (/Firefox\//.test(ua)) return 'firefox';
  if (/Chrome\//.test(ua)) return 'chrome';
  if (/Safari\//.test(ua) && /Version\//.test(ua)) return 'safari';
  return 'other';
}

function baseLocale(): string {
  if (typeof document !== 'undefined') {
    const l = document.documentElement.lang;
    if (l) return l.split('-')[0];
  }
  return 'en';
}

/**
 * Report that `tool` failed with a given error, in `engine`, content-blind.
 * Pass either an `ErrorClass` token or a raw error. Raw errors are classified
 * locally; only the fixed class token is transmitted. Safe to call from any
 * catch block; never throws.
 */
export function reportError(tool: string, error: unknown, engine: EngineLayer = 'unknown'): void {
  try {
    if (!telemetryEnabled()) return;
    if (typeof navigator === 'undefined' || typeof navigator.sendBeacon !== 'function') return;
    const cls: ErrorClass = typeof error === 'string' && isErrorClass(error) ? error : errorClass(error);
    if (cls === 'cancelled') return; // a user cancelling is not a bug — don't report it
    const payload = {
      t: String(tool).slice(0, 40),
      e: cls,
      g: engine,
      b: browserFamily(),
      l: baseLocale(),
      v: APP_VERSION,
    };
    navigator.sendBeacon(ENDPOINT, new Blob([JSON.stringify(payload)], { type: 'application/json' }));
  } catch {
    /* never let telemetry break a tool */
  }
}

const CLASSES = new Set<ErrorClass>([
  'cancelled',
  'password',
  'invalid-pdf',
  'oom',
  'render',
  'selection',
  'unsupported',
  'other',
]);
function isErrorClass(v: string): v is ErrorClass {
  return CLASSES.has(v as ErrorClass);
}

/** The engine layer that backs a given tool id (for the beacon's `engine`). */
export function engineForTool(toolId: string): EngineLayer {
  if (toolId === 'compress') return 'ghostscript';
  if (toolId === 'ocr-pdf') return 'tesseract';
  if (toolId === 'image-convert' || toolId === 'compress-image') return 'canvas';
  // Anything that rasterizes or reads a page visually runs through pdf.js.
  const pdfjs = new Set([
    'pdf-to-jpg', 'pdf-to-png', 'pdf-to-text', 'pdf-to-epub', 'pdf-to-docx', 'pdf-to-excel',
    'pdf-to-pptx', 'pdfa', 'compare-pdf', 'auto-crop', 'redact',
  ]);
  if (pdfjs.has(toolId)) return 'pdfjs';
  return 'rust'; // object-graph + generation tools
}
