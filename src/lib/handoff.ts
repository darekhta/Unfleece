// Cross-page file handoff. The hero "quick start" stashes dropped files here,
// then navigates to the chosen tool, whose runner picks them up. Uses IndexedDB
// (stores File/Blob natively via structured clone) so it survives the MPA page
// navigation AND handles large files without base64 inflation. Everything stays
// on the device — this is local storage, not an upload.

const DB = 'unfleece';
const STORE = 'handoff';
const KEY = 'pending';

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(STORE);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

/** Stash files for the next tool page to consume. */
export async function stashFiles(files: File[]): Promise<void> {
  if (!files.length || typeof indexedDB === 'undefined') return;
  // Copy to a plain array of raw File objects — a Svelte $state proxy array is
  // not structured-cloneable and would throw DataCloneError.
  const plain: File[] = Array.from(files, (f) => f);
  const db = await openDb();
  try {
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction(STORE, 'readwrite');
      tx.objectStore(STORE).put(plain, KEY);
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
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
        const val = getReq.result as File[] | undefined;
        resolve(val && val.length ? val : null);
      };
      getReq.onerror = () => reject(getReq.error);
    });
  } catch {
    return null;
  } finally {
    db.close();
  }
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
      if (rule.endsWith('/*')) return mime.startsWith(rule.slice(0, -1));
      if (rule.startsWith('.')) return name.endsWith(rule);
      return mime === rule;
    });
}
