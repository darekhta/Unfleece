import JSZip from 'jszip';

export interface ExtractedTextItem {
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ExtractedTextPage {
  pageNumber: number;
  text: string;
  items: ExtractedTextItem[];
}

const DOCX_MIME = 'application/vnd.openxmlformats-officedocument.wordprocessingml.document';
const XLSX_MIME = 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet';
const PPTX_MIME = 'application/vnd.openxmlformats-officedocument.presentationml.presentation';

export const OFFICE_MIME = {
  docx: DOCX_MIME,
  xlsx: XLSX_MIME,
  pptx: PPTX_MIME,
} as const;

function xmlEscape(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

function docxText(text: string): string {
  const escaped = xmlEscape(text);
  return escaped.trim() === escaped ? `<w:t>${escaped}</w:t>` : `<w:t xml:space="preserve">${escaped}</w:t>`;
}

function docxParagraph(text: string, style?: string): string {
  const pPr = style ? `<w:pPr><w:pStyle w:val="${style}"/></w:pPr>` : '';
  return `<w:p>${pPr}<w:r>${docxText(text)}</w:r></w:p>`;
}

function contentTypes(overrides: string): string {
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
${overrides}
</Types>`;
}

function packageRels(type: string, target: string): string {
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="${type}" Target="${target}"/>
</Relationships>`;
}

function docxDocumentRels(): string {
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>`;
}

function docxStylesXml(): string {
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/><w:pPr><w:spacing w:after="120"/></w:pPr><w:rPr><w:sz w:val="22"/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="ExtractedHeading"><w:name w:val="Extracted page heading"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:spacing w:before="160" w:after="120"/></w:pPr><w:rPr><w:b/><w:color w:val="0E1320"/><w:sz w:val="28"/></w:rPr></w:style>
</w:styles>`;
}

/** Generate a simple DOCX from selectable PDF text, one paragraph per extracted line. */
export async function textPagesToDocx(pages: ExtractedTextPage[]): Promise<Uint8Array> {
  const body: string[] = [];
  const nonEmpty = pages.some((page) => page.text.trim());

  if (!nonEmpty) {
    body.push(docxParagraph('No selectable text found. This PDF may be scanned; OCR is not available in this tool yet.'));
  } else {
    for (const [index, page] of pages.entries()) {
      const lines = page.text.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
      body.push(docxParagraph(`Page ${page.pageNumber}`, 'ExtractedHeading'));
      for (const line of lines) body.push(docxParagraph(line));
      if (index < pages.length - 1) body.push('<w:p><w:r><w:br w:type="page"/></w:r></w:p>');
    }
  }

  const documentXml = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    ${body.join('\n    ')}
    <w:sectPr>
      <w:pgSz w:w="12240" w:h="15840"/>
      <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/>
    </w:sectPr>
  </w:body>
</w:document>`;

  const zip = new JSZip();
  zip.file('[Content_Types].xml', contentTypes(`  <Override PartName="/word/document.xml" ContentType="${DOCX_MIME}.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>`));
  zip.file('_rels/.rels', packageRels('http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument', 'word/document.xml'));
  zip.file('word/document.xml', documentXml);
  zip.file('word/_rels/document.xml.rels', docxDocumentRels());
  zip.file('word/styles.xml', docxStylesXml());
  return zip.generateAsync({ type: 'uint8array', compression: 'DEFLATE' });
}

interface TableRow {
  pageNumber: number;
  rowNumber: number;
  cells: string[];
}

function splitLine(line: string): string[] {
  const tabbed = line.split(/\t+/).map((cell) => cell.trim()).filter(Boolean);
  if (tabbed.length > 1) return tabbed;
  const spaced = line.split(/\s{2,}/).map((cell) => cell.trim()).filter(Boolean);
  return spaced.length > 1 ? spaced : [line.trim()];
}

function rowsFromItems(page: ExtractedTextPage): string[][] {
  if (page.items.length === 0) {
    return page.text.split(/\r?\n/).map(splitLine).filter((row) => row.some(Boolean));
  }

  const sorted = [...page.items]
    .filter((item) => item.text.trim())
    .sort((a, b) => (Math.abs(b.y - a.y) > 3 ? b.y - a.y : a.x - b.x));

  const lineGroups: ExtractedTextItem[][] = [];
  for (const item of sorted) {
    const last = lineGroups.at(-1);
    if (!last || Math.abs(last[0].y - item.y) > Math.max(3, item.height * 0.6)) {
      lineGroups.push([item]);
    } else {
      last.push(item);
    }
  }

  return lineGroups.map((line) => {
    const cells: string[] = [];
    let current = '';
    let previous: ExtractedTextItem | undefined;
    for (const item of line.sort((a, b) => a.x - b.x)) {
      const gap = previous ? item.x - (previous.x + previous.width) : 0;
      if (previous && gap > Math.max(18, previous.height * 1.8)) {
        if (current.trim()) cells.push(current.trim());
        current = item.text;
      } else {
        current += current ? ` ${item.text}` : item.text;
      }
      previous = item;
    }
    if (current.trim()) cells.push(current.trim());
    return cells.length > 1 ? cells : splitLine(cells[0] ?? '');
  }).filter((row) => row.some(Boolean));
}

function sheetRows(pages: ExtractedTextPage[]): string[][] {
  const rows: TableRow[] = [];
  for (const page of pages) {
    const pageRows = rowsFromItems(page);
    pageRows.forEach((cells, index) => rows.push({ pageNumber: page.pageNumber, rowNumber: index + 1, cells }));
  }

  if (rows.length === 0) {
    return [['Note'], ['No selectable text found. This PDF may be scanned; OCR is not available in this tool yet.']];
  }

  const maxCells = Math.max(...rows.map((row) => row.cells.length));
  const header = ['Page', 'Row', ...Array.from({ length: maxCells }, (_, index) => `Column ${index + 1}`)];
  return [
    header,
    ...rows.map((row) => [
      String(row.pageNumber),
      String(row.rowNumber),
      ...row.cells,
      ...Array.from({ length: maxCells - row.cells.length }, () => ''),
    ]),
  ];
}

function columnName(index: number): string {
  let n = index + 1;
  let out = '';
  while (n > 0) {
    const r = (n - 1) % 26;
    out = String.fromCharCode(65 + r) + out;
    n = Math.floor((n - 1) / 26);
  }
  return out;
}

function worksheetXml(rows: string[][]): string {
  const xmlRows = rows.map((row, rIndex) => {
    const cells = row.map((value, cIndex) => {
      const ref = `${columnName(cIndex)}${rIndex + 1}`;
      const style = rIndex === 0 ? ' s="1"' : '';
      return `<c r="${ref}"${style} t="inlineStr"><is><t>${xmlEscape(value)}</t></is></c>`;
    }).join('');
    return `<row r="${rIndex + 1}">${cells}</row>`;
  }).join('');

  const maxCols = Math.max(1, ...rows.map((row) => row.length));
  const cols = Array.from({ length: maxCols }, (_, index) => {
    const width = index < 2 ? 10 : 24;
    return `<col min="${index + 1}" max="${index + 1}" width="${width}" customWidth="1"/>`;
  }).join('');

  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <cols>${cols}</cols>
  <sheetData>${xmlRows}</sheetData>
</worksheet>`;
}

function xlsxStylesXml(): string {
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="2"><font><sz val="11"/><name val="Aptos"/></font><font><b/><sz val="11"/><color rgb="FF0E1320"/><name val="Aptos"/></font></fonts>
  <fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FFE8FFF5"/><bgColor indexed="64"/></patternFill></fill></fills>
  <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
  <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
  <cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1"/></cellXfs>
  <cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
</styleSheet>`;
}

/** Generate a best-effort XLSX workbook from selectable PDF text rows. */
export async function textPagesToXlsx(pages: ExtractedTextPage[]): Promise<Uint8Array> {
  const rows = sheetRows(pages);
  const workbookXml = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets><sheet name="Extracted text" sheetId="1" r:id="rId1"/></sheets>
</workbook>`;
  const workbookRels = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>`;

  const zip = new JSZip();
  zip.file('[Content_Types].xml', contentTypes(`  <Override PartName="/xl/workbook.xml" ContentType="${XLSX_MIME}.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>`));
  zip.file('_rels/.rels', packageRels('http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument', 'xl/workbook.xml'));
  zip.file('xl/workbook.xml', workbookXml);
  zip.file('xl/_rels/workbook.xml.rels', workbookRels);
  zip.file('xl/worksheets/sheet1.xml', worksheetXml(rows));
  zip.file('xl/styles.xml', xlsxStylesXml());
  return zip.generateAsync({ type: 'uint8array', compression: 'DEFLATE' });
}

function pptParagraph(text: string, size = 2000, color = '263244', bold = false): string {
  const b = bold ? ' b="1"' : '';
  return `<a:p><a:r><a:rPr lang="en-US" sz="${size}"${b}><a:solidFill><a:srgbClr val="${color}"/></a:solidFill></a:rPr><a:t>${xmlEscape(text)}</a:t></a:r></a:p>`;
}

function pptShape(id: number, name: string, x: number, y: number, cx: number, cy: number, paragraphs: string[]): string {
  return `<p:sp>
  <p:nvSpPr><p:cNvPr id="${id}" name="${xmlEscape(name)}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="${x}" y="${y}"/><a:ext cx="${cx}" cy="${cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr>
  <p:txBody><a:bodyPr wrap="square"/><a:lstStyle/>${paragraphs.join('')}</p:txBody>
</p:sp>`;
}

function slideXml(page: ExtractedTextPage): string {
  const title = `Page ${page.pageNumber}`;
  const lines = page.text.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  const bodyLines = lines.length > 0
    ? lines.slice(0, 16)
    : ['No selectable text found. This PDF may be scanned; OCR is not available in this tool yet.'];
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
      ${pptShape(2, 'Title', 457200, 274320, 8229600, 685800, [pptParagraph(title, 3200, '0E1320', true)])}
      ${pptShape(3, 'Extracted text', 685800, 1188720, 7772400, 3474720, bodyLines.map((line) => pptParagraph(line)))}
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sld>`;
}

function slideLayoutXml(): string {
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank" preserve="1">
  <p:cSld name="Blank"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sldLayout>`;
}

function slideMasterXml(): string {
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst>
  <p:txStyles><p:titleStyle/><p:bodyStyle/><p:otherStyle/></p:txStyles>
</p:sldMaster>`;
}

function themeXml(): string {
  return `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Unfleece">
  <a:themeElements>
    <a:clrScheme name="Unfleece"><a:dk1><a:srgbClr val="0E1320"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="263244"/></a:dk2><a:lt2><a:srgbClr val="F5F7FA"/></a:lt2><a:accent1><a:srgbClr val="2ECC8F"/></a:accent1><a:accent2><a:srgbClr val="4F8CFF"/></a:accent2><a:accent3><a:srgbClr val="F5B84B"/></a:accent3><a:accent4><a:srgbClr val="FB6F84"/></a:accent4><a:accent5><a:srgbClr val="7C6CFF"/></a:accent5><a:accent6><a:srgbClr val="20B8A5"/></a:accent6><a:hlink><a:srgbClr val="4F8CFF"/></a:hlink><a:folHlink><a:srgbClr val="7C6CFF"/></a:folHlink></a:clrScheme>
    <a:fontScheme name="Unfleece"><a:majorFont><a:latin typeface="Aptos Display"/></a:majorFont><a:minorFont><a:latin typeface="Aptos"/></a:minorFont></a:fontScheme>
    <a:fmtScheme name="Unfleece"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="9525"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme>
  </a:themeElements>
</a:theme>`;
}

/** Generate a simple PPTX from selectable PDF text, one slide per PDF page. */
export async function textPagesToPptx(pages: ExtractedTextPage[]): Promise<Uint8Array> {
  const deckPages = pages.length > 0 ? pages : [{ pageNumber: 1, text: '', items: [] }];
  const slideOverrides = deckPages
    .map((_, index) => `  <Override PartName="/ppt/slides/slide${index + 1}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>`)
    .join('\n');
  const slideIds = deckPages.map((_, index) => `<p:sldId id="${256 + index}" r:id="rId${index + 1}"/>`).join('');
  const presentationRels = [
    ...deckPages.map((_, index) => `  <Relationship Id="rId${index + 1}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide${index + 1}.xml"/>`),
    `  <Relationship Id="rId${deckPages.length + 1}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>`,
    `  <Relationship Id="rId${deckPages.length + 2}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>`,
  ].join('\n');

  const presentationXml = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId${deckPages.length + 1}"/></p:sldMasterIdLst>
  <p:sldIdLst>${slideIds}</p:sldIdLst>
  <p:sldSz cx="9144000" cy="5143500" type="screen16x9"/>
  <p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>`;

  const zip = new JSZip();
  zip.file('[Content_Types].xml', contentTypes(`  <Override PartName="/ppt/presentation.xml" ContentType="${PPTX_MIME}.main+xml"/>
  <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
  <Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
${slideOverrides}`));
  zip.file('_rels/.rels', packageRels('http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument', 'ppt/presentation.xml'));
  zip.file('ppt/presentation.xml', presentationXml);
  zip.file('ppt/_rels/presentation.xml.rels', `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
${presentationRels}
</Relationships>`);
  zip.file('ppt/slideMasters/slideMaster1.xml', slideMasterXml());
  zip.file('ppt/slideMasters/_rels/slideMaster1.xml.rels', `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>`);
  zip.file('ppt/slideLayouts/slideLayout1.xml', slideLayoutXml());
  zip.file('ppt/slideLayouts/_rels/slideLayout1.xml.rels', `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>`);
  zip.file('ppt/theme/theme1.xml', themeXml());
  deckPages.forEach((page, index) => {
    zip.file(`ppt/slides/slide${index + 1}.xml`, slideXml(page));
    zip.file(`ppt/slides/_rels/slide${index + 1}.xml.rels`, `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>`);
  });
  return zip.generateAsync({ type: 'uint8array', compression: 'DEFLATE' });
}
