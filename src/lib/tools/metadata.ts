import { loadPdf } from '../util/pdf.js';

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

/** Read document metadata. */
export async function getMetadata(bytes: Uint8Array): Promise<PdfMetadata> {
  const doc = await loadPdf(bytes);
  const kw = doc.getKeywords();
  return {
    title: doc.getTitle() ?? undefined,
    author: doc.getAuthor() ?? undefined,
    subject: doc.getSubject() ?? undefined,
    keywords: kw ? kw.split(/[,;]\s*/).filter(Boolean) : undefined,
    creator: doc.getCreator() ?? undefined,
    producer: doc.getProducer() ?? undefined,
    creationDate: doc.getCreationDate() ?? undefined,
    modificationDate: doc.getModificationDate() ?? undefined,
    pageCount: doc.getPageCount(),
  };
}

export type MetadataUpdate = Partial<Omit<PdfMetadata, 'pageCount'>>;

/** Set (overwrite) the provided metadata fields. */
export async function setMetadata(bytes: Uint8Array, meta: MetadataUpdate): Promise<Uint8Array> {
  const doc = await loadPdf(bytes);
  if (meta.title !== undefined) doc.setTitle(meta.title);
  if (meta.author !== undefined) doc.setAuthor(meta.author);
  if (meta.subject !== undefined) doc.setSubject(meta.subject);
  // pdf-lib joins array entries with a space; store one comma-separated string
  // so the delimiter survives a round-trip through getMetadata().
  if (meta.keywords !== undefined) doc.setKeywords([meta.keywords.join(', ')]);
  if (meta.creator !== undefined) doc.setCreator(meta.creator);
  if (meta.producer !== undefined) doc.setProducer(meta.producer);
  if (meta.creationDate !== undefined) doc.setCreationDate(meta.creationDate);
  if (meta.modificationDate !== undefined) doc.setModificationDate(meta.modificationDate);
  return doc.save();
}

/** Strip all document-info metadata (privacy hygiene). */
export async function stripMetadata(bytes: Uint8Array): Promise<Uint8Array> {
  const doc = await loadPdf(bytes);
  doc.setTitle('');
  doc.setAuthor('');
  doc.setSubject('');
  doc.setKeywords([]);
  doc.setCreator('');
  doc.setProducer('');
  return doc.save();
}
