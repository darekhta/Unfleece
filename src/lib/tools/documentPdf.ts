import { wasmTextToPdf } from '../wasm/core.js';

export interface TextToPdfOptions {
  title?: string;
  pageSize?: 'a4' | 'letter';
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

/** NFKD-fold + ASCII-clamp the title (the Rust core re-clamps but does not NFKD). */
function safeText(text: string): string {
  return text
    .normalize('NFKD')
    .replace(/[^\x09\x0A\x0D\x20-\x7E]/g, '?');
}

export async function textToPdf(source: string, filename = '', opts: TextToPdfOptions = {}): Promise<Uint8Array> {
  const {
    title = filename.replace(/\.[^.]+$/, '') || 'Document',
    pageSize = 'a4',
    fontSize = 11,
    margin = 54,
  } = opts;
  // HTML/Markdown stripping + NFKD title folding stay in TS; the Rust core lays
  // out the text into PDF pages with the shared Helvetica metrics.
  const text = sourceToPlainText(source, filename) || 'No text content found.';
  return wasmTextToPdf(text, { title: safeText(title), pageSize, fontSize, margin });
}
