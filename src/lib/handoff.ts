// Cross-page file handoff. The hero "quick start" stashes dropped files here,
// then navigates to the chosen tool, whose runner picks them up. Uses IndexedDB
// so it survives the MPA page navigation AND handles large files without base64
// inflation. Everything stays on the device — this is local storage, not an upload.
//
// Store raw ArrayBuffers, never File/Blob objects: WebKit can silently fail to
// persist Blob-backed values in IndexedDB.

const DB = 'unfleece';
const STORE = 'handoff';
const KEY = 'pending';

export interface StoredHandoffFile {
  name: string;
  type: string;
  lastModified: number;
  bytes: ArrayBuffer;
}

export interface StoredHandoff {
  version: 2;
  files: StoredHandoffFile[];
}

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(STORE);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

export async function serializeHandoffFiles(files: File[]): Promise<StoredHandoff> {
  return {
    version: 2,
    files: await Promise.all(
      files.map(async (file) => ({
        name: file.name,
        type: file.type,
        lastModified: file.lastModified,
        bytes: await file.arrayBuffer(),
      })),
    ),
  };
}

function bytesFromStored(value: unknown): ArrayBuffer | null {
  if (value instanceof ArrayBuffer) return value;
  if (ArrayBuffer.isView(value)) {
    const view = value as ArrayBufferView;
    const copy = new Uint8Array(view.byteLength);
    copy.set(new Uint8Array(view.buffer, view.byteOffset, view.byteLength));
    return copy.buffer;
  }
  return null;
}

function fileFromStored(value: unknown): File | null {
  if (value instanceof File) return value;
  if (typeof value !== 'object' || value === null) return null;
  const record = value as Partial<StoredHandoffFile>;
  if (typeof record.name !== 'string') return null;
  const bytes = bytesFromStored(record.bytes);
  if (!bytes) return null;
  return new File([bytes], record.name, {
    type: typeof record.type === 'string' ? record.type : '',
    lastModified: typeof record.lastModified === 'number' ? record.lastModified : Date.now(),
  });
}

export function restoreHandoffFiles(value: unknown): File[] | null {
  let rawFiles: unknown[] = [];
  if (Array.isArray(value)) {
    rawFiles = value;
  } else if (typeof value === 'object' && value !== null) {
    const maybeFiles = (value as { files?: unknown }).files;
    if (Array.isArray(maybeFiles)) rawFiles = maybeFiles;
  }
  const files = rawFiles.map(fileFromStored).filter((file): file is File => Boolean(file));
  return files.length ? files : null;
}

/** Stash files for the next tool page to consume. */
export async function stashFiles(files: File[]): Promise<void> {
  if (!files.length) return;
  if (typeof indexedDB === 'undefined') throw new Error('Local file handoff storage is not available');
  const record = await serializeHandoffFiles(Array.from(files, (f) => f));
  const db = await openDb();
  try {
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction(STORE, 'readwrite');
      tx.objectStore(STORE).put(record, KEY);
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error ?? new Error('IndexedDB transaction aborted'));
    });
  } finally {
    db.close();
  }
}

/** Read and clear any stashed files (returns null if none). */
export async function takePendingFiles(): Promise<File[] | null> {
  if (typeof indexedDB === 'undefined') return null;
  let db: IDBDatabase;
  try {
    db = await openDb();
  } catch {
    return null;
  }
  try {
    return await new Promise<File[] | null>((resolve, reject) => {
      const tx = db.transaction(STORE, 'readwrite');
      const store = tx.objectStore(STORE);
      const getReq = store.get(KEY);
      getReq.onsuccess = () => {
        store.delete(KEY);
        resolve(restoreHandoffFiles(getReq.result));
      };
      getReq.onerror = () => reject(getReq.error);
    });
  } catch {
    return null;
  } finally {
    db.close();
  }
}

const MIME_EXTENSION_FALLBACKS: Record<string, string[]> = {
  'application/pdf': ['.pdf'],
  'image/png': ['.png'],
  'image/jpeg': ['.jpg', '.jpeg', '.jfif', '.pjp', '.pjpeg'],
  'image/webp': ['.webp'],
  'image/avif': ['.avif'],
  'image/jxl': ['.jxl'],
  'image/tiff': ['.tif', '.tiff'],
  'image/bmp': ['.bmp'],
  'image/gif': ['.gif'],
  'image/x-icon': ['.ico', '.cur'],
  'image/vnd.microsoft.icon': ['.ico', '.cur'],
  'image/vnd.adobe.photoshop': ['.psd', '.psb'],
  'image/heic': ['.heic'],
  'image/heif': ['.heif'],
  'image/jp2': ['.jp2', '.j2k'],
  'text/html': ['.html', '.htm'],
  'text/markdown': ['.md', '.markdown'],
  'text/plain': ['.txt'],
};

const IMAGE_EXTENSIONS = new Set(Object.entries(MIME_EXTENSION_FALLBACKS)
  .filter(([mime]) => mime.startsWith('image/'))
  .flatMap(([, extensions]) => extensions));

function hasExtension(name: string, extensions: Iterable<string>): boolean {
  for (const ext of extensions) {
    if (name.endsWith(ext)) return true;
  }
  return false;
}

function mimeCanUseExtensionFallback(mime: string): boolean {
  return mime === '' || mime === 'application/octet-stream';
}

/** Does a file satisfy an HTML `accept` string? */
export function fileMatchesAccept(file: File, accept: string): boolean {
  const mime = file.type.toLowerCase();
  const name = file.name.toLowerCase();
  return accept
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
    .some((token) => {
      const rule = token.toLowerCase();
      if (rule === '*/*') return true;
      if (rule.endsWith('/*')) {
        if (mime.startsWith(rule.slice(0, -1))) return true;
        return rule === 'image/*' && mimeCanUseExtensionFallback(mime) && hasExtension(name, IMAGE_EXTENSIONS);
      }
      if (rule.startsWith('.')) return name.endsWith(rule);
      if (mime === rule) return true;
      const extensions = MIME_EXTENSION_FALLBACKS[rule];
      return Boolean(extensions && mimeCanUseExtensionFallback(mime) && hasExtension(name, extensions));
    });
}
