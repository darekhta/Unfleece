import { PDFDocument } from '@cantoo/pdf-lib';

/**
 * Load a PDF for editing. `ignoreEncryption` lets us open documents that have
 * an owner password but no user password (the common "restricted permissions"
 * case) so the user can still operate on their own files.
 */
export async function loadPdf(bytes: Uint8Array): Promise<PDFDocument> {
  return PDFDocument.load(bytes, { ignoreEncryption: true });
}

/** Standard page sizes in PDF points (1/72 inch). */
export const PAGE_SIZES = {
  a4: [595.28, 841.89] as [number, number],
  letter: [612, 792] as [number, number],
} satisfies Record<string, [number, number]>;
