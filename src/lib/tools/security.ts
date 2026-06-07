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

// "/Encrypt" as raw bytes. The trailer (or xref-stream dictionary) that
// references the encryption dictionary can never live inside a compressed
// stream, so a raw byte scan reliably detects encrypted documents.
const ENCRYPT_MARKER = [0x2f, 0x45, 0x6e, 0x63, 0x72, 0x79, 0x70, 0x74];

function hasEncryptDictionary(bytes: Uint8Array): boolean {
  // Scan backward: trailers live at the end of the file.
  outer: for (let i = bytes.length - ENCRYPT_MARKER.length; i >= 0; i--) {
    for (let j = 0; j < ENCRYPT_MARKER.length; j++) {
      if (bytes[i + j] !== ENCRYPT_MARKER[j]) continue outer;
    }
    return true;
  }
  return false;
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
  if (hasEncryptDictionary(bytes)) {
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
