// Local signature library — saved ONLY in this browser (IndexedDB), never
// uploaded. Uses its own database so it doesn't version-bump the handoff DB.
//
// Records store raw ArrayBuffer bytes, NOT Blobs: WebKit (iOS Safari) can
// silently fail to persist Blob values in IndexedDB, which loses the library
// on the next reload. ArrayBuffers structured-clone reliably everywhere.

const DB = 'unfleece-sigs';
const STORE = 'sigs';

export interface SavedSignature {
  id: string;
  blob: Blob;
  imageType: 'png' | 'jpg';
  aspect: number;
  createdAt: number;
}

interface StoredSig {
  id: string;
  bytes?: ArrayBuffer;
  /** legacy shape (pre WebKit fix) — kept readable for migration */
  blob?: Blob;
  imageType: 'png' | 'jpg';
  aspect: number;
  createdAt: number;
}

function mime(t: 'png' | 'jpg'): string {
  return t === 'png' ? 'image/png' : 'image/jpeg';
}

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(STORE, { keyPath: 'id' });
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

function tx<T>(mode: IDBTransactionMode, run: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  return openDb().then(
    (db) =>
      new Promise<T>((resolve, reject) => {
        const t = db.transaction(STORE, mode);
        const req = run(t.objectStore(STORE));
        t.oncomplete = () => {
          db.close();
          resolve(req.result);
        };
        t.onerror = () => {
          db.close();
          reject(t.error);
        };
        t.onabort = () => {
          db.close();
          reject(t.error ?? new Error('IndexedDB transaction aborted'));
        };
      }),
  );
}

export async function listSignatures(): Promise<SavedSignature[]> {
  if (typeof indexedDB === 'undefined') return [];
  try {
    const all = await tx<StoredSig[]>('readonly', (s) => s.getAll() as IDBRequest<StoredSig[]>);
    return (all ?? [])
      .map((r) => {
        // tolerate any historical record shape: Blob, ArrayBuffer, or a typed-array view
        const raw = r.bytes as unknown;
        const buf =
          raw instanceof ArrayBuffer ? raw : ArrayBuffer.isView(raw) ? (raw.buffer as ArrayBuffer) : new ArrayBuffer(0);
        const imageType: 'png' | 'jpg' = r.imageType === 'jpg' ? 'jpg' : 'png';
        return {
          id: r.id,
          blob: r.blob instanceof Blob ? r.blob : new Blob([buf], { type: mime(imageType) }),
          imageType,
          aspect: Number.isFinite(r.aspect) && r.aspect > 0 ? r.aspect : 0.32,
          createdAt: r.createdAt ?? 0,
        };
      })
      .filter((r) => r.blob.size > 0)
      .sort((a, b) => b.createdAt - a.createdAt);
  } catch {
    return [];
  }
}

export async function saveSignature(blob: Blob, imageType: 'png' | 'jpg', aspect: number): Promise<SavedSignature> {
  const id = typeof crypto !== 'undefined' && 'randomUUID' in crypto ? crypto.randomUUID() : String(Date.now());
  const createdAt = Date.now();
  const bytes = await blob.arrayBuffer();
  await tx('readwrite', (s) => s.put({ id, bytes, imageType, aspect, createdAt } satisfies StoredSig));
  return { id, blob, imageType, aspect, createdAt };
}

export async function deleteSignature(id: string): Promise<void> {
  await tx('readwrite', (s) => s.delete(id));
}
