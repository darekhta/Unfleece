// Document metadata read/set/strip, executed by the Rust→WASM core (lopdf).
// The wasm layer speaks verbatim PDF date strings ("D:YYYYMMDD…"); this module
// converts them to/from JS `Date` to keep the exported contract unchanged.
import { wasmReadMetadata, wasmSetMetadata, wasmStripMetadata } from '../wasm/core.js';

export interface PdfMetadata {
  title?: string;
  author?: string;
  subject?: string;
  keywords?: string[];
  creator?: string;
  producer?: string;
  creationDate?: Date;
  modificationDate?: Date;
  pageCount: number;
}

// PDF date string (PDF 32000-1 §7.9.4): D:YYYY[MM[DD[HH[mm[SS[O[HH'mm']]]]]]]
const PDF_DATE =
  /^D:(\d{4})(\d{2})?(\d{2})?(\d{2})?(\d{2})?(\d{2})?([Zz]|[+-]\d{2}(?:'?\d{2}'?)?)?$/;

function validDateParts(year: number, month: number, day: number, hour: number, minute: number, second: number): boolean {
  if (month < 1 || month > 12) return false;
  if (day < 1 || day > 31) return false;
  if (hour < 0 || hour > 23) return false;
  if (minute < 0 || minute > 59) return false;
  if (second < 0 || second > 59) return false;
  const normalized = new Date(Date.UTC(year, month - 1, day, hour, minute, second));
  return (
    normalized.getUTCFullYear() === year &&
    normalized.getUTCMonth() === month - 1 &&
    normalized.getUTCDate() === day &&
    normalized.getUTCHours() === hour &&
    normalized.getUTCMinutes() === minute &&
    normalized.getUTCSeconds() === second
  );
}

/** Parse a PDF date string into a JS Date; `undefined` if absent/malformed. */
function parsePdfDate(value: string | undefined): Date | undefined {
  if (!value) return undefined;
  const match = PDF_DATE.exec(value.trim());
  if (!match) return undefined;
  const [, year, month = '01', day = '01', hour = '00', minute = '00', second = '00', tz] = match;
  const parts = [+year, +month, +day, +hour, +minute, +second] as const;
  if (!validDateParts(...parts)) return undefined;
  let offsetMinutes = 0;
  if (tz && tz !== 'Z' && tz !== 'z') {
    const sign = tz.startsWith('-') ? -1 : 1;
    const digits = tz.slice(1).replace(/'/g, '');
    const offsetHours = Number(digits.slice(0, 2));
    const offsetMins = Number(digits.slice(2, 4) || '0');
    if (offsetHours > 23 || offsetMins > 59) return undefined;
    offsetMinutes = sign * (offsetHours * 60 + offsetMins);
  }
  const date = new Date(
    Date.UTC(parts[0], parts[1] - 1, parts[2], parts[3], parts[4], parts[5]) - offsetMinutes * 60_000,
  );
  return Number.isNaN(date.getTime()) ? undefined : date;
}

/** Format a JS Date as a UTC PDF date string (same shape as before: D:…Z). */
function toPdfDate(date: Date): string {
  if (Number.isNaN(date.getTime())) throw new Error('Invalid metadata date');
  const pad = (n: number, width = 2) => String(n).padStart(width, '0');
  return (
    `D:${pad(date.getUTCFullYear(), 4)}${pad(date.getUTCMonth() + 1)}${pad(date.getUTCDate())}` +
    `${pad(date.getUTCHours())}${pad(date.getUTCMinutes())}${pad(date.getUTCSeconds())}Z`
  );
}

/** Read document metadata. */
export async function getMetadata(bytes: Uint8Array): Promise<PdfMetadata> {
  const meta = await wasmReadMetadata(bytes);
  return {
    title: meta.title,
    author: meta.author,
    subject: meta.subject,
    keywords: meta.keywords,
    creator: meta.creator,
    producer: meta.producer,
    creationDate: parsePdfDate(meta.creationDate),
    modificationDate: parsePdfDate(meta.modificationDate),
    pageCount: meta.pageCount ?? 0,
  };
}

export type MetadataUpdate = Partial<Omit<PdfMetadata, 'pageCount'>>;

/**
 * Set (overwrite) the provided metadata fields. Absent fields are left
 * untouched; an empty string (or empty keywords array) removes the field
 * entirely from the Info dictionary.
 */
export async function setMetadata(bytes: Uint8Array, meta: MetadataUpdate): Promise<Uint8Array> {
  const update: Record<string, unknown> = {};
  if (meta.title !== undefined) update.title = meta.title;
  if (meta.author !== undefined) update.author = meta.author;
  if (meta.subject !== undefined) update.subject = meta.subject;
  // Rust stores arrays as one ", "-joined string so the delimiter survives a
  // round-trip through getMetadata() (raw comma strings are also accepted).
  if (meta.keywords !== undefined) update.keywords = meta.keywords;
  if (meta.creator !== undefined) update.creator = meta.creator;
  if (meta.producer !== undefined) update.producer = meta.producer;
  if (meta.creationDate !== undefined) update.creationDate = toPdfDate(meta.creationDate);
  if (meta.modificationDate !== undefined) update.modificationDate = toPdfDate(meta.modificationDate);
  return wasmSetMetadata(bytes, update);
}

/**
 * Strip all document metadata (privacy hygiene): removes the entire Info
 * dictionary AND the XMP metadata stream — stronger than blanking the fields.
 */
export async function stripMetadata(bytes: Uint8Array): Promise<Uint8Array> {
  return wasmStripMetadata(bytes);
}
