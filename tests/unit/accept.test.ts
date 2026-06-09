import { describe, expect, it } from 'vitest';
import { fileMatchesAccept, restoreHandoffFiles, serializeHandoffFiles } from '../../src/lib/handoff';

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

  it('falls back to common extensions when browsers omit MIME types', () => {
    expect(fileMatchesAccept(file('scan.pdf', ''), 'application/pdf')).toBe(true);
    expect(fileMatchesAccept(file('photo.JPG', ''), 'image/jpeg')).toBe(true);
    expect(fileMatchesAccept(file('scan.tif', ''), 'image/*')).toBe(true);
    expect(fileMatchesAccept(file('notes.txt', ''), 'application/pdf')).toBe(false);
  });
});

describe('handoff serialization', () => {
  it('stores ArrayBuffers instead of File or Blob objects', async () => {
    const original = new File(['private pdf bytes'], 'Private.pdf', {
      type: 'application/pdf',
      lastModified: 123,
    });

    const stored = await serializeHandoffFiles([original]);

    expect(stored.files).toHaveLength(1);
    expect(stored.files[0].bytes).toBeInstanceOf(ArrayBuffer);
    expect(stored.files[0]).not.toHaveProperty('blob');

    const restored = restoreHandoffFiles(stored);
    expect(restored).toHaveLength(1);
    expect(restored?.[0].name).toBe('Private.pdf');
    expect(restored?.[0].type).toBe('application/pdf');
    expect(restored?.[0].lastModified).toBe(123);
    expect(await restored?.[0].text()).toBe('private pdf bytes');
  });

  it('still reads legacy File-array handoff values', () => {
    const legacy = [file('old.pdf', 'application/pdf')];
    const restored = restoreHandoffFiles(legacy);

    expect(restored).toHaveLength(1);
    expect(restored?.[0].name).toBe('old.pdf');
  });
});
