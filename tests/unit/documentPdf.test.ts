import { describe, expect, it } from 'vitest';
import { sourceToPlainText, textToPdf } from '@lib/tools/documentPdf.js';
import { isPdf, pageCount } from './_fixtures.js';

describe('documentPdf', () => {
  it('extracts readable text from simple HTML', () => {
    expect(sourceToPlainText('<h1>Hello</h1><p>Private <strong>PDF</strong></p>', 'doc.html')).toBe('Hello\nPrivate PDF');
  });

  it('extracts readable text from markdown', () => {
    expect(sourceToPlainText('# Title\n\n- one\n- two\n\n[link](https://example.com)', 'note.md')).toBe('Title\n- one\n- two\n\nlink');
  });

  it('builds a valid multi-page PDF from long text', async () => {
    const source = Array.from({ length: 220 }, (_, i) => `Line ${i + 1} with enough words to wrap cleanly.`).join('\n');
    const out = await textToPdf(source, 'notes.md', { fontSize: 12 });

    expect(isPdf(out)).toBe(true);
    expect(await pageCount(out)).toBeGreaterThan(1);
  });
});
