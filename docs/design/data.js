/* ============================================================
   UNFLEECE — Tool registry (single source of truth)
   ============================================================ */

const CATEGORIES = [
  { id: 'organize', label: 'Organize', cat: '', hue: 'var(--cat-organize)', descriptor: 'Reorder, merge, split, rotate' },
  { id: 'convert',  label: 'Convert',  cat: 'cat-convert',  hue: 'var(--cat-convert)',  descriptor: 'Move between PDF, image and text' },
  { id: 'edit',     label: 'Edit',     cat: 'cat-edit',     hue: 'var(--cat-edit)',     descriptor: 'Stamp, mark up and sign pages' },
  { id: 'optimize', label: 'Optimize', cat: 'cat-optimize', hue: 'var(--cat-optimize)', descriptor: 'Make files smaller' },
  { id: 'forms',    label: 'Forms',    cat: 'cat-forms',    hue: 'var(--cat-forms)',    descriptor: 'Detect and fill form fields' },
  { id: 'security', label: 'Security & Privacy', cat: 'cat-security', hue: 'var(--cat-security)', descriptor: 'Strip what you don\u2019t want to share' },
];

const TOOLS = [
  // Organize (9)
  { slug: 'merge-pdf', name: 'Merge PDF', icon: 'merge', cat: 'organize', tag: 'Combine PDFs into one document.', desc: 'Combine multiple PDF files into a single document, in the order you add them.', feature: true },
  { slug: 'split-pdf', name: 'Split PDF', icon: 'split', cat: 'organize', tag: 'Split one PDF into several files.' },
  { slug: 'extract-pages', name: 'Extract pages', icon: 'extract', cat: 'organize', tag: 'Pull out just the pages you need.' },
  { slug: 'remove-pages', name: 'Remove pages', icon: 'remove', cat: 'organize', tag: 'Delete the pages you don\u2019t want.' },
  { slug: 'reorder', name: 'Reorder pages', icon: 'reorder', cat: 'organize', tag: 'Drag pages into a new order.' },
  { slug: 'rotate', name: 'Rotate PDF', icon: 'rotate', cat: 'organize', tag: 'Turn pages the right way up.' },
  { slug: 'n-up', name: 'N-up per sheet', icon: 'nup', cat: 'organize', tag: 'Print several pages per sheet.' },
  { slug: 'booklet-pdf', name: 'Booklet', icon: 'booklet', cat: 'organize', tag: 'Impose pages for folded booklet printing.' },
  { slug: 'compare-pdfs', name: 'Compare PDFs', icon: 'comparePdf', cat: 'organize', tag: 'Find visual differences between two PDFs.' },

  // Convert (12)
  { slug: 'images-to-pdf', name: 'Images \u2192 PDF', icon: 'imgToPdf', cat: 'convert', tag: 'Turn JPGs and PNGs into a PDF.', desc: 'Combine your images into a PDF \u2014 one image per page.', feature: true },
  { slug: 'pdf-to-jpg', name: 'PDF \u2192 JPG', icon: 'pdfToJpg', cat: 'convert', tag: 'Save each page as a JPG.' },
  { slug: 'pdf-to-png', name: 'PDF \u2192 PNG', icon: 'pdfToPng', cat: 'convert', tag: 'Save each page as a PNG.' },
  { slug: 'pdf-to-text', name: 'PDF \u2192 Text', icon: 'pdfToText', cat: 'convert', tag: 'Pull selectable text out of a PDF.' },
  { slug: 'pdf-to-epub', name: 'PDF \u2192 EPUB', icon: 'pdfToEpub', cat: 'convert', tag: 'Create a reflowable or fixed-layout EPUB.' },
  { slug: 'extract-pdf-to-word', name: 'Extract to Word', icon: 'pdfToDocx', cat: 'convert', tag: 'Save selectable PDF text as DOCX.' },
  { slug: 'extract-pdf-to-excel', name: 'Extract to Excel', icon: 'pdfToExcel', cat: 'convert', tag: 'Turn selectable rows into a spreadsheet.' },
  { slug: 'extract-pdf-to-powerpoint', name: 'Extract to PowerPoint', icon: 'pdfToPptx', cat: 'convert', tag: 'Create one text slide per PDF page.' },
  { slug: 'ocr-searchable-pdf', name: 'OCR searchable PDF', icon: 'search', cat: 'convert', tag: 'Add selectable text to scanned PDFs.', wasm: true },
  { slug: 'export-pdfa', name: 'PDF/A export', icon: 'pdfToText', cat: 'convert', tag: 'Create an archival PDF/A-2b copy.', wasm: true },
  { slug: 'convert-image', name: 'Convert image', icon: 'convertImg', cat: 'convert', tag: 'Switch between PNG, JPG, WebP, AVIF, JPEG XL and TIFF-style inputs.' },
  { slug: 'html-markdown-to-pdf', name: 'HTML / Markdown \u2192 PDF', icon: 'htmlToPdf', cat: 'convert', tag: 'Turn a text document into a clean PDF.' },

  // Edit (7)
  { slug: 'page-numbers', name: 'Page numbers', icon: 'pageNumbers', cat: 'edit', tag: 'Stamp page numbers onto a PDF.' },
  { slug: 'bates-numbering', name: 'Bates numbering', icon: 'bates', cat: 'edit', tag: 'Add legal-style Bates numbers.' },
  { slug: 'watermark', name: 'Watermark', icon: 'watermark', cat: 'edit', tag: 'Add a text watermark to every page.' },
  { slug: 'crop', name: 'Crop', icon: 'crop', cat: 'edit', tag: 'Trim the margins off your pages.' },
  { slug: 'auto-crop-pdf', name: 'Auto-crop margins', icon: 'autoCrop', cat: 'edit', tag: 'Detect and trim white page margins.' },
  { slug: 'edit-metadata', name: 'Edit metadata', icon: 'metadata', cat: 'edit', tag: 'Change the title, author and more.' },
  { slug: 'sign', name: 'Sign', icon: 'sign', cat: 'edit', tag: 'Draw a signature and place it.', feature: true, desc: 'Draw a signature on a white pad, then place it on the page you choose.' },

  // Optimize (3)
  { slug: 'optimize', name: 'Optimize', icon: 'optimize', cat: 'optimize', tag: 'Shrink the file without quality loss.', feature: true, wasm: true, desc: 'Losslessly rewrite the PDF to a smaller size. Runs on a Rust\u2192WASM engine.' },
  { slug: 'compress', name: 'Compress', icon: 'compress', cat: 'optimize', tag: 'Shrink image-heavy PDFs with Ghostscript.', wasm: true },
  { slug: 'compress-image', name: 'Compress image', icon: 'compressImg', cat: 'optimize', tag: 'Shrink PNG, JPG, WebP, AVIF, JPEG XL and TIFF-style inputs.' },

  // Forms (2)
  { slug: 'fill-form', name: 'Fill form', icon: 'fillForm', cat: 'forms', tag: 'Fill in detected form fields.', feature: true, desc: 'Upload a PDF, we detect its AcroForm fields and give you one input per field.' },
  { slug: 'flatten', name: 'Flatten', icon: 'flatten', cat: 'forms', tag: 'Bake form fields into the page.' },

  // Security (5)
  { slug: 'remove-metadata', name: 'Remove metadata', icon: 'removeMeta', cat: 'security', tag: 'Strip hidden info before sharing.', wide: true, desc: 'Remove document metadata (title, author, producer\u2026) so it isn\u2019t shared by accident.' },
  { slug: 'protect-pdf', name: 'Protect PDF', icon: 'protect', cat: 'security', tag: 'Add a password to open a PDF.' },
  { slug: 'unlock-pdf', name: 'Unlock PDF', icon: 'unlock', cat: 'security', tag: 'Remove password encryption when you know the password.' },
  { slug: 'sanitize-pdf', name: 'Sanitize PDF', icon: 'sanitize', cat: 'security', tag: 'Remove active and hidden PDF extras.', wide: true },
  { slug: 'redact-pdf', name: 'Redact PDF', icon: 'redact', cat: 'security', tag: 'Burn in redaction boxes.', wide: true },
];

function toolsByCat(catId) { return TOOLS.filter(function (t) { return t.cat === catId; }); }
function catMeta(catId) { return CATEGORIES.find(function (c) { return c.id === catId; }); }
