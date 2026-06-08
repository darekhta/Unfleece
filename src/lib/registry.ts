/**
 * The single source of truth for every Unfleece tool. Declarative only (no
 * heavy imports) so it can be consumed at build time by Astro to generate one
 * static, indexable page per tool, and at runtime by the UI to render the tool.
 */

export type Category = 'organize' | 'edit' | 'convert' | 'optimize' | 'forms' | 'security';

export interface CategoryMeta {
  label: string;
  order: number;
  /** CSS chip modifier class ('' for the default emerald). */
  catClass: string;
  /** CSS custom-property reference for the category hue. */
  hue: string;
  descriptor: string;
}

export const CATEGORIES: Record<Category, CategoryMeta> = {
  organize: { label: 'Organize', order: 1, catClass: '', hue: 'var(--cat-organize)', descriptor: 'Reorder, merge, split, rotate' },
  convert: { label: 'Convert', order: 2, catClass: 'cat-convert', hue: 'var(--cat-convert)', descriptor: 'Move between PDF, image and text' },
  edit: { label: 'Edit', order: 3, catClass: 'cat-edit', hue: 'var(--cat-edit)', descriptor: 'Stamp, mark up and sign pages' },
  optimize: { label: 'Optimize', order: 4, catClass: 'cat-optimize', hue: 'var(--cat-optimize)', descriptor: 'Make files smaller' },
  forms: { label: 'Forms', order: 5, catClass: 'cat-forms', hue: 'var(--cat-forms)', descriptor: 'Detect and fill form fields' },
  security: { label: 'Security & Privacy', order: 6, catClass: 'cat-security', hue: 'var(--cat-security)', descriptor: 'Strip what you don’t want to share' },
};

type OptionBase = {
  name: string;
  label: string;
  showWhen?: { name: string; value: string | number | boolean };
};

export type OptionField =
  | (OptionBase & { type: 'text'; default?: string; placeholder?: string; help?: string; required?: boolean })
  | (OptionBase & { type: 'pages'; default?: string; placeholder?: string; help?: string })
  | (OptionBase & { type: 'number'; default?: number; min?: number; max?: number; step?: number })
  | (OptionBase & { type: 'select'; default: string; options: { value: string; label: string }[] })
  | (OptionBase & { type: 'checkbox'; default?: boolean });

export type Engine = 'worker' | 'browser';
export type OutputKind = 'pdf' | 'zip' | 'text' | 'image' | 'docx' | 'xlsx' | 'pptx' | 'epub';

export interface Tool {
  id: string;
  slug: string;
  name: string;
  category: Category;
  /** Icon name from src/lib/icons.ts. */
  icon: string;
  tagline: string;
  description: string;
  /** HTML input `accept`. */
  accept: string;
  /** Accept multiple input files. */
  multiple: boolean;
  output: OutputKind;
  engine: Engine;
  options?: OptionField[];
  /** Special interactive UIs handled by dedicated components. */
  special?: 'fill-form' | 'sign' | 'page-numbers' | 'crop' | 'watermark' | 'redact';
  /** Bento layout hints. */
  feature?: boolean;
  wide?: boolean;
  /** Powered by the Rust→WASM engine (shows a badge). */
  wasm?: boolean;
}

const PDF = 'application/pdf';
const IMG = [
  'image/png', 'image/jpeg', 'image/webp', 'image/avif', 'image/jxl',
  'image/tiff', 'image/bmp', 'image/gif', 'image/x-icon', 'image/vnd.microsoft.icon',
  'image/vnd.adobe.photoshop', 'image/heic', 'image/heif', 'image/jp2',
  '.jxl', '.tif', '.tiff', '.psd', '.psb', '.bmp', '.gif', '.ico', '.cur', '.heic', '.heif', '.jp2', '.j2k', '.tga', '.pcx',
].join(',');

export const TOOLS: Tool[] = [
  // ---------------- Organize ----------------
  {
    id: 'merge', slug: 'merge-pdf', name: 'Merge PDF', category: 'organize', icon: 'merge', feature: true,
    tagline: 'Combine PDFs into one document.',
    description: 'Combine multiple PDF files into a single document, in the order you add them.',
    accept: PDF, multiple: true, output: 'pdf', engine: 'worker',
  },
  {
    id: 'split', slug: 'split-pdf', name: 'Split PDF', category: 'organize', icon: 'split', feature: true,
    tagline: 'Split one PDF into several files.',
    description: 'Split one PDF into separate files — one per page, or by custom page ranges.',
    accept: PDF, multiple: false, output: 'zip', engine: 'worker',
    options: [
      { name: 'mode', label: 'Split mode', type: 'select', default: 'each',
        options: [
          { value: 'each', label: 'Every page into its own file' },
          { value: 'ranges', label: 'By page ranges' },
        ] },
      { name: 'ranges', label: 'Ranges (one file per range)', type: 'text', placeholder: 'e.g. 1-3, 4-6, 7' },
    ],
  },
  {
    id: 'extract-pages', slug: 'extract-pdf-pages', name: 'Extract pages', category: 'organize', icon: 'extract',
    tagline: 'Pull out just the pages you need.',
    description: 'Create a new PDF containing only the pages you choose.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [{ name: 'pages', label: 'Pages to extract', type: 'pages', placeholder: 'e.g. 1-3, 5', help: 'Comma-separated pages and ranges (1-based).' }],
  },
  {
    id: 'remove-pages', slug: 'remove-pdf-pages', name: 'Remove pages', category: 'organize', icon: 'remove',
    tagline: 'Delete the pages you don’t want.',
    description: 'Remove the pages you select and keep the rest.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [{ name: 'pages', label: 'Pages to remove', type: 'pages', placeholder: 'e.g. 2, 5-7' }],
  },
  {
    id: 'reorder', slug: 'reorder-pdf-pages', name: 'Reorder pages', category: 'organize', icon: 'reorder',
    tagline: 'Put pages into a new order.',
    description: 'Reorder the pages of a PDF into a new sequence.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [{ name: 'order', label: 'New order', type: 'text', placeholder: 'e.g. 3,1,2', help: 'Every page number exactly once.' }],
  },
  {
    id: 'rotate', slug: 'rotate-pdf', name: 'Rotate PDF', category: 'organize', icon: 'rotate',
    tagline: 'Turn pages the right way up.',
    description: 'Rotate all or selected pages by 90°, 180° or 270°. Lossless.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [
      { name: 'angle', label: 'Angle', type: 'select', default: '90',
        options: [{ value: '90', label: '90° clockwise' }, { value: '180', label: '180°' }, { value: '270', label: '270° (90° ccw)' }] },
      { name: 'pages', label: 'Pages (blank = all)', type: 'pages', placeholder: 'e.g. 1, 3-4' },
    ],
  },
  {
    id: 'n-up', slug: 'n-up-pdf', name: 'N-up per sheet', category: 'organize', icon: 'nup',
    tagline: 'Print several pages per sheet.',
    description: 'Place several source pages onto each A4 sheet — great for handouts.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [{ name: 'perSheet', label: 'Pages per sheet', type: 'select', default: '4',
      options: ['2', '4', '6', '9'].map((v) => ({ value: v, label: `${v} per sheet` })) }],
  },
  {
    id: 'booklet', slug: 'booklet-pdf', name: 'Booklet', category: 'organize', icon: 'booklet',
    tagline: 'Impose pages for folded booklet printing.',
    description: 'Reorder and place pages two-up on landscape sheets so they fold into a booklet.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [
      { name: 'pageSize', label: 'Sheet size', type: 'select', default: 'a4',
        options: [{ value: 'a4', label: 'A4' }, { value: 'letter', label: 'Letter' }] },
      { name: 'binding', label: 'Binding edge', type: 'select', default: 'left',
        options: [{ value: 'left', label: 'Left-bound' }, { value: 'right', label: 'Right-bound' }] },
      { name: 'margin', label: 'Margin (pt)', type: 'number', default: 18, min: 0, max: 120, step: 1 },
    ],
  },
  {
    id: 'compare-pdf', slug: 'compare-pdfs', name: 'Compare PDFs', category: 'organize', icon: 'comparePdf', feature: true,
    tagline: 'Find visual differences between two PDFs.',
    description: 'Render two PDFs locally and produce a page-by-page visual difference report.',
    accept: PDF, multiple: true, output: 'text', engine: 'browser',
    options: [
      { name: 'scale', label: 'Render scale', type: 'number', default: 0.75, min: 0.25, max: 2, step: 0.25 },
      { name: 'threshold', label: 'Pixel threshold', type: 'number', default: 24, min: 0, max: 255, step: 1 },
    ],
  },

  // ---------------- Convert ----------------
  {
    id: 'images-to-pdf', slug: 'images-to-pdf', name: 'Images → PDF', category: 'convert', icon: 'imgToPdf', feature: true,
    tagline: 'Turn JPGs and PNGs into a PDF.',
    description: 'Turn one or more JPG/PNG images into a PDF, one image per page.',
    accept: IMG, multiple: true, output: 'pdf', engine: 'worker',
    options: [
      { name: 'pageSize', label: 'Page size', type: 'select', default: 'fit',
        options: [{ value: 'fit', label: 'Fit page to image' }, { value: 'a4', label: 'A4' }, { value: 'letter', label: 'Letter' }] },
      { name: 'margin', label: 'Margin (pt, fixed sizes)', type: 'number', default: 24, min: 0, max: 200 },
    ],
  },
  {
    id: 'pdf-to-jpg', slug: 'pdf-to-jpg', name: 'PDF → JPG', category: 'convert', icon: 'pdfToJpg',
    tagline: 'Save each page as a JPG.',
    description: 'Render each PDF page to a JPG image and download them as a ZIP.',
    accept: PDF, multiple: false, output: 'zip', engine: 'browser',
    options: [
      { name: 'scale', label: 'Resolution (scale)', type: 'number', default: 2, min: 1, max: 4, step: 0.5 },
      { name: 'quality', label: 'JPG quality', type: 'number', default: 0.85, min: 0.3, max: 1, step: 0.05 },
    ],
  },
  {
    id: 'pdf-to-png', slug: 'pdf-to-png', name: 'PDF → PNG', category: 'convert', icon: 'pdfToPng',
    tagline: 'Save each page as a PNG.',
    description: 'Render each PDF page to a lossless PNG image and download them as a ZIP.',
    accept: PDF, multiple: false, output: 'zip', engine: 'browser',
    options: [{ name: 'scale', label: 'Resolution (scale)', type: 'number', default: 2, min: 1, max: 4, step: 0.5 }],
  },
  {
    id: 'pdf-to-text', slug: 'pdf-to-text', name: 'PDF → Text', category: 'convert', icon: 'pdfToText',
    tagline: 'Pull selectable text out of a PDF.',
    description: 'Extract the selectable text from a PDF. (Scanned/image PDFs have no text to extract.)',
    accept: PDF, multiple: false, output: 'text', engine: 'browser',
  },
  {
    id: 'pdf-to-epub', slug: 'pdf-to-epub', name: 'PDF → EPUB', category: 'convert', icon: 'pdfToEpub',
    tagline: 'Create an EPUB from selectable PDF text.',
    description: 'Build a reflowable EPUB from selectable PDF text, or a fixed-layout EPUB from rendered pages for scans and complex layouts.',
    accept: PDF, multiple: false, output: 'epub', engine: 'browser',
    options: [
      { name: 'mode', label: 'EPUB mode', type: 'select', default: 'reflowable',
        options: [
          { value: 'reflowable', label: 'Reflowable text EPUB' },
          { value: 'fixed', label: 'Fixed-layout page images' },
        ] },
      { name: 'title', label: 'Title', type: 'text', placeholder: 'Defaults to file name' },
      { name: 'author', label: 'Author', type: 'text', placeholder: 'Optional' },
      { name: 'language', label: 'Language', type: 'select', default: 'en',
        options: [
          { value: 'ar', label: 'Arabic' },
          { value: 'zh', label: 'Chinese' },
          { value: 'nl', label: 'Dutch' },
          { value: 'en', label: 'English' },
          { value: 'fr', label: 'French' },
          { value: 'de', label: 'German' },
          { value: 'hi', label: 'Hindi' },
          { value: 'it', label: 'Italian' },
          { value: 'ja', label: 'Japanese' },
          { value: 'ko', label: 'Korean' },
          { value: 'pl', label: 'Polish' },
          { value: 'pt', label: 'Portuguese' },
          { value: 'ru', label: 'Russian' },
          { value: 'es', label: 'Spanish' },
          { value: 'tr', label: 'Turkish' },
          { value: 'uk', label: 'Ukrainian' },
        ] },
      { name: 'removeHeadersFooters', label: 'Remove repeated headers/footers', type: 'checkbox', default: true, showWhen: { name: 'mode', value: 'reflowable' } },
      { name: 'unwrapParagraphs', label: 'Unwrap hard line breaks', type: 'checkbox', default: true, showWhen: { name: 'mode', value: 'reflowable' } },
      { name: 'repairHyphenation', label: 'Repair line-end hyphenation', type: 'checkbox', default: true, showWhen: { name: 'mode', value: 'reflowable' } },
      { name: 'imageScale', label: 'Render scale', type: 'number', default: 1.5, min: 1, max: 3, step: 0.25, showWhen: { name: 'mode', value: 'fixed' } },
      { name: 'imageQuality', label: 'JPG quality', type: 'number', default: 0.82, min: 0.4, max: 0.95, step: 0.05, showWhen: { name: 'mode', value: 'fixed' } },
    ],
  },
  {
    id: 'pdf-to-docx', slug: 'extract-pdf-to-word', name: 'Extract to Word', category: 'convert', icon: 'pdfToDocx',
    tagline: 'Save selectable PDF text as DOCX.',
    description: 'Extract selectable text into a simple Word document. It preserves text, not the original layout.',
    accept: PDF, multiple: false, output: 'docx', engine: 'browser',
  },
  {
    id: 'pdf-to-excel', slug: 'extract-pdf-to-excel', name: 'Extract to Excel', category: 'convert', icon: 'pdfToExcel',
    tagline: 'Turn selectable rows into a spreadsheet.',
    description: 'Best-effort extraction of selectable text rows into an Excel workbook. Scans need OCR first.',
    accept: PDF, multiple: false, output: 'xlsx', engine: 'browser',
  },
  {
    id: 'pdf-to-pptx', slug: 'extract-pdf-to-powerpoint', name: 'Extract to PowerPoint', category: 'convert', icon: 'pdfToPptx',
    tagline: 'Turn PDF text into simple slides.',
    description: 'Extract selectable PDF text into a simple PowerPoint deck, one slide per page. It does not recreate the original layout.',
    accept: PDF, multiple: false, output: 'pptx', engine: 'browser',
  },
  {
    id: 'ocr-pdf', slug: 'ocr-searchable-pdf', name: 'OCR searchable PDF', category: 'convert', icon: 'search', wasm: true,
    tagline: 'Add selectable text to scanned PDFs.',
    description: 'Run Tesseract OCR locally and add an invisible searchable text layer over each page. English model included.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'browser',
    options: [
      { name: 'scale', label: 'OCR render scale', type: 'number', default: 2.5, min: 1, max: 4, step: 0.5 },
      { name: 'minConfidence', label: 'Minimum confidence', type: 'number', default: 0.35, min: 0, max: 1, step: 0.05 },
    ],
  },
  {
    id: 'pdfa', slug: 'export-pdfa', name: 'PDF/A export', category: 'convert', icon: 'pdfToText', wasm: true,
    tagline: 'Create an archival PDF/A-2b copy.',
    description: 'Render pages locally and rebuild them as a visual PDF/A-2b file with the Rust krilla engine. Text becomes non-selectable.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'browser',
    options: [
      { name: 'scale', label: 'Render scale', type: 'number', default: 1.5, min: 1, max: 3, step: 0.25 },
    ],
  },
  {
    id: 'image-convert', slug: 'convert-image', name: 'Convert image', category: 'convert', icon: 'convertImg',
    tagline: 'Switch between PNG, JPG, WebP, AVIF, JPEG XL and TIFF-style inputs.',
    description: 'Convert browser images and formats like TIFF, PSD, BMP, GIF and ICO into PNG, JPG, WebP, AVIF or JPEG XL, fully in your browser.',
    accept: IMG, multiple: false, output: 'image', engine: 'browser',
    options: [
      { name: 'format', label: 'Convert to', type: 'select', default: 'image/webp',
        options: [
          { value: 'image/webp', label: 'WebP' },
          { value: 'image/avif', label: 'AVIF' },
          { value: 'image/jxl', label: 'JPEG XL' },
          { value: 'image/jpeg', label: 'JPG' },
          { value: 'image/png', label: 'PNG' },
        ] },
      { name: 'quality', label: 'Quality (lossy formats)', type: 'number', default: 0.85, min: 0.3, max: 1, step: 0.05 },
    ],
  },
  {
    id: 'html-to-pdf', slug: 'html-markdown-to-pdf', name: 'HTML / Markdown → PDF', category: 'convert', icon: 'htmlToPdf',
    tagline: 'Turn a text document into a clean PDF.',
    description: 'Convert an HTML, Markdown or plain-text file into a readable PDF. This is text-focused, not a browser layout engine.',
    accept: '.html,.htm,.md,.markdown,.txt,text/html,text/markdown,text/plain', multiple: false, output: 'pdf', engine: 'browser',
    options: [
      { name: 'title', label: 'Document title', type: 'text', placeholder: 'Defaults to file name' },
      { name: 'pageSize', label: 'Page size', type: 'select', default: 'a4',
        options: [{ value: 'a4', label: 'A4' }, { value: 'letter', label: 'Letter' }] },
      { name: 'fontSize', label: 'Font size', type: 'number', default: 11, min: 8, max: 18, step: 1 },
    ],
  },

  // ---------------- Edit ----------------
  {
    id: 'page-numbers', slug: 'add-page-numbers', name: 'Page numbers', category: 'edit', icon: 'pageNumbers', special: 'page-numbers',
    tagline: 'Stamp page numbers onto a PDF.',
    description: 'Add page numbers with customizable format and position.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [
      { name: 'format', label: 'Format', type: 'text', default: '{n}', help: '{n} = page number, {total} = total pages' },
      { name: 'position', label: 'Position', type: 'select', default: 'bottom-center',
        options: ['bottom-center', 'bottom-right', 'bottom-left', 'top-center', 'top-right', 'top-left'].map((v) => ({ value: v, label: v.replace('-', ' ') })) },
      { name: 'fontSize', label: 'Font size', type: 'number', default: 10, min: 6, max: 48 },
      { name: 'margin', label: 'Margin (pt)', type: 'number', default: 24, min: 0, max: 120 },
    ],
  },
  {
    id: 'bates', slug: 'bates-numbering', name: 'Bates numbering', category: 'edit', icon: 'bates',
    tagline: 'Add legal-style Bates numbers.',
    description: 'Stamp sequential Bates numbers on each page with a prefix, padded number and position.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [
      { name: 'prefix', label: 'Prefix', type: 'text', default: 'BATES-' },
      { name: 'startAt', label: 'Start number', type: 'number', default: 1, min: 0, max: 999999999, step: 1 },
      { name: 'padTo', label: 'Digits', type: 'number', default: 6, min: 1, max: 12, step: 1 },
      { name: 'position', label: 'Position', type: 'select', default: 'bottom-right',
        options: ['bottom-right', 'bottom-center', 'bottom-left', 'top-right', 'top-center', 'top-left'].map((v) => ({ value: v, label: v.replace('-', ' ') })) },
      { name: 'fontSize', label: 'Font size', type: 'number', default: 9, min: 6, max: 24 },
      { name: 'margin', label: 'Margin (pt)', type: 'number', default: 24, min: 0, max: 120 },
    ],
  },
  {
    id: 'watermark', slug: 'watermark-pdf', name: 'Watermark', category: 'edit', icon: 'watermark', special: 'watermark',
    tagline: 'Add a text watermark to every page.',
    description: 'Stamp a diagonal text watermark across every page.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [
      { name: 'text', label: 'Watermark text', type: 'text', default: 'CONFIDENTIAL', required: true },
      { name: 'fontSize', label: 'Font size', type: 'number', default: 50, min: 8, max: 200 },
      { name: 'opacity', label: 'Opacity', type: 'number', default: 0.25, min: 0.05, max: 1, step: 0.05 },
      { name: 'angle', label: 'Angle', type: 'number', default: 45, min: -90, max: 90 },
    ],
  },
  {
    id: 'crop', slug: 'crop-pdf', name: 'Crop', category: 'edit', icon: 'crop', special: 'crop',
    tagline: 'Trim the margins off your pages.',
    description: 'Trim margins from every page by setting the crop box.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [
      { name: 'top', label: 'Top (pt)', type: 'number', default: 0, min: 0 },
      { name: 'right', label: 'Right (pt)', type: 'number', default: 0, min: 0 },
      { name: 'bottom', label: 'Bottom (pt)', type: 'number', default: 0, min: 0 },
      { name: 'left', label: 'Left (pt)', type: 'number', default: 0, min: 0 },
    ],
  },
  {
    id: 'auto-crop', slug: 'auto-crop-pdf', name: 'Auto-crop margins', category: 'edit', icon: 'autoCrop',
    tagline: 'Detect and trim white page margins.',
    description: 'Render each page locally, detect the non-white content bounds, and set per-page crop boxes with padding.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'browser',
    options: [
      { name: 'scale', label: 'Scan scale', type: 'number', default: 1.5, min: 0.5, max: 3, step: 0.25 },
      { name: 'tolerance', label: 'White tolerance', type: 'number', default: 12, min: 0, max: 80, step: 1 },
      { name: 'padding', label: 'Padding kept (pt)', type: 'number', default: 6, min: 0, max: 144, step: 1 },
    ],
  },
  {
    id: 'metadata', slug: 'edit-pdf-metadata', name: 'Edit metadata', category: 'edit', icon: 'metadata',
    tagline: 'Change the title, author and more.',
    description: 'View and overwrite document metadata (title, author, subject, keywords).',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [
      { name: 'title', label: 'Title', type: 'text' },
      { name: 'author', label: 'Author', type: 'text' },
      { name: 'subject', label: 'Subject', type: 'text' },
      { name: 'keywords', label: 'Keywords (comma-separated)', type: 'text' },
      { name: 'creator', label: 'Creator', type: 'text' },
    ],
  },
  {
    id: 'sign', slug: 'sign-pdf', name: 'Sign', category: 'edit', icon: 'sign', feature: true, special: 'sign',
    tagline: 'Draw a signature and place it.',
    description: 'Draw a signature and place it on the page. Electronic mark, not a cryptographic signature.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'browser',
  },

  // ---------------- Optimize ----------------
  {
    id: 'optimize', slug: 'optimize-pdf', name: 'Optimize', category: 'optimize', icon: 'optimize', feature: true, wasm: true,
    tagline: 'Shrink the file without quality loss.',
    description:
      'Losslessly shrink a PDF by removing unused objects and compressing streams — text stays selectable.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'browser',
  },
  {
    id: 'compress', slug: 'compress-pdf', name: 'Compress', category: 'optimize', icon: 'compress', wasm: true,
    tagline: 'Shrink image-heavy PDFs with Ghostscript.',
    description: 'Use Ghostscript pdfwrite locally for lossy PDF compression. Text stays selectable when Ghostscript can rewrite the file; the raster fallback makes text non-selectable.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'browser',
    options: [
      { name: 'engine', label: 'Engine', type: 'select', default: 'ghostscript',
        options: [
          { value: 'ghostscript', label: 'Ghostscript pdfwrite' },
          { value: 'raster', label: 'Raster fallback' },
        ] },
      { name: 'preset', label: 'Ghostscript preset', type: 'select', default: 'ebook',
        options: [
          { value: 'screen', label: 'Screen (smallest)' },
          { value: 'ebook', label: 'Ebook (balanced)' },
          { value: 'printer', label: 'Printer' },
          { value: 'prepress', label: 'Prepress' },
          { value: 'default', label: 'Default' },
        ] },
      { name: 'scale', label: 'Raster render scale', type: 'number', default: 1.5, min: 0.5, max: 3, step: 0.25 },
      { name: 'quality', label: 'Raster JPG quality', type: 'number', default: 0.6, min: 0.2, max: 0.95, step: 0.05 },
    ],
  },
  {
    id: 'compress-image', slug: 'compress-image', name: 'Compress image', category: 'optimize', icon: 'compressImg',
    tagline: 'Shrink PNG, JPG, WebP, AVIF, JPEG XL and TIFF-style inputs.',
    description: 'Re-encode and optionally resize browser images plus TIFF, PSD, BMP, GIF and ICO locally. Choose WebP, AVIF, JPEG XL or JPG for smaller lossy output.',
    accept: IMG, multiple: false, output: 'image', engine: 'browser',
    options: [
      { name: 'format', label: 'Output format', type: 'select', default: 'image/webp',
        options: [
          { value: 'image/webp', label: 'WebP' },
          { value: 'image/avif', label: 'AVIF' },
          { value: 'image/jxl', label: 'JPEG XL' },
          { value: 'image/jpeg', label: 'JPG' },
          { value: 'image/png', label: 'PNG' },
        ] },
      { name: 'quality', label: 'Quality (lossy formats)', type: 'number', default: 0.72, min: 0.2, max: 1, step: 0.05 },
      { name: 'maxDimension', label: 'Max width/height px (0 = original)', type: 'number', default: 0, min: 0, max: 12000, step: 100 },
    ],
  },

  // ---------------- Forms ----------------
  {
    id: 'fill-form', slug: 'fill-pdf-form', name: 'Fill form', category: 'forms', icon: 'fillForm', feature: true, special: 'fill-form',
    tagline: 'Fill in detected form fields.',
    description: 'Detect AcroForm fields and fill them in your browser.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
  },
  {
    id: 'flatten', slug: 'flatten-pdf', name: 'Flatten', category: 'forms', icon: 'flatten', feature: true,
    tagline: 'Bake form fields into the page.',
    description: 'Flatten form fields and annotations into static page content.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
  },

  // ---------------- Security & Privacy ----------------
  {
    id: 'remove-metadata', slug: 'remove-pdf-metadata', name: 'Remove metadata', category: 'security', icon: 'removeMeta',
    tagline: 'Strip hidden info before sharing.',
    description: 'Remove document metadata (title, author, producer, …) so it isn’t shared by accident.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
  },
  {
    id: 'protect', slug: 'protect-pdf', name: 'Protect PDF', category: 'security', icon: 'protect', feature: true,
    tagline: 'Add a password to open a PDF.',
    description: 'Encrypt a PDF locally with a user password and optional owner password/permissions.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [
      { name: 'userPassword', label: 'Open password', type: 'text', required: true },
      { name: 'ownerPassword', label: 'Owner password (optional)', type: 'text', help: 'Leave blank to reuse the open password.' },
      { name: 'allowPrinting', label: 'Allow printing', type: 'checkbox', default: true },
      { name: 'allowCopying', label: 'Allow copying text/images', type: 'checkbox', default: false },
      { name: 'allowModifying', label: 'Allow editing/forms/assembly', type: 'checkbox', default: false },
    ],
  },
  {
    id: 'unlock', slug: 'unlock-pdf', name: 'Unlock PDF', category: 'security', icon: 'unlock', feature: true,
    tagline: 'Remove password encryption when you know the password.',
    description: 'Open an encrypted PDF with its password and save an unlocked copy locally.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [{ name: 'password', label: 'PDF password', type: 'text', required: true }],
  },
  {
    id: 'sanitize', slug: 'sanitize-pdf', name: 'Sanitize PDF', category: 'security', icon: 'sanitize',
    tagline: 'Remove active and hidden PDF extras.',
    description: 'Strip metadata, document scripts, embedded files, page actions and optional annotations/forms before sharing.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'worker',
    options: [
      { name: 'removeAnnotations', label: 'Remove annotations', type: 'checkbox', default: true },
      { name: 'removeForms', label: 'Remove form fields', type: 'checkbox', default: true },
    ],
  },
  {
    id: 'redact', slug: 'redact-pdf', name: 'Redact PDF', category: 'security', icon: 'redact', special: 'redact', feature: true,
    tagline: 'Burn in redaction boxes.',
    description: 'Cover selected areas and rebuild pages as image-only PDF pages so hidden text is not left underneath.',
    accept: PDF, multiple: false, output: 'pdf', engine: 'browser',
  },
];

export const TOOLS_BY_SLUG: Record<string, Tool> = Object.fromEntries(TOOLS.map((t) => [t.slug, t]));
export const TOOLS_BY_ID: Record<string, Tool> = Object.fromEntries(TOOLS.map((t) => [t.id, t]));

export function toolsByCategory(): { category: Category; meta: CategoryMeta; tools: Tool[] }[] {
  return (Object.keys(CATEGORIES) as Category[])
    .sort((a, b) => CATEGORIES[a].order - CATEGORIES[b].order)
    .map((category) => ({ category, meta: CATEGORIES[category], tools: TOOLS.filter((t) => t.category === category) }))
    .filter((g) => g.tools.length > 0);
}
