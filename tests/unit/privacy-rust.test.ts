// Privacy tools (metadata read/set/strip + sanitize) driving the REAL
// Rust→WASM core. pdf-lib appears here only to build fixtures and to inspect
// outputs — the code under test no longer uses it.
import { describe, expect, it } from 'vitest';
import { PDFArray, PDFDict, PDFDocument, PDFName, PDFString } from '@cantoo/pdf-lib';
import { getMetadata, setMetadata, stripMetadata } from '@lib/tools/metadata.js';
import { protectPdf, sanitizePdf } from '@lib/tools/security.js';
import { wasmSetMetadata } from '@lib/wasm/core.js';
import { isPdf, load, makeFormPdf, makeSamplePdf, pageCount } from './_fixtures.js';

const asLatin1 = (bytes: Uint8Array) => new TextDecoder('latin1').decode(bytes);

/** A 1-page PDF with a raw (uncompressed) XMP metadata stream. */
async function makeXmpPdf(): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  doc.addPage([200, 200]);
  const xmp =
    '<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>' +
    '<x:xmpmeta xmlns:x="adobe:ns:meta/">creator-tool-secret</x:xmpmeta>' +
    '<?xpacket end="w"?>';
  const stream = doc.context.stream(xmp, { Type: 'Metadata', Subtype: 'XML' });
  doc.catalog.set(PDFName.of('Metadata'), doc.context.register(stream));
  return doc.save({ useObjectStreams: false });
}

describe('metadata (Rust core)', () => {
  it('round-trips a Ukrainian title/author/subject through UTF-16BE', async () => {
    const out = await setMetadata(await makeSamplePdf(1), {
      title: 'Привіт, світе — Київ',
      author: 'Дарій Ткаченко',
      subject: 'Тестовий документ',
    });
    const meta = await getMetadata(out);
    expect(meta.title).toBe('Привіт, світе — Київ');
    expect(meta.author).toBe('Дарій Ткаченко');
    expect(meta.subject).toBe('Тестовий документ');
    expect(meta.pageCount).toBe(1);
  });

  it('splits stored keywords on semicolons as well as commas', async () => {
    const out = await setMetadata(await makeSamplePdf(1), { keywords: ['alpha; beta', 'gamma'] });
    expect((await getMetadata(out)).keywords).toEqual(['alpha', 'beta', 'gamma']);
  });

  it('separators-only keywords read back as a present-but-empty array', async () => {
    const out = await setMetadata(await makeSamplePdf(1), { keywords: [';'] });
    expect((await getMetadata(out)).keywords).toEqual([]);
  });

  it('an empty keywords array removes the keywords entry entirely', async () => {
    const withKeywords = await setMetadata(await makeSamplePdf(1), { keywords: ['a', 'b'] });
    expect((await getMetadata(withKeywords)).keywords).toEqual(['a', 'b']);
    const cleared = await setMetadata(withKeywords, { keywords: [] });
    expect((await getMetadata(cleared)).keywords).toBeUndefined();
  });

  it('an empty string clears a field; other fields stay untouched', async () => {
    const withMeta = await setMetadata(await makeSamplePdf(1), {
      title: 'Secret',
      author: 'Keep Me',
      subject: 'Also Keep',
    });
    const cleared = await setMetadata(withMeta, { title: '' });
    const meta = await getMetadata(cleared);
    expect(meta.title).toBeUndefined();
    expect(meta.author).toBe('Keep Me');
    expect(meta.subject).toBe('Also Keep');
  });

  it('round-trips creation/modification dates as JS Dates', async () => {
    const created = new Date(Date.UTC(2024, 0, 2, 3, 4, 5));
    const modified = new Date(Date.UTC(2026, 5, 7, 12, 0, 0));
    const out = await setMetadata(await makeSamplePdf(1), {
      creationDate: created,
      modificationDate: modified,
    });
    const meta = await getMetadata(out);
    expect(meta.creationDate).toBeInstanceOf(Date);
    expect(meta.creationDate?.getTime()).toBe(created.getTime());
    expect(meta.modificationDate?.getTime()).toBe(modified.getTime());
  });

  it("parses timezone-offset PDF dates (D:…+02'00') to the right UTC instant", async () => {
    const out = await wasmSetMetadata(await makeSamplePdf(1), {
      creationDate: "D:20250607120000+02'00'",
      modificationDate: 'D:20250607120000Z',
    });
    const meta = await getMetadata(out);
    expect(meta.creationDate?.getTime()).toBe(Date.UTC(2025, 5, 7, 10, 0, 0));
    expect(meta.modificationDate?.getTime()).toBe(Date.UTC(2025, 5, 7, 12, 0, 0));
  });

  it('after stripMetadata only pageCount remains', async () => {
    // pdf-lib fixtures carry Producer/Creator/dates by default — strip them.
    const meta = await getMetadata(await stripMetadata(await makeSamplePdf(2)));
    expect(meta.pageCount).toBe(2);
    for (const field of ['title', 'author', 'subject', 'keywords', 'creator', 'producer', 'creationDate', 'modificationDate'] as const) {
      expect(meta[field], field).toBeUndefined();
    }
  });

  it('stripMetadata removes the XMP packet bytes from the file', async () => {
    const pdf = await makeXmpPdf();
    expect(asLatin1(pdf)).toContain('xpacket'); // sanity: fixture really has XMP
    const out = await stripMetadata(pdf);
    expect(isPdf(out)).toBe(true);
    expect(asLatin1(out)).not.toContain('xpacket');
    expect(asLatin1(out)).not.toContain('creator-tool-secret');
  });

  it('stripMetadata removes custom (non-standard) Info keys too', async () => {
    const doc = await PDFDocument.create();
    doc.addPage([200, 200]);
    doc.setTitle('Public Title');
    const info = doc.context.lookup(doc.context.trailerInfo.Info, PDFDict);
    info.set(PDFName.of('CustomTool'), PDFString.of('secret-app-name'));
    const pdf = await doc.save({ useObjectStreams: false });
    expect(asLatin1(pdf)).toContain('secret-app-name');

    const out = await stripMetadata(pdf);
    expect(asLatin1(out)).not.toContain('secret-app-name');
    expect((await getMetadata(out)).title).toBeUndefined();
  });

  it('metadata ops preserve odd page sizes and page count', async () => {
    const out = await setMetadata(await makeSamplePdf(3, [200.5, 333.25]), { title: 'Odd Sizes' });
    expect(await pageCount(out)).toBe(3);
    const size = (await load(out)).getPage(0).getSize();
    expect(size.width).toBeCloseTo(200.5, 2);
    expect(size.height).toBeCloseTo(333.25, 2);
    expect((await getMetadata(out)).title).toBe('Odd Sizes');
  });

  it('getMetadata rejects non-PDF input', async () => {
    await expect(getMetadata(new TextEncoder().encode('not a pdf at all'))).rejects.toThrow();
  });
});

describe('sanitizePdf (Rust core)', () => {
  it('removes every trace of JavaScript from the output bytes', async () => {
    const doc = await PDFDocument.create();
    doc.addPage([200, 200]);
    doc.addJavaScript('boot', 'app.alert("bad")');
    const pdf = await doc.save({ useObjectStreams: false });
    // Sanity: the fixture really is risky. pdf-lib hex-encodes the script
    // body (UTF-16BE), so check for the /JavaScript action name instead.
    expect(asLatin1(pdf)).toContain('/JavaScript');

    const out = await sanitizePdf(pdf);
    expect(isPdf(out)).toBe(true);
    const text = asLatin1(out);
    expect(text).not.toContain('app.alert');
    expect(text).not.toContain('/JavaScript');
  });

  it('removeAnnotations: false keeps the annotation but strips its action', async () => {
    const doc = await PDFDocument.create();
    const page = doc.addPage([300, 300]);
    page.node.set(
      PDFName.of('Annots'),
      doc.context.obj([
        {
          Type: 'Annot',
          Subtype: 'Link',
          Rect: [0, 0, 50, 50],
          A: { S: 'URI', URI: PDFString.of('https://example.com') },
        },
      ]),
    );
    const out = await sanitizePdf(await doc.save(), { removeAnnotations: false });

    const clean = await load(out);
    const annots = clean.getPage(0).node.lookupMaybe(PDFName.Annots, PDFArray);
    expect(annots?.size()).toBe(1);
    const annot = annots?.lookupMaybe(0, PDFDict);
    expect(annot?.lookupMaybe(PDFName.of('Subtype'), PDFName)?.toString()).toBe('/Link');
    expect(annot?.has(PDFName.of('A'))).toBe(false);
  });

  it('removeForms: false keeps the AcroForm that the default pass removes', async () => {
    const pdf = await makeFormPdf();

    const kept = await load(await sanitizePdf(pdf, { removeForms: false }));
    expect(kept.catalog.lookupMaybe(PDFName.of('AcroForm'), PDFDict)).toBeDefined();

    const removed = await load(await sanitizePdf(pdf));
    expect(removed.catalog.lookupMaybe(PDFName.of('AcroForm'), PDFDict)).toBeUndefined();
  });

  it('rejects password-protected PDFs with a friendly message', async () => {
    const locked = await protectPdf(await makeSamplePdf(1), { userPassword: 'secret' });
    await expect(sanitizePdf(locked)).rejects.toThrow(/password-protected.*Unlock/i);
  });

  it('keeps all pages of a multi-page document and reports Rust-core progress', async () => {
    const events: { phase: string; label: string }[] = [];
    const out = await sanitizePdf(await makeSamplePdf(5), {}, (p) => events.push({ phase: p.phase, label: p.label }));
    expect(await pageCount(out)).toBe(5);
    expect(events[0]?.label).toContain('Rust core');
    expect(events.at(-1)?.phase).toBe('saving');
  });

  it('rejects non-PDF input', async () => {
    await expect(sanitizePdf(new Uint8Array([1, 2, 3]))).rejects.toThrow();
  });
});
