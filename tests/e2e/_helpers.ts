import { PDFDocument, StandardFonts, rgb } from '@cantoo/pdf-lib';

/** Generate a sample PDF as a Buffer for Playwright file inputs. */
export async function samplePdfBuffer(pages = 3, text?: string): Promise<Buffer> {
  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  for (let i = 0; i < pages; i++) {
    const p = doc.addPage([300, 400]);
    p.drawText(text ?? `Page ${i + 1}`, { x: 40, y: 340, size: 18, font, color: rgb(0, 0, 0) });
  }
  return Buffer.from(await doc.save());
}

export async function protectedPdfBuffer(password = 'secret'): Promise<Buffer> {
  const doc = await PDFDocument.load(await samplePdfBuffer(1, 'Protected content'));
  doc.encrypt({ userPassword: password, ownerPassword: password, permissions: { printing: false, copying: false } });
  return Buffer.from(await doc.save({ rewrite: true }));
}

/** A minimal valid 1x1 PNG. */
export const PNG_1x1 = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=',
  'base64',
);

export function tiff1x1Buffer(): Buffer {
  const ifdOffset = 8;
  const entryCount = 10;
  const bitsOffset = ifdOffset + 2 + entryCount * 12 + 4;
  const pixelOffset = bitsOffset + 6;
  const out = Buffer.alloc(pixelOffset + 3);
  out.write('II', 0, 'ascii');
  out.writeUInt16LE(42, 2);
  out.writeUInt32LE(ifdOffset, 4);
  out.writeUInt16LE(entryCount, ifdOffset);

  let p = ifdOffset + 2;
  const entry = (tag: number, type: number, count: number, value: number) => {
    out.writeUInt16LE(tag, p);
    out.writeUInt16LE(type, p + 2);
    out.writeUInt32LE(count, p + 4);
    if (type === 3 && count === 1) out.writeUInt16LE(value, p + 8);
    else out.writeUInt32LE(value, p + 8);
    p += 12;
  };
  entry(256, 4, 1, 1); // ImageWidth
  entry(257, 4, 1, 1); // ImageLength
  entry(258, 3, 3, bitsOffset); // BitsPerSample
  entry(259, 3, 1, 1); // Compression: none
  entry(262, 3, 1, 2); // PhotometricInterpretation: RGB
  entry(273, 4, 1, pixelOffset); // StripOffsets
  entry(277, 3, 1, 3); // SamplesPerPixel
  entry(278, 4, 1, 1); // RowsPerStrip
  entry(279, 4, 1, 3); // StripByteCounts
  entry(284, 3, 1, 1); // PlanarConfiguration: chunky
  out.writeUInt32LE(0, p);
  out.writeUInt16LE(8, bitsOffset);
  out.writeUInt16LE(8, bitsOffset + 2);
  out.writeUInt16LE(8, bitsOffset + 4);
  out[pixelOffset] = 255;
  out[pixelOffset + 1] = 0;
  out[pixelOffset + 2] = 0;
  return out;
}

export const pdfFile = (name: string, buffer: Buffer) => ({ name, mimeType: 'application/pdf', buffer });
export const pngFile = (name: string, buffer: Buffer) => ({ name, mimeType: 'image/png', buffer });
export const tiffFile = (name: string, buffer: Buffer) => ({ name, mimeType: 'image/tiff', buffer });
export const textFile = (name: string, text: string, mimeType = 'text/plain') => ({ name, mimeType, buffer: Buffer.from(text) });
