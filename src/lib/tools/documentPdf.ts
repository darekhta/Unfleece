import { PDFDocument, StandardFonts, rgb, type PDFFont } from '@cantoo/pdf-lib';
import { PAGE_SIZES } from '../util/pdf.js';

export interface TextToPdfOptions {
  title?: string;
  pageSize?: keyof typeof PAGE_SIZES;
  fontSize?: number;
  margin?: number;
}

function stripHtml(source: string): string {
  return source
    .replace(/<script[\s\S]*?<\/script>/gi, '')
    .replace(/<style[\s\S]*?<\/style>/gi, '')
    .replace(/<\/(p|div|section|article|header|footer|h[1-6]|li|tr)>/gi, '\n')
    .replace(/<br\s*\/?>/gi, '\n')
    .replace(/<li[^>]*>/gi, '- ')
    .replace(/<[^>]+>/g, '')
    .replace(/&nbsp;/gi, ' ')
    .replace(/&amp;/gi, '&')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/&quot;/gi, '"')
    .replace(/&#39;/gi, "'");
}

function stripMarkdown(source: string): string {
  return source
    .replace(/```[\s\S]*?```/g, (block) => block.replace(/```[a-z]*\n?/gi, '').replace(/```/g, ''))
    .replace(/^#{1,6}\s+/gm, '')
    .replace(/^>\s?/gm, '')
    .replace(/^\s*[-*+]\s+/gm, '- ')
    .replace(/^\s*\d+\.\s+/gm, (match) => `${match.trim()} `)
    .replace(/!\[([^\]]*)\]\([^)]+\)/g, '$1')
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/[*_~`]/g, '');
}

export function sourceToPlainText(source: string, filename = ''): string {
  const lower = filename.toLowerCase();
  const stripped = lower.endsWith('.html') || lower.endsWith('.htm') || /<\/?[a-z][\s\S]*>/i.test(source)
    ? stripHtml(source)
    : stripMarkdown(source);
  return stripped
    .split(/\r?\n/)
    .map((line) => line.replace(/\s+/g, ' ').trim())
    .join('\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

function safeText(text: string): string {
  return text
    .normalize('NFKD')
    .replace(/[^\x09\x0A\x0D\x20-\x7E]/g, '?');
}

function wrapLine(text: string, maxWidth: number, font: PDFFont, fontSize: number): string[] {
  const words = text.split(/\s+/).filter(Boolean);
  if (words.length === 0) return [''];

  const lines: string[] = [];
  let current = '';
  for (const word of words) {
    const next = current ? `${current} ${word}` : word;
    if (font.widthOfTextAtSize(next, fontSize) <= maxWidth) {
      current = next;
      continue;
    }
    if (current) lines.push(current);
    if (font.widthOfTextAtSize(word, fontSize) <= maxWidth) {
      current = word;
    } else {
      let chunk = '';
      for (const char of word) {
        const candidate = chunk + char;
        if (font.widthOfTextAtSize(candidate, fontSize) > maxWidth && chunk) {
          lines.push(chunk);
          chunk = char;
        } else {
          chunk = candidate;
        }
      }
      current = chunk;
    }
  }
  if (current) lines.push(current);
  return lines;
}

export async function textToPdf(source: string, filename = '', opts: TextToPdfOptions = {}): Promise<Uint8Array> {
  const {
    title = filename.replace(/\.[^.]+$/, '') || 'Document',
    pageSize = 'a4',
    fontSize = 11,
    margin = 54,
  } = opts;
  const [pageWidth, pageHeight] = PAGE_SIZES[pageSize] ?? PAGE_SIZES.a4;
  const doc = await PDFDocument.create();
  const regular = await doc.embedFont(StandardFonts.Helvetica);
  const bold = await doc.embedFont(StandardFonts.HelveticaBold);
  const lineHeight = fontSize * 1.45;
  const maxWidth = pageWidth - margin * 2;
  const bottom = margin;

  let page = doc.addPage([pageWidth, pageHeight]);
  let y = pageHeight - margin;

  const titleText = safeText(title);
  page.drawText(titleText, { x: margin, y, size: fontSize + 5, font: bold, color: rgb(0.08, 0.1, 0.16) });
  y -= lineHeight * 1.8;

  const text = sourceToPlainText(source, filename) || 'No text content found.';
  for (const paragraph of text.split(/\n{2,}/)) {
    const lines = paragraph.split(/\n/).flatMap((line) => wrapLine(safeText(line), maxWidth, regular, fontSize));
    for (const line of lines) {
      if (y < bottom) {
        page = doc.addPage([pageWidth, pageHeight]);
        y = pageHeight - margin;
      }
      page.drawText(line || ' ', { x: margin, y, size: fontSize, font: regular, color: rgb(0.08, 0.1, 0.16) });
      y -= lineHeight;
    }
    y -= lineHeight * 0.5;
  }

  doc.setTitle(titleText);
  doc.setCreator('Unfleece');
  return doc.save();
}
