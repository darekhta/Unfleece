import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { errorClass } from '@lib/errors.js';
import { engineForTool, reportError, telemetryEnabled } from '@lib/telemetry.js';

describe('errorClass (content-blind)', () => {
  it('classifies by category, never leaking the message', () => {
    expect(errorClass(new Error('Wrong password for "Q3 salaries.pdf"'))).toBe('password');
    expect(errorClass(new Error('Invalid PDF: no %PDF header in secret-merger-deal.pdf'))).toBe('invalid-pdf');
    expect(errorClass(new Error('Array buffer allocation failed'))).toBe('oom');
    expect(errorClass(new Error('Canvas rendering failed'))).toBe('render');
    expect(errorClass(new Error('No pages selected'))).toBe('selection');
    expect(errorClass(new Error('XFA forms are not supported'))).toBe('unsupported');
    expect(errorClass(new Error('aborted'))).toBe('cancelled');
    expect(errorClass(new Error('something weird about MyTaxes2024.pdf'))).toBe('other');
  });

  it('returns a token from the fixed allowlist for any input', () => {
    const allowed = ['cancelled', 'password', 'invalid-pdf', 'oom', 'render', 'selection', 'unsupported', 'other'];
    for (const input of [null, undefined, 42, {}, 'plain string', new Error('')]) {
      expect(allowed).toContain(errorClass(input));
    }
  });
});

describe('engineForTool', () => {
  it('maps each tool to its engine layer', () => {
    expect(engineForTool('merge')).toBe('rust');
    expect(engineForTool('sign')).toBe('rust');
    expect(engineForTool('compress')).toBe('ghostscript');
    expect(engineForTool('ocr-pdf')).toBe('tesseract');
    expect(engineForTool('image-convert')).toBe('canvas');
    expect(engineForTool('pdf-to-jpg')).toBe('pdfjs');
    expect(engineForTool('redact')).toBe('pdfjs');
    expect(engineForTool('something-new')).toBe('rust');
  });
});

describe('reportError', () => {
  const originalNavigator = Object.getOwnPropertyDescriptor(globalThis, 'navigator');
  const originalDocument = Object.getOwnPropertyDescriptor(globalThis, 'document');
  const originalLocalStorage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  let store: Map<string, string>;
  let sendBeacon: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    store = new Map();
    sendBeacon = vi.fn(() => true);
    Object.defineProperty(globalThis, 'navigator', {
      configurable: true,
      value: {
        doNotTrack: '0',
        sendBeacon,
        userAgent: 'Mozilla/5.0 AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36',
      },
    });
    Object.defineProperty(globalThis, 'document', {
      configurable: true,
      value: { documentElement: { lang: 'uk-UA' } },
    });
    Object.defineProperty(globalThis, 'localStorage', {
      configurable: true,
      value: {
        getItem: (key: string) => store.get(key) ?? null,
        setItem: (key: string, value: string) => store.set(key, value),
      },
    });
  });

  afterEach(() => {
    restoreDescriptor('navigator', originalNavigator);
    restoreDescriptor('document', originalDocument);
    restoreDescriptor('localStorage', originalLocalStorage);
    vi.restoreAllMocks();
  });

  it('sends only allowlisted content-blind fields', async () => {
    reportError('merge', new Error('Wrong password for "Q3 salaries.pdf"'), 'rust');

    expect(sendBeacon).toHaveBeenCalledOnce();
    const [url, blob] = sendBeacon.mock.calls[0];
    expect(url).toBe('/api/err');
    expect(blob).toBeInstanceOf(Blob);
    const raw = await (blob as Blob).text();
    expect(raw).not.toContain('Q3 salaries.pdf');
    expect(JSON.parse(raw)).toEqual({
      t: 'merge',
      e: 'password',
      g: 'rust',
      b: 'chrome',
      l: 'uk',
      v: '1',
    });
  });

  it('does not report cancellations, opt-out, or Do-Not-Track', () => {
    reportError('merge', new Error('aborted'), 'rust');
    expect(sendBeacon).not.toHaveBeenCalled();

    store.set('uf-telemetry', 'off');
    reportError('merge', new Error('Invalid PDF'), 'rust');
    expect(sendBeacon).not.toHaveBeenCalled();

    store.clear();
    Object.defineProperty(globalThis, 'navigator', {
      configurable: true,
      value: { doNotTrack: '1', sendBeacon, userAgent: 'Mozilla/5.0 Firefox/130.0' },
    });
    expect(telemetryEnabled()).toBe(false);
    reportError('merge', new Error('Invalid PDF'), 'rust');
    expect(sendBeacon).not.toHaveBeenCalled();
  });
});

function restoreDescriptor(key: 'navigator' | 'document' | 'localStorage', descriptor: PropertyDescriptor | undefined): void {
  if (descriptor) {
    Object.defineProperty(globalThis, key, descriptor);
  } else {
    Reflect.deleteProperty(globalThis, key);
  }
}
