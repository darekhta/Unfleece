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

function asciiBytes(text: string): Uint8Array {
  return Uint8Array.from(text, (c) => c.charCodeAt(0));
}

/** A minimal single-page PDF with caller-controlled raw page content. */
export function makePdfWithRawContent(content: string, size: [number, number] = [200, 200]): Uint8Array {
  const stream = `${content}\n`;
  const streamLength = asciiBytes(stream).length;
  const objects = [
    '1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n',
    '2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n',
    `3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${size[0]} ${size[1]}] /Contents 4 0 R >>\nendobj\n`,
    `4 0 obj\n<< /Length ${streamLength} >>\nstream\n${stream}endstream\nendobj\n`,
  ];

  let pdf = '%PDF-1.7\n';
  const offsets: number[] = [];
  for (const object of objects) {
    offsets.push(pdf.length);
    pdf += object;
  }

  const xrefOffset = pdf.length;
  pdf += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  pdf += offsets.map((offset) => `${String(offset).padStart(10, '0')} 00000 n \n`).join('');
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;
  return asciiBytes(pdf);
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
