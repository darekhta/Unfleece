import JSZip from 'jszip';
import type { RenderedPage } from '../browser/render.js';
import type { ExtractedTextItem, ExtractedTextPage } from './office.js';

export const EPUB_MIME = 'application/epub+zip';

export interface EpubMetadataOptions {
  title?: string;
  author?: string;
  language?: string;
  identifier?: string;
  modified?: string | Date;
}

export interface ReflowableEpubOptions extends EpubMetadataOptions {
  removeHeadersFooters?: boolean;
  unwrapParagraphs?: boolean;
  repairHyphenation?: boolean;
}

export interface FixedLayoutEpubOptions extends EpubMetadataOptions {}

export interface FixedLayoutPage extends RenderedPage {
  pageNumber?: number;
  width: number;
  height: number;
}

interface Metadata {
  title: string;
  author: string;
  language: string;
  identifier: string;
  modified: string;
}

interface EpubContentDoc {
  id: string;
  href: string;
  title: string;
  xml: string;
}

interface EpubImage {
  id: string;
  href: string;
  bytes: ArrayBuffer;
  mediaType: string;
}

interface TextLine {
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
  pageWidth?: number;
  pageHeight?: number;
}

function cleanXmlText(value: string): string {
  return value
    .replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F]/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}

function xmlEscape(value: string): string {
  return cleanXmlText(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

function attributeEscape(value: string): string {
  return xmlEscape(value);
}

function padPage(n: number): string {
  return String(n).padStart(3, '0');
}

function modifiedTimestamp(value?: string | Date): string {
  const date = value instanceof Date ? value : value ? new Date(value) : new Date();
  if (Number.isNaN(date.getTime())) return modifiedTimestamp();
  return date.toISOString().replace(/\.\d{3}Z$/, 'Z');
}

function randomIdentifier(): string {
  const randomUUID = globalThis.crypto?.randomUUID?.bind(globalThis.crypto);
  if (randomUUID) return `urn:uuid:${randomUUID()}`;
  return `urn:uuid:${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
}

function normalizeLanguage(value?: string): string {
  const language = String(value || 'en').trim();
  return /^[A-Za-z]{2,3}(?:-[A-Za-z0-9]{2,8})*$/.test(language) ? language : 'en';
}

function metadataFromOptions(options: EpubMetadataOptions): Metadata {
  return {
    title: cleanXmlText(options.title || 'Converted PDF') || 'Converted PDF',
    author: cleanXmlText(options.author || ''),
    language: normalizeLanguage(options.language),
    identifier: cleanXmlText(options.identifier || randomIdentifier()),
    modified: modifiedTimestamp(options.modified),
  };
}

function median(values: number[]): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0 ? (sorted[mid - 1] + sorted[mid]) / 2 : sorted[mid];
}

function joinLineItems(items: ExtractedTextItem[]): string {
  let out = '';
  let previous: ExtractedTextItem | undefined;
  for (const item of items) {
    const text = item.text.trim();
    if (!text) continue;
    const gap = previous ? item.x - (previous.x + previous.width) : 0;
    const needsSpace = Boolean(previous) && gap > Math.max(1.5, previous!.height * 0.18);
    out += out && needsSpace ? ` ${text}` : text;
    previous = item;
  }
  return cleanXmlText(out);
}

function fallbackLinesFromText(page: ExtractedTextPage): TextLine[] {
  const lines = page.text.split(/\r?\n/).map(cleanXmlText).filter(Boolean);
  const pageHeight = page.height ?? lines.length * 14;
  return lines.map((text, index) => ({
    text,
    x: 0,
    y: pageHeight - index * 14,
    width: text.length * 7,
    height: 12,
    pageWidth: page.width,
    pageHeight,
  }));
}

function groupItemsIntoLines(page: ExtractedTextPage): TextLine[] {
  const items = page.items.filter((item) => item.text.trim());
  if (items.length === 0) return fallbackLinesFromText(page);

  const typicalHeight = median(items.map((item) => item.height).filter((height) => height > 0)) || 10;
  const tolerance = Math.max(3, typicalHeight * 0.65);
  const sorted = [...items].sort((a, b) => (Math.abs(b.y - a.y) > tolerance ? b.y - a.y : a.x - b.x));
  const groups: ExtractedTextItem[][] = [];

  for (const item of sorted) {
    const last = groups.at(-1);
    if (!last || Math.abs(last[0].y - item.y) > tolerance) groups.push([item]);
    else last.push(item);
  }

  return groups
    .map((group) => {
      const lineItems = group.sort((a, b) => a.x - b.x);
      const x1 = Math.min(...lineItems.map((item) => item.x));
      const x2 = Math.max(...lineItems.map((item) => item.x + item.width));
      return {
        text: joinLineItems(lineItems),
        x: x1,
        y: median(lineItems.map((item) => item.y)),
        width: Math.max(0, x2 - x1),
        height: Math.max(...lineItems.map((item) => item.height || typicalHeight)),
        pageWidth: page.width,
        pageHeight: page.height,
      };
    })
    .filter((line) => line.text);
}

function orderedLines(page: ExtractedTextPage): TextLine[] {
  const lines = groupItemsIntoLines(page);
  if (lines.length < 8) return [...lines].sort((a, b) => b.y - a.y || a.x - b.x);

  const maxX = Math.max(...lines.map((line) => line.x + line.width));
  const minX = Math.min(...lines.map((line) => line.x));
  const pageWidth = page.width ?? Math.max(1, maxX - minX);
  const candidates = lines.filter((line) => line.width > pageWidth * 0.08 && line.width < pageWidth * 0.72);
  if (candidates.length < 8) return [...lines].sort((a, b) => b.y - a.y || a.x - b.x);

  const leftEdges = candidates.map((line) => line.x).sort((a, b) => a - b);
  let largestGap = 0;
  let gapIndex = -1;
  for (let i = 1; i < leftEdges.length; i++) {
    const gap = leftEdges[i] - leftEdges[i - 1];
    if (gap > largestGap) {
      largestGap = gap;
      gapIndex = i;
    }
  }

  const minCluster = Math.max(3, Math.floor(candidates.length * 0.25));
  if (gapIndex < minCluster || candidates.length - gapIndex < minCluster || largestGap < pageWidth * 0.16) {
    return [...lines].sort((a, b) => b.y - a.y || a.x - b.x);
  }

  const split = (leftEdges[gapIndex - 1] + leftEdges[gapIndex]) / 2;
  const left = lines.filter((line) => line.x < split && line.x + line.width < split + pageWidth * 0.12);
  const right = lines.filter((line) => line.x >= split || line.x + line.width >= split + pageWidth * 0.55);
  const columnYs = [...left, ...right].map((line) => line.y);
  const topColumnY = Math.max(...columnYs);
  const bottomColumnY = Math.min(...columnYs);
  const spanning = lines.filter((line) => !left.includes(line) && !right.includes(line));
  const topSpanning = spanning.filter((line) => line.y > topColumnY).sort((a, b) => b.y - a.y || a.x - b.x);
  const middleSpanning = spanning.filter((line) => line.y <= topColumnY && line.y >= bottomColumnY).sort((a, b) => b.y - a.y || a.x - b.x);
  const bottomSpanning = spanning.filter((line) => line.y < bottomColumnY).sort((a, b) => b.y - a.y || a.x - b.x);

  return [
    ...topSpanning,
    ...left.sort((a, b) => b.y - a.y || a.x - b.x),
    ...right.sort((a, b) => b.y - a.y || a.x - b.x),
    ...middleSpanning,
    ...bottomSpanning,
  ];
}

function normalizedRepeatedLineKey(text: string): string {
  return cleanXmlText(text).toLowerCase().replace(/\d/g, '#');
}

function isStandalonePageNumber(text: string): boolean {
  return /^(?:page\s*)?\d+(?:\s*(?:\/|of)\s*\d+)?$/i.test(cleanXmlText(text));
}

function isInHeaderFooterBand(line: TextLine): boolean {
  const pageHeight = line.pageHeight;
  if (pageHeight && pageHeight > 0) return line.y >= pageHeight * 0.9 || line.y <= pageHeight * 0.1;
  return false;
}

function removeRepeatedHeadersFooters(pages: TextLine[][]): TextLine[][] {
  const counts = new Map<string, Set<number>>();

  pages.forEach((lines, pageIndex) => {
    for (const line of lines.filter(isInHeaderFooterBand)) {
      const key = normalizedRepeatedLineKey(line.text);
      if (!key) continue;
      if (!counts.has(key)) counts.set(key, new Set());
      counts.get(key)!.add(pageIndex);
    }
  });

  const repeated = new Set<string>();
  for (const [key, pageIndexes] of counts) {
    if (pageIndexes.size >= 3 || (pageIndexes.size >= 2 && pageIndexes.size / Math.max(1, pages.length) >= 0.4)) {
      repeated.add(key);
    }
  }

  return pages.map((lines) => lines.filter((line) => {
    if (isStandalonePageNumber(line.text)) return false;
    if (!isInHeaderFooterBand(line)) return true;
    return !repeated.has(normalizedRepeatedLineKey(line.text));
  }));
}

function startsWithListMarker(text: string): boolean {
  return /^(?:[-*•]|\d{1,3}[.)]|[ivxlcdm]{1,8}[.)])\s+/i.test(text);
}

function looksLikeHeading(text: string, medianLength: number): boolean {
  const cleaned = cleanXmlText(text);
  if (!cleaned || cleaned.length > Math.max(80, medianLength * 1.4)) return false;
  if (/[.!?;:]$/.test(cleaned)) return false;
  const words = cleaned.split(/\s+/);
  const capitalized = words.filter((word) => /^[A-Z0-9]/.test(word)).length;
  return words.length <= 9 && capitalized / Math.max(1, words.length) > 0.55;
}

function startsUppercase(text: string): boolean {
  return /^[A-ZÀ-Þ]/.test(text);
}

function endsTerminal(text: string): boolean {
  return /[.!?:"')\]]$/.test(text);
}

function appendWrappedLine(paragraph: string, next: string, repairHyphenation: boolean): string {
  if (repairHyphenation && /[A-Za-zÀ-ÖØ-öø-ÿ]-$/.test(paragraph) && /^[a-zà-öø-ÿ]/.test(next)) {
    return `${paragraph.slice(0, -1)}${next}`;
  }
  return `${paragraph} ${next}`;
}

function reconstructParagraphs(lines: TextLine[], options: ReflowableEpubOptions): string[] {
  if (lines.length === 0) return [];
  if (options.unwrapParagraphs === false) return lines.map((line) => line.text).filter(Boolean);

  const lengths = lines.map((line) => line.text.length).filter(Boolean);
  const medianLength = median(lengths) || 64;
  const gaps = lines.slice(1).map((line, index) => Math.max(0, lines[index].y - line.y)).filter((gap) => gap > 0);
  const normalGap = median(gaps) || Math.max(12, median(lines.map((line) => line.height).filter(Boolean)) * 1.35);
  const paragraphs: string[] = [];
  let current = '';
  let previous: TextLine | undefined;

  for (const line of lines) {
    const text = line.text;
    if (!text) continue;

    if (!current || !previous) {
      current = text;
      previous = line;
      continue;
    }

    const gap = Math.max(0, previous.y - line.y);
    const shortPrevious = previous.text.length < medianLength * 0.72;
    const indented = line.x - previous.x > Math.max(12, (line.pageWidth ?? 600) * 0.035);
    const headingBoundary = gap > normalGap * 1.2 && (looksLikeHeading(previous.text, medianLength) || looksLikeHeading(text, medianLength));
    const boundary =
      gap > normalGap * 1.65
      || startsWithListMarker(text)
      || startsWithListMarker(previous.text)
      || headingBoundary
      || indented
      || (shortPrevious && endsTerminal(previous.text) && startsUppercase(text));

    if (boundary) {
      paragraphs.push(current);
      current = text;
    } else {
      current = appendWrappedLine(current, text, options.repairHyphenation !== false);
    }
    previous = line;
  }

  if (current) paragraphs.push(current);
  return paragraphs.map(cleanXmlText).filter(Boolean);
}

function reflowableCss(): string {
  return `html {
  color-scheme: light;
}
body {
  margin: 5%;
  font-family: serif;
  line-height: 1.55;
  color: #111827;
}
.page-heading {
  margin: 0 0 1rem;
  font: 700 1rem/1.3 sans-serif;
  color: #4b5563;
}
p {
  margin: 0 0 0.85rem;
}
.empty-page {
  color: #6b7280;
  font-style: italic;
}
`;
}

function fixedCss(): string {
  return `html, body {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
}
body {
  background: #fff;
}
img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
}
`;
}

function xhtmlPage(title: string, body: string, cssHref = '../styles/book.css', language = 'en'): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="${attributeEscape(language)}" lang="${attributeEscape(language)}">
  <head>
    <title>${xmlEscape(title)}</title>
    <link rel="stylesheet" type="text/css" href="${attributeEscape(cssHref)}"/>
  </head>
  <body>
${body}
  </body>
</html>`;
}

function fixedXhtmlPage(title: string, imageHref: string, width: number, height: number, language: string): string {
  const safeWidth = Math.max(1, Math.round(width));
  const safeHeight = Math.max(1, Math.round(height));
  return `<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="${attributeEscape(language)}" lang="${attributeEscape(language)}">
  <head>
    <title>${xmlEscape(title)}</title>
    <meta name="viewport" content="width=${safeWidth}, height=${safeHeight}"/>
    <link rel="stylesheet" type="text/css" href="../styles/fixed.css"/>
  </head>
  <body>
    <img src="${attributeEscape(imageHref)}" alt="${attributeEscape(title)}"/>
  </body>
</html>`;
}

function navDocument(metadata: Metadata, docs: EpubContentDoc[]): string {
  const tocItems = docs.map((doc) =>
    `      <li><a href="${attributeEscape(doc.href)}">${xmlEscape(doc.title)}</a></li>`,
  ).join('\n');
  const startHref = docs[0]?.href ?? 'nav.xhtml';
  return `<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="${attributeEscape(metadata.language)}" lang="${attributeEscape(metadata.language)}">
  <head>
    <title>${xmlEscape(metadata.title)} navigation</title>
  </head>
  <body>
    <nav epub:type="toc" id="toc">
      <h1>${xmlEscape(metadata.title)}</h1>
      <ol>
${tocItems}
      </ol>
    </nav>
    <nav epub:type="landmarks" id="landmarks" hidden="">
      <h2>Landmarks</h2>
      <ol>
        <li><a epub:type="bodymatter" href="${attributeEscape(startHref)}">Start</a></li>
      </ol>
    </nav>
  </body>
</html>`;
}

function containerXml(): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="EPUB/package.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>`;
}

function packageDocument(metadata: Metadata, docs: EpubContentDoc[], cssFiles: { id: string; href: string }[], images: EpubImage[], fixedLayout: boolean): string {
  const creator = metadata.author ? `    <dc:creator>${xmlEscape(metadata.author)}</dc:creator>\n` : '';
  const fixedMeta = fixedLayout
    ? `    <meta property="rendition:layout">pre-paginated</meta>
    <meta property="rendition:spread">none</meta>
    <meta property="rendition:orientation">auto</meta>
`
    : '';
  const cssItems = cssFiles.map((css) =>
    `    <item id="${attributeEscape(css.id)}" href="${attributeEscape(css.href)}" media-type="text/css"/>`,
  ).join('\n');
  const contentItems = docs.map((doc) =>
    `    <item id="${attributeEscape(doc.id)}" href="${attributeEscape(doc.href)}" media-type="application/xhtml+xml"/>`,
  ).join('\n');
  const imageItems = images.map((image) =>
    `    <item id="${attributeEscape(image.id)}" href="${attributeEscape(image.href)}" media-type="${attributeEscape(image.mediaType)}"/>`,
  ).join('\n');
  const spine = docs.map((doc) => `    <itemref idref="${attributeEscape(doc.id)}"/>`).join('\n');

  return `<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" unique-identifier="book-id" xmlns="http://www.idpf.org/2007/opf" prefix="rendition: http://www.idpf.org/vocab/rendition/#">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="book-id">${xmlEscape(metadata.identifier)}</dc:identifier>
    <dc:title>${xmlEscape(metadata.title)}</dc:title>
    <dc:language>${xmlEscape(metadata.language)}</dc:language>
${creator}    <meta property="dcterms:modified">${xmlEscape(metadata.modified)}</meta>
${fixedMeta}  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
${cssItems}
${contentItems}
${imageItems}
  </manifest>
  <spine>
${spine}
  </spine>
</package>`;
}

async function writeEpub(
  metadata: Metadata,
  docs: EpubContentDoc[],
  cssFiles: { id: string; href: string; content: string }[],
  images: EpubImage[] = [],
  fixedLayout = false,
): Promise<Uint8Array> {
  const zip = new JSZip();
  zip.file('mimetype', EPUB_MIME, { compression: 'STORE' });
  zip.file('META-INF/container.xml', containerXml());
  zip.file('EPUB/package.opf', packageDocument(metadata, docs, cssFiles, images, fixedLayout));
  zip.file('EPUB/nav.xhtml', navDocument(metadata, docs));

  for (const css of cssFiles) zip.file(`EPUB/${css.href}`, css.content);
  for (const doc of docs) zip.file(`EPUB/${doc.href}`, doc.xml);
  for (const image of images) zip.file(`EPUB/${image.href}`, image.bytes);

  return zip.generateAsync({ type: 'uint8array', compression: 'DEFLATE', mimeType: EPUB_MIME });
}

function pageBody(pageNumber: number, paragraphs: string[]): string {
  const content = paragraphs.length > 0
    ? paragraphs.map((paragraph) => `    <p>${xmlEscape(paragraph)}</p>`).join('\n')
    : '    <p class="empty-page">No selectable text found on this page.</p>';
  return `    <section epub:type="chapter" aria-label="Page ${pageNumber}">
      <h1 class="page-heading">Page ${pageNumber}</h1>
${content}
    </section>`;
}

export async function textPagesToEpub(pages: ExtractedTextPage[], options: ReflowableEpubOptions = {}): Promise<Uint8Array> {
  const metadata = metadataFromOptions(options);
  const pageLines = pages.map(orderedLines);
  const cleanedLines = options.removeHeadersFooters === false ? pageLines : removeRepeatedHeadersFooters(pageLines);
  const docs = cleanedLines.map((lines, index): EpubContentDoc => {
    const pageNumber = pages[index]?.pageNumber ?? index + 1;
    const paragraphs = reconstructParagraphs(lines, options);
    const href = `text/page-${padPage(index + 1)}.xhtml`;
    const title = `Page ${pageNumber}`;
    return {
      id: `page-${padPage(index + 1)}`,
      href,
      title,
      xml: xhtmlPage(title, pageBody(pageNumber, paragraphs), '../styles/book.css', metadata.language),
    };
  });

  if (docs.length === 0) {
    docs.push({
      id: 'page-001',
      href: 'text/page-001.xhtml',
      title: 'Page 1',
      xml: xhtmlPage('Page 1', pageBody(1, []), '../styles/book.css', metadata.language),
    });
  }

  return writeEpub(metadata, docs, [{ id: 'css', href: 'styles/book.css', content: reflowableCss() }]);
}

function imageExtensionAndType(page: FixedLayoutPage): { ext: string; mediaType: string } {
  if (page.blob.type === 'image/png' || /\.png$/i.test(page.name)) return { ext: 'png', mediaType: 'image/png' };
  return { ext: 'jpg', mediaType: 'image/jpeg' };
}

export async function fixedPagesToEpub(pages: FixedLayoutPage[], options: FixedLayoutEpubOptions = {}): Promise<Uint8Array> {
  const metadata = metadataFromOptions(options);
  const docs: EpubContentDoc[] = [];
  const images: EpubImage[] = [];

  for (const [index, page] of pages.entries()) {
    const pageNumber = page.pageNumber ?? index + 1;
    const padded = padPage(index + 1);
    const { ext, mediaType } = imageExtensionAndType(page);
    const imageHref = `images/page-${padded}.${ext}`;
    const docHref = `text/page-${padded}.xhtml`;
    const title = `Page ${pageNumber}`;
    docs.push({
      id: `page-${padded}`,
      href: docHref,
      title,
      xml: fixedXhtmlPage(title, `../${imageHref}`, page.width, page.height, metadata.language),
    });
    images.push({
      id: `image-${padded}`,
      href: imageHref,
      bytes: await page.blob.arrayBuffer(),
      mediaType,
    });
  }

  if (docs.length === 0) {
    docs.push({
      id: 'page-001',
      href: 'text/page-001.xhtml',
      title: 'Page 1',
      xml: xhtmlPage('Page 1', pageBody(1, []), '../styles/book.css', metadata.language),
    });
  }

  return writeEpub(metadata, docs, [{ id: 'fixed-css', href: 'styles/fixed.css', content: fixedCss() }], images, true);
}
