import { PDFDocument, StandardFonts, rgb } from '@cantoo/pdf-lib';

/** A multi-page sample PDF with distinct pages. */
export async function makeSamplePdf(pageCount = 3, size: [number, number] = [300, 400]): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  for (let i = 0; i < pageCount; i++) {
    const page = doc.addPage(size);
    page.drawText(`Page ${i + 1}`, { x: 40, y: size[1] - 60, size: 24, font, color: rgb(0, 0, 0) });
  }
  return doc.save();
}

/** A PDF with an AcroForm: one text field and one checkbox. */
export async function makeFormPdf(): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  const page = doc.addPage([300, 400]);
  const form = doc.getForm();
  form.createTextField('fullName').addToPage(page, { x: 40, y: 300, width: 200, height: 20 });
  form.createCheckBox('agree').addToPage(page, { x: 40, y: 250, width: 15, height: 15 });
  const plan = form.createDropdown('plan');
  plan.addOptions(['Free', 'Pro', 'Team']);
  plan.select('Free');
  plan.addToPage(page, { x: 40, y: 210, width: 120, height: 22 });
  return doc.save();
}

/** Re-read a PDF and return its page count. */
export async function pageCount(bytes: Uint8Array): Promise<number> {
  const doc = await PDFDocument.load(bytes, { ignoreEncryption: true });
  return doc.getPageCount();
}

export async function load(bytes: Uint8Array): Promise<PDFDocument> {
  return PDFDocument.load(bytes, { ignoreEncryption: true });
}

/** A minimal valid 1x1 PNG. */
export const PNG_1x1: Uint8Array = Uint8Array.from(
  atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII='),
  (c) => c.charCodeAt(0),
);

/** Assert bytes look like a PDF (header + non-trivial size). */
export function isPdf(bytes: Uint8Array): boolean {
  const header = String.fromCharCode(...bytes.slice(0, 5));
  return header === '%PDF-' && bytes.length > 100;
}
