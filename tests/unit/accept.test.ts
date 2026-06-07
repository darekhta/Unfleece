import { describe, expect, it } from 'vitest';
import { fileMatchesAccept } from '../../src/lib/handoff';

function file(name: string, type: string) {
  return new File(['x'], name, { type });
}

describe('fileMatchesAccept', () => {
  it('matches exact MIME types', () => {
    expect(fileMatchesAccept(file('doc.pdf', 'application/pdf'), 'application/pdf')).toBe(true);
    expect(fileMatchesAccept(file('doc.pdf', 'application/pdf'), 'image/png')).toBe(false);
  });

  it('matches MIME wildcards', () => {
    expect(fileMatchesAccept(file('photo.webp', 'image/webp'), 'image/*')).toBe(true);
    expect(fileMatchesAccept(file('doc.pdf', 'application/pdf'), 'image/*')).toBe(false);
  });

  it('matches extension rules when MIME type is unavailable', () => {
    expect(fileMatchesAccept(file('scan.PDF', ''), '.pdf')).toBe(true);
    expect(fileMatchesAccept(file('scan.txt', ''), '.pdf')).toBe(false);
  });

  it('handles comma-separated accept lists', () => {
    const accept = 'application/pdf, image/png, image/jpeg';
    expect(fileMatchesAccept(file('scan.png', 'image/png'), accept)).toBe(true);
    expect(fileMatchesAccept(file('scan.gif', 'image/gif'), accept)).toBe(false);
  });
});
