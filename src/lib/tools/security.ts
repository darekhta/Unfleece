import { PDFDocument } from '@cantoo/pdf-lib';
import { notifyProgress, type ProgressCallback } from '../progress.js';
import { wasmSanitize } from '../wasm/core.js';

export interface SanitizeOptions {
  removeAnnotations?: boolean;
  removeForms?: boolean;
}

export interface ProtectOptions {
  userPassword?: string;
  ownerPassword?: string;
  allowPrinting?: boolean;
  allowCopying?: boolean;
  allowModifying?: boolean;
}

async function hasEncryptDictionary(bytes: Uint8Array): Promise<boolean> {
  try {
    const doc = await PDFDocument.load(bytes, { ignoreEncryption: true, updateMetadata: false });
    return doc.context.trailerInfo.Encrypt !== undefined;
  } catch {
    // Let the Rust core surface its normal parser error for malformed PDFs.
    return false;
  }
}

/**
 * Strip common active and hidden PDF structures before sharing: metadata,
 * scripts, embedded files, page/document actions and (optionally) annotations
 * and forms. Executed by the Rust→WASM core (lopdf).
 *
 * Encrypted PDFs are rejected up front: the core cannot decrypt them, and
 * sanitizing without decrypting would silently corrupt the page content.
 */
export async function sanitizePdf(bytes: Uint8Array, options: SanitizeOptions = {}, onProgress?: ProgressCallback): Promise<Uint8Array> {
  if (await hasEncryptDictionary(bytes)) {
    throw new Error('This PDF is password-protected. Unlock it first, then sanitize the unlocked copy.');
  }
  notifyProgress(onProgress, { phase: 'working', label: 'Sanitizing PDF in Rust core…', current: 0, total: 1 });
  const out = await wasmSanitize(bytes, {
    removeAnnotations: options.removeAnnotations !== false,
    removeForms: options.removeForms !== false,
  });
  notifyProgress(onProgress, { phase: 'saving', label: 'Saving sanitized PDF…', current: 1, total: 1 });
  return out;
}

export async function protectPdf(bytes: Uint8Array, options: ProtectOptions, onProgress?: ProgressCallback): Promise<Uint8Array> {
  const userPassword = String(options.userPassword ?? '').trim();
  if (!userPassword) throw new Error('Enter a password to protect this PDF');
  const ownerPassword = String(options.ownerPassword ?? '').trim() || userPassword;

  notifyProgress(onProgress, { phase: 'loading', label: 'Opening PDF…' });
  const doc = await PDFDocument.load(bytes, { ignoreEncryption: true });
  notifyProgress(onProgress, { phase: 'working', label: 'Encrypting PDF…' });
  doc.encrypt({
    userPassword,
    ownerPassword,
    permissions: {
      printing: options.allowPrinting ? 'highResolution' : false,
      copying: Boolean(options.allowCopying),
      contentAccessibility: Boolean(options.allowCopying),
      modifying: Boolean(options.allowModifying),
      annotating: Boolean(options.allowModifying),
      fillingForms: Boolean(options.allowModifying),
      documentAssembly: Boolean(options.allowModifying),
    },
  });
  notifyProgress(onProgress, { phase: 'saving', label: 'Saving protected PDF…' });
  return doc.save({ rewrite: true });
}

export async function unlockPdf(bytes: Uint8Array, password: string, onProgress?: ProgressCallback): Promise<Uint8Array> {
  const cleanPassword = String(password ?? '').trim();
  if (!cleanPassword) throw new Error('Enter the password for this PDF');

  notifyProgress(onProgress, { phase: 'loading', label: 'Opening encrypted PDF…' });
  const doc = await PDFDocument.load(bytes, { password: cleanPassword, updateMetadata: false });
  notifyProgress(onProgress, { phase: 'saving', label: 'Saving unlocked PDF…' });
  return doc.save({ rewrite: true });
}
