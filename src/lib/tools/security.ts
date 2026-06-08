import { notifyProgress, type ProgressCallback } from '../progress.js';
import { wasmSanitize, wasmIsEncrypted, wasmProtect, wasmUnlock } from '../wasm/core.js';

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
    return await wasmIsEncrypted(bytes);
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
 * Encrypted PDFs are rejected up front: sanitizing without decrypting would
 * silently corrupt the page content — unlock first.
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

/** Encrypt with AESV2 Standard Security via the Rust core (RustCrypto). */
export async function protectPdf(bytes: Uint8Array, options: ProtectOptions, onProgress?: ProgressCallback): Promise<Uint8Array> {
  const userPassword = String(options.userPassword ?? '').trim();
  if (!userPassword) throw new Error('Enter a password to protect this PDF');
  const ownerPassword = String(options.ownerPassword ?? '').trim() || userPassword;

  notifyProgress(onProgress, { phase: 'working', label: 'Encrypting PDF in Rust core…', current: 0, total: 1 });
  const out = await wasmProtect(bytes, {
    userPassword,
    ownerPassword,
    allowPrinting: options.allowPrinting !== false,
    allowCopying: Boolean(options.allowCopying),
    allowModifying: Boolean(options.allowModifying),
  });
  notifyProgress(onProgress, { phase: 'saving', label: 'Saving protected PDF…', current: 1, total: 1 });
  return out;
}

/** Decrypt with a user or owner password via the Rust core. */
export async function unlockPdf(bytes: Uint8Array, password: string, onProgress?: ProgressCallback): Promise<Uint8Array> {
  const cleanPassword = String(password ?? '').trim();
  if (!cleanPassword) throw new Error('Enter the password for this PDF');

  notifyProgress(onProgress, { phase: 'working', label: 'Decrypting PDF in Rust core…', current: 0, total: 1 });
  const out = await wasmUnlock(bytes, cleanPassword);
  notifyProgress(onProgress, { phase: 'saving', label: 'Saving unlocked PDF…', current: 1, total: 1 });
  return out;
}
