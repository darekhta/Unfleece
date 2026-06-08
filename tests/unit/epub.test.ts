import { describe, expect, it } from 'vitest';
import JSZip from 'jszip';
import { EPUB_MIME, fixedPagesToEpub, textPagesToEpub, type FixedLayoutPage } from '@lib/tools/epub.js';
import type { ExtractedTextItem, ExtractedTextPage } from '@lib/tools/office.js';

function localHeaderInfo(bytes: Uint8Array): { compressionMethod: number; name: string } {
  const compressionMethod = bytes[8] | (bytes[9] << 8);
  const nameLength = bytes[26] | (bytes[27] << 8);
  const extraLength = bytes[28] | (bytes[29] << 8);
  const name = new TextDecoder().decode(bytes.slice(30, 30 + nameLength));
  expect(extraLength).toBeGreaterThanOrEqual(0);
  return { compressionMethod, name };
}

function item(text: string, x: number, y: number, width = text.length * 6): ExtractedTextItem {
  return { text, x, y, width, height: 12 };
}

const textPages: ExtractedTextPage[] = [1, 2, 3].map((pageNumber) => ({
  pageNumber,
  width: 600,
  height: 800,
  text: '',
  items: [
    item('Quarterly Report 2026', 40, 760, 140),
    item('Fish & Chips', 72, 700, 74),
    item('exam-', 72, 680, 34),
    item('ple text <tag>', 72, 666, 82),
    item(String(pageNumber), 294, 24, 12),
  ],
}));

describe('EPUB generation', () => {
  it('writes an EPUB 3 package with mimetype stored first and reflowable XHTML', async () => {
    const out = await textPagesToEpub(textPages, {
      title: 'A & B',
      author: 'Ada <Lovelace>',
      language: 'en',
      identifier: 'urn:uuid:test-book',
      modified: '2026-01-02T03:04:05Z',
    });
    const firstHeader = localHeaderInfo(out);
    expect(firstHeader).toEqual({ compressionMethod: 0, name: 'mimetype' });

    const zip = await JSZip.loadAsync(out);
    await expect(zip.file('mimetype')?.async('string')).resolves.toBe(EPUB_MIME);
    await expect(zip.file('META-INF/container.xml')?.async('string')).resolves.toContain('EPUB/package.opf');

    const packageXml = await zip.file('EPUB/package.opf')?.async('string');
    const navXml = await zip.file('EPUB/nav.xhtml')?.async('string');
    const pageXml = await zip.file('EPUB/text/page-001.xhtml')?.async('string');

    expect(packageXml).toContain('<dc:title>A &amp; B</dc:title>');
    expect(packageXml).toContain('<dc:creator>Ada &lt;Lovelace&gt;</dc:creator>');
    expect(packageXml).toContain('<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>');
    expect(packageXml).toContain('<itemref idref="page-001"/>');
    expect(navXml).toContain('<nav epub:type="toc"');
    expect(navXml).toContain('text/page-001.xhtml');
    expect(pageXml).toContain('Fish &amp; Chips example text &lt;tag&gt;');
    expect(pageXml).not.toContain('Quarterly Report');
    expect(pageXml).not.toContain('>1<');
  });

  it('keeps scanned or empty reflowable input valid and honest', async () => {
    const out = await textPagesToEpub([{ pageNumber: 1, text: '', items: [], width: 300, height: 400 }], {
      title: 'Scan',
      identifier: 'urn:uuid:scan',
      modified: '2026-01-02T03:04:05Z',
    });
    const zip = await JSZip.loadAsync(out);
    const pageXml = await zip.file('EPUB/text/page-001.xhtml')?.async('string');
    expect(pageXml).toContain('No selectable text found on this page.');
  });

  it('writes fixed-layout EPUB pages with images and viewport metadata', async () => {
    const image = new Blob([new Uint8Array([0xff, 0xd8, 0xff, 0xd9])], { type: 'image/jpeg' });
    const pages: FixedLayoutPage[] = [{
      name: 'page-001.jpg',
      blob: image,
      pageNumber: 1,
      width: 612,
      height: 792,
      pixelWidth: 918,
      pixelHeight: 1188,
    }];

    const out = await fixedPagesToEpub(pages, {
      title: 'Fixed',
      language: 'uk',
      identifier: 'urn:uuid:fixed',
      modified: '2026-01-02T03:04:05Z',
    });
    const zip = await JSZip.loadAsync(out);
    const packageXml = await zip.file('EPUB/package.opf')?.async('string');
    const pageXml = await zip.file('EPUB/text/page-001.xhtml')?.async('string');

    expect(packageXml).toContain('<meta property="rendition:layout">pre-paginated</meta>');
    expect(packageXml).toContain('<item id="image-001" href="images/page-001.jpg" media-type="image/jpeg"/>');
    expect(pageXml).toContain('<meta name="viewport" content="width=612, height=792"/>');
    expect(pageXml).toContain('<img src="../images/page-001.jpg" alt="Page 1"/>');
    expect(zip.file('EPUB/images/page-001.jpg')).toBeTruthy();
    expect(await zip.file('EPUB/images/page-001.jpg')?.async('uint8array')).toHaveLength(4);
  });
});
