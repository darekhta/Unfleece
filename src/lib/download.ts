/** Trigger a browser download for a Blob. */
export function downloadBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/**
 * Derive a clean, human-friendly base name from an input filename for naming
 * outputs (e.g. "Report 2024.pdf" → "Report 2024", then callers append a
 * suffix like "-merged.pdf"). Strips the extension, trims stray separators, and
 * falls back to "document" so an oddly-named input can never produce a name
 * like "-merged.pdf".
 */
export function outputBaseName(filename: string): string {
  const stem = (filename ?? '')
    .replace(/\.[^.]+$/, '') // drop the extension
    .replace(/[\\/]+/g, ' ') // no path separators in a download name
    .trim()
    .replace(/^[.\-_\s]+|[.\-_\s]+$/g, '') // trim leading/trailing dots, dashes, underscores, spaces
    .slice(0, 100)
    .trim();
  return stem || 'document';
}

export function humanSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}
