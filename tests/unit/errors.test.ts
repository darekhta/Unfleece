import { describe, expect, it } from 'vitest';
import { friendlyError, rawErrorMessage } from '../../src/lib/errors';

describe('friendlyError', () => {
  it('maps parser failures to a user-facing invalid PDF message', () => {
    const error = friendlyError(new Error('Failed to parse PDF document: No PDF header found'));

    expect(error).toEqual({
      title: 'Invalid PDF',
      message: 'This does not look like a valid PDF. Choose a different PDF and try again. No file left your device.',
    });
  });

  it('maps password-protected PDFs without exposing implementation details', () => {
    const error = friendlyError(new Error('Input document is encrypted and cannot be modified'));

    expect(error.title).toBe('Password-protected PDF');
    expect(error.message).toContain('Use Unlock PDF with the password');
    expect(error.message).toContain('No file left your device');
  });

  it('keeps actionable validation messages intact', () => {
    const error = friendlyError(new Error('Draw your signature first.'));

    expect(error).toEqual({
      title: 'Check the selection',
      message: 'Draw your signature first.',
    });
  });

  it('maps cancellations separately', () => {
    const error = friendlyError({ name: 'AbortError', message: 'The operation was aborted' });

    expect(error).toEqual({
      title: 'Cancelled',
      message: 'That run was cancelled. Your file stayed on this device.',
    });
  });

  it('uses a generic private fallback for unknown failures', () => {
    const error = friendlyError(new Error('Cannot read properties of undefined'), 'Could not process file');

    expect(error).toEqual({
      title: 'Could not process file',
      message: 'We could not finish this file. Try another file or a smaller document. No file left your device.',
    });
  });
});

describe('rawErrorMessage', () => {
  it('extracts messages from Error-like values', () => {
    expect(rawErrorMessage(new Error('Nope'))).toBe('Nope');
    expect(rawErrorMessage({ message: 'Still nope' })).toBe('Still nope');
    expect(rawErrorMessage('Plain string')).toBe('Plain string');
  });
});
