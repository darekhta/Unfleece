import { PDFArray, PDFDict, PDFDocument, PDFName, PDFRef } from '@cantoo/pdf-lib';
import { notifyProgress, type ProgressCallback } from '../progress.js';

export interface SanitizeOptions {
  removeAnnotations?: boolean;
  removeForms?: boolean;
}

export interface ProtectOptions {
  userPassword?: string;
  ownerPassword?: string;
  allowPrinting?: boolean;
  allowCopying?: boolean;
  allowModifying?: boolean;
}

const name = (key: string) => PDFName.of(key);

const INFO_KEYS = [
  PDFName.Title,
  PDFName.Author,
  PDFName.Subject,
  PDFName.Keywords,
  PDFName.Creator,
  PDFName.Producer,
  PDFName.CreationDate,
  PDFName.ModDate,
];

const CATALOG_KEYS = [
  'AA',
  'OpenAction',
  'Metadata',
  'PieceInfo',
  'StructTreeRoot',
  'SpiderInfo',
  'Collection',
  'Requirements',
];

const PAGE_KEYS = [
  'AA',
  'Metadata',
  'PieceInfo',
  'PresSteps',
];

const ANNOTATION_ACTIVE_KEYS = [
  'A',
  'AA',
  'JS',
  'FS',
  'RichMediaContent',
  'RichMediaSettings',
  'Sound',
  'Movie',
  'Rendition',
];

const NAME_TREE_KEYS = [
  'JavaScript',
  'EmbeddedFiles',
  'Renditions',
  'AlternatePresentations',
];

const DANGEROUS_TYPES = new Set([
  '/EmbeddedFile',
  '/Filespec',
  '/JavaScript',
  '/Rendition',
  '/RichMedia',
  '/Sound',
  '/Movie',
]);

const DANGEROUS_ACTIONS = new Set([
  '/JavaScript',
  '/Launch',
  '/SubmitForm',
  '/ImportData',
  '/GoToE',
  '/GoToR',
  '/Rendition',
  '/RichMediaExecute',
]);

function deleteKeys(dict: PDFDict | undefined, keys: string[]): number {
  if (!dict) return 0;
  let removed = 0;
  for (const key of keys) {
    if (dict.delete(name(key))) removed += 1;
  }
  return removed;
}

function deleteInfoDict(doc: PDFDocument): number {
  const ref = doc.context.trailerInfo.Info;
  const info = doc.context.lookupMaybe(ref, PDFDict);
  let removed = 0;
  if (info) {
    for (const key of INFO_KEYS) {
      if (info.delete(key)) removed += 1;
    }
  }
  if (ref instanceof PDFRef && doc.context.delete(ref)) removed += 1;
  delete doc.context.trailerInfo.Info;
  return removed;
}

function cleanNameTree(doc: PDFDocument): number {
  const names = doc.catalog.lookupMaybe(name('Names'), PDFDict);
  if (!names) return 0;
  let removed = deleteKeys(names, NAME_TREE_KEYS);
  if (names.keys().length === 0 && doc.catalog.delete(name('Names'))) removed += 1;
  return removed;
}

function cleanAnnotation(annotation: PDFDict): number {
  return deleteKeys(annotation, ANNOTATION_ACTIVE_KEYS);
}

function cleanPageAnnotations(pageAnnots: PDFArray | undefined, removeAnnotations: boolean): number {
  if (!pageAnnots || removeAnnotations) return 0;
  let removed = 0;
  for (let i = 0; i < pageAnnots.size(); i++) {
    const annotation = pageAnnots.lookupMaybe(i, PDFDict);
    if (annotation) removed += cleanAnnotation(annotation);
  }
  return removed;
}

function deleteDangerousIndirectObjects(doc: PDFDocument): number {
  let removed = 0;
  for (const [ref, object] of doc.context.enumerateIndirectObjects()) {
    const dict = object instanceof PDFDict
      ? object
      : 'dict' in object && object.dict instanceof PDFDict
        ? object.dict
        : undefined;
    if (!dict) continue;

    const type = dict.lookupMaybe(PDFName.Type, PDFName)?.toString();
    const subtype = dict.lookupMaybe(name('Subtype'), PDFName)?.toString();
    const action = dict.lookupMaybe(name('S'), PDFName)?.toString();
    if (
      (type && DANGEROUS_TYPES.has(type)) ||
      (subtype && DANGEROUS_TYPES.has(subtype)) ||
      (action && DANGEROUS_ACTIONS.has(action))
    ) {
      if (doc.context.delete(ref)) removed += 1;
    }
  }
  return removed;
}

/** Strip common active and hidden PDF structures before sharing. */
export async function sanitizePdf(bytes: Uint8Array, options: SanitizeOptions = {}, onProgress?: ProgressCallback): Promise<Uint8Array> {
  const doc = await PDFDocument.load(bytes, { ignoreEncryption: true, updateMetadata: false });
  const removeAnnotations = options.removeAnnotations !== false;
  const removeForms = options.removeForms !== false;
  let removed = 0;

  notifyProgress(onProgress, { phase: 'working', label: 'Removing document metadata and name trees…', current: 0, total: 4 });
  removed += deleteInfoDict(doc);
  removed += deleteKeys(doc.catalog, CATALOG_KEYS);
  removed += cleanNameTree(doc);
  if (removeForms && doc.catalog.delete(name('AcroForm'))) removed += 1;

  const pages = doc.getPages();
  notifyProgress(onProgress, { phase: 'working', label: `Cleaning ${pages.length} pages…`, current: 1, total: 4 });
  for (const [i, page] of pages.entries()) {
    removed += deleteKeys(page.node, PAGE_KEYS);
    const annots = page.node.lookupMaybe(PDFName.Annots, PDFArray);
    if (removeAnnotations) {
      if (page.node.delete(PDFName.Annots)) removed += 1;
    } else {
      removed += cleanPageAnnotations(annots, removeAnnotations);
    }
    notifyProgress(onProgress, { phase: 'working', label: `Cleaned page ${i + 1} of ${pages.length}.`, current: i + 1, total: pages.length });
  }

  notifyProgress(onProgress, { phase: 'working', label: 'Dropping unreferenced active objects…', current: 3, total: 4 });
  removed += deleteDangerousIndirectObjects(doc);

  notifyProgress(onProgress, { phase: 'saving', label: removed > 0 ? `Removed ${removed} risky entries. Saving…` : 'No risky entries found. Saving clean copy…', current: 4, total: 4 });
  return doc.save({ rewrite: true, useObjectStreams: true, updateFieldAppearances: false });
}

export async function protectPdf(bytes: Uint8Array, options: ProtectOptions, onProgress?: ProgressCallback): Promise<Uint8Array> {
  const userPassword = String(options.userPassword ?? '').trim();
  if (!userPassword) throw new Error('Enter a password to protect this PDF');
  const ownerPassword = String(options.ownerPassword ?? '').trim() || userPassword;

  notifyProgress(onProgress, { phase: 'loading', label: 'Opening PDF…' });
  const doc = await PDFDocument.load(bytes, { ignoreEncryption: true });
  notifyProgress(onProgress, { phase: 'working', label: 'Encrypting PDF…' });
  doc.encrypt({
    userPassword,
    ownerPassword,
    permissions: {
      printing: options.allowPrinting ? 'highResolution' : false,
      copying: Boolean(options.allowCopying),
      contentAccessibility: Boolean(options.allowCopying),
      modifying: Boolean(options.allowModifying),
      annotating: Boolean(options.allowModifying),
      fillingForms: Boolean(options.allowModifying),
      documentAssembly: Boolean(options.allowModifying),
    },
  });
  notifyProgress(onProgress, { phase: 'saving', label: 'Saving protected PDF…' });
  return doc.save({ rewrite: true });
}

export async function unlockPdf(bytes: Uint8Array, password: string, onProgress?: ProgressCallback): Promise<Uint8Array> {
  const cleanPassword = String(password ?? '').trim();
  if (!cleanPassword) throw new Error('Enter the password for this PDF');

  notifyProgress(onProgress, { phase: 'loading', label: 'Opening encrypted PDF…' });
  const doc = await PDFDocument.load(bytes, { password: cleanPassword, updateMetadata: false });
  notifyProgress(onProgress, { phase: 'saving', label: 'Saving unlocked PDF…' });
  return doc.save({ rewrite: true });
}
