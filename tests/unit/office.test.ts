import { describe, expect, it } from 'vitest';
import JSZip from 'jszip';
import { textPagesToDocx, textPagesToPptx, textPagesToXlsx, type ExtractedTextPage } from '@lib/tools/office.js';

const pages: ExtractedTextPage[] = [
  {
    pageNumber: 1,
    text: 'Name  Amount\nAda  42',
    items: [
      { text: 'Name', x: 40, y: 300, width: 35, height: 12 },
      { text: 'Amount', x: 150, y: 300, width: 50, height: 12 },
      { text: 'Ada', x: 40, y: 280, width: 24, height: 12 },
      { text: '42', x: 150, y: 280, width: 12, height: 12 },
    ],
  },
];

describe('office generation', () => {
  it('generates a DOCX containing extracted text', async () => {
    const out = await textPagesToDocx(pages);
    const zip = await JSZip.loadAsync(out);
    const documentXml = await zip.file('word/document.xml')?.async('string');
    const stylesXml = await zip.file('word/styles.xml')?.async('string');

    expect(zip.file('[Content_Types].xml')).toBeTruthy();
    expect(stylesXml).toContain('ExtractedHeading');
    expect(documentXml).toContain('Page 1');
    expect(documentXml).toContain('Name  Amount');
    expect(documentXml).toContain('Ada  42');
  });

  it('generates an XLSX with page, row and inferred columns', async () => {
    const out = await textPagesToXlsx(pages);
    const zip = await JSZip.loadAsync(out);
    const sheetXml = await zip.file('xl/worksheets/sheet1.xml')?.async('string');
    const stylesXml = await zip.file('xl/styles.xml')?.async('string');

    expect(zip.file('xl/workbook.xml')).toBeTruthy();
    expect(stylesXml).toContain('FFE8FFF5');
    expect(sheetXml).toContain('<cols>');
    expect(sheetXml).toContain('Column 1');
    expect(sheetXml).toContain('Amount');
    expect(sheetXml).toContain('42');
  });

  it('generates a PPTX with one slide per page', async () => {
    const out = await textPagesToPptx(pages);
    const zip = await JSZip.loadAsync(out);
    const presentationXml = await zip.file('ppt/presentation.xml')?.async('string');
    const slideXml = await zip.file('ppt/slides/slide1.xml')?.async('string');

    expect(zip.file('[Content_Types].xml')).toBeTruthy();
    expect(presentationXml).toContain('<p:sldIdLst>');
    expect(slideXml).toContain('Page 1');
    expect(slideXml).toContain('b="1"');
    expect(slideXml).toContain('Name  Amount');
  });

  it('keeps empty extraction honest', async () => {
    const out = await textPagesToDocx([{ pageNumber: 1, text: '', items: [] }]);
    const zip = await JSZip.loadAsync(out);
    const documentXml = await zip.file('word/document.xml')?.async('string');

    expect(documentXml).toContain('No selectable text found');
  });
});
