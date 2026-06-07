import { describe, expect, it, vi } from 'vitest';
import { PDFArray, PDFDict, PDFDocument, PDFName, StandardFonts, rgb } from '@cantoo/pdf-lib';
import { protectPdf, sanitizePdf, unlockPdf } from '@lib/tools/security.js';
import { isPdf, makeSamplePdf, pageCount } from './_fixtures.js';

const name = (key: string) => PDFName.of(key);

async function makeRiskyPdf(): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  doc.setTitle('Secret Title');
  doc.setAuthor('Secret Author');
  doc.addJavaScript('open-script', 'app.alert("bad")');
  await doc.attach(new Uint8Array([1, 2, 3]), 'secret.txt', { mimeType: 'text/plain' });

  const font = await doc.embedFont(StandardFonts.Helvetica);
  const page = doc.addPage([300, 400]);
  page.drawText('Visible content', { x: 40, y: 340, size: 18, font, color: rgb(0, 0, 0) });
  page.node.set(name('AA'), doc.context.obj({ O: { S: 'JavaScript', JS: 'bad' } }));
  page.node.set(PDFName.Annots, doc.context.obj([{ A: { S: 'JavaScript', JS: 'bad' } }]));

  const form = doc.getForm();
  form.createTextField('hiddenField').addToPage(page, { x: 40, y: 280, width: 180, height: 20 });
  return doc.save();
}

describe('sanitizePdf', () => {
  it('strips scripts, attachments, metadata, annotations and forms', async () => {
    const onProgress = vi.fn();
    const out = await sanitizePdf(await makeRiskyPdf(), {}, onProgress);
    const clean = await PDFDocument.load(out, { ignoreEncryption: true, updateMetadata: false });

    expect(isPdf(out)).toBe(true);
    expect(clean.getAttachments()).toHaveLength(0);
    expect(clean.context.lookupMaybe(clean.context.trailerInfo.Info, PDFDict)).toBeUndefined();
    expect(clean.catalog.lookupMaybe(name('AcroForm'), PDFDict)).toBeUndefined();
    expect(clean.getPage(0).node.lookupMaybe(name('AA'), PDFDict)).toBeUndefined();
    expect(clean.getPage(0).node.lookupMaybe(PDFName.Annots, PDFArray)).toBeUndefined();

    const names = clean.catalog.lookupMaybe(name('Names'), PDFDict);
    expect(names?.lookupMaybe(name('JavaScript'), PDFDict)).toBeUndefined();
    expect(names?.lookupMaybe(name('EmbeddedFiles'), PDFDict)).toBeUndefined();
    expect(onProgress).toHaveBeenCalledWith(expect.objectContaining({ phase: 'saving' }));
  });
});

describe('protectPdf and unlockPdf', () => {
  it('encrypts a PDF so it requires a password', async () => {
    const protectedPdf = await protectPdf(await makeSamplePdf(1), { userPassword: 'secret', allowPrinting: true });

    expect(isPdf(protectedPdf)).toBe(true);
    await expect(PDFDocument.load(protectedPdf)).rejects.toThrow(/encrypted/i);
    expect(await pageCount(await unlockPdf(protectedPdf, 'secret'))).toBe(1);
  });

  it('rejects unlock attempts with the wrong password', async () => {
    const protectedPdf = await protectPdf(await makeSamplePdf(1), { userPassword: 'secret' });

    await expect(unlockPdf(protectedPdf, 'wrong')).rejects.toThrow();
  });
});
