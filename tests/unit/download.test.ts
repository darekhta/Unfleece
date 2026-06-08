import { describe, expect, it } from 'vitest';
import { outputBaseName } from '@lib/download.js';

describe('outputBaseName', () => {
  it('strips the extension and keeps a clean stem', () => {
    expect(outputBaseName('report.pdf')).toBe('report');
    expect(outputBaseName('Report 2024.pdf')).toBe('Report 2024');
    expect(outputBaseName('scan_001.PNG')).toBe('scan_001');
  });

  it('falls back to "document" for empty or extension-only names', () => {
    expect(outputBaseName('.pdf')).toBe('document');
    expect(outputBaseName('')).toBe('document');
    expect(outputBaseName('   ')).toBe('document');
    expect(outputBaseName('---.pdf')).toBe('document');
  });

  it('trims stray separators and path segments so the suffix reads cleanly', () => {
    expect(outputBaseName('  spaced  .pdf')).toBe('spaced');
    expect(outputBaseName('-leading.pdf')).toBe('leading');
    expect(outputBaseName('folder/sub/file.pdf')).toBe('folder sub file');
    // the practical point: name + suffix never starts with a dash
    expect(`${outputBaseName('.pdf')}-merged.pdf`).toBe('document-merged.pdf');
  });

  it('handles names without an extension and very long names', () => {
    expect(outputBaseName('noext')).toBe('noext');
    expect(outputBaseName('a'.repeat(200) + '.pdf').length).toBeLessThanOrEqual(100);
  });
});
