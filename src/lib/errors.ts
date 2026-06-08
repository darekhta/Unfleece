export interface FriendlyError {
  title: string;
  message: string;
}

const PRIVACY_NOTE = 'Nothing left your device.';

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function firstLine(message: string): string {
  return message.trim().split(/\r?\n/, 1)[0] ?? '';
}

export function rawErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (isRecord(error) && typeof error.message === 'string') return error.message;
  if (typeof error === 'string') return error;
  return '';
}

function errorName(error: unknown): string {
  if (error instanceof Error) return error.name;
  if (isRecord(error) && typeof error.name === 'string') return error.name;
  return '';
}

function sentenceWithPrivacy(message: string): string {
  return message.endsWith(PRIVACY_NOTE) ? message : `${message} ${PRIVACY_NOTE}`;
}

export function friendlyError(error: unknown, fallbackTitle = 'Something went wrong'): FriendlyError {
  if (isRecord(error) && typeof error.title === 'string' && typeof error.message === 'string') {
    return { title: error.title, message: error.message };
  }

  const raw = firstLine(rawErrorMessage(error));
  const haystack = `${errorName(error)} ${raw}`.toLowerCase();

  if (haystack.includes('abort')) {
    return {
      title: 'Cancelled',
      message: 'That run was cancelled. Your file stayed on this device.',
    };
  }

  if (/(password|encrypted|encrypt|protected)/i.test(haystack)) {
    return {
      title: 'Password-protected PDF',
      message: sentenceWithPrivacy('Use Unlock PDF with the password, then try this tool again.'),
    };
  }

  if (
    /(invalid pdf|no pdf header|failed to parse|parse pdf|not a pdf|xref|trailer|unexpected eof|corrupt|malformed|cannot read.*pdf)/i
      .test(haystack)
  ) {
    return {
      title: 'Invalid PDF',
      message: sentenceWithPrivacy('This does not look like a valid PDF. Choose a different PDF and try again.'),
    };
  }

  if (/(out of memory|allocation|array buffer allocation|max.*call stack|file.*too large|too large)/i.test(haystack)) {
    return {
      title: 'File too large for this device',
      message: sentenceWithPrivacy('Your browser ran out of room while processing this file. Try a smaller document or fewer files.'),
    };
  }

  if (/(canvas|toblob|dommatrix|render|rendering|bitmap|webgl)/i.test(haystack)) {
    return {
      title: 'Browser rendering issue',
      message: sentenceWithPrivacy('Your browser could not render this PDF. Try refreshing, updating the browser, or using a different PDF.'),
    };
  }

  if (
    /(draw your signature|no pages selected|cannot remove every page|order must|enter at least|invalid page range|page range|select at least|choose at least)/i
      .test(haystack)
  ) {
    return {
      title: 'Check the selection',
      message: raw || sentenceWithPrivacy('Update the options and try again.'),
    };
  }

  return {
    title: fallbackTitle,
    message: sentenceWithPrivacy('We could not finish this file. Try another file or a smaller document.'),
  };
}
