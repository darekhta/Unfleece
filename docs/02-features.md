# 02 — Feature Catalog

> **Status: shipped.** Unfleece is live at <https://unfleece.com> with **38 working
> tools**, every one of them running fully client-side. This document lists what
> shipped (and on which engine), the honest reframes we made, and the candidate pool
> for future tools.

Feasibility legend (kept for the future-candidates section):
- 🟢 **fully client-side** — pure JS or light WASM, ships freely
- 🟡 **partial / degraded** — works client-side but with honest caveats
- 🔴 **server-needed / skip** — no good client-side path; reframe or drop

---

## Shipped — the 38 live tools

All tools run in the browser; "worker" = Web Worker via Comlink, "browser" = main-thread
pdf.js/Canvas work, "WASM" = the Rust `unfleece-core` engine.

### Organize
| Tool | URL | Engine | Notes |
|---|---|---|---|
| Merge PDF | `/tools/merge-pdf` | Rust→WASM `unfleece-core` (worker) | multi-file, drag-reorderable list |
| Split PDF | `/tools/split-pdf` | Rust→WASM `unfleece-core` (worker) | every-page or custom ranges → ZIP |
| Extract pages | `/tools/extract-pdf-pages` | Rust→WASM `unfleece-core` (worker) | page/range expressions |
| Remove pages | `/tools/remove-pdf-pages` | Rust→WASM `unfleece-core` (worker) | |
| Reorder pages | `/tools/reorder-pdf-pages` | Rust→WASM `unfleece-core` (worker) | explicit order list |
| Rotate PDF | `/tools/rotate-pdf` | Rust→WASM `unfleece-core` | lossless `/Rotate`, per-page selection |
| N-up per sheet | `/tools/n-up-pdf` | Rust→WASM `unfleece-core` (worker) | 2/4/6/9 per A4 sheet |
| Booklet | `/tools/booklet-pdf` | Rust→WASM `unfleece-core` (worker) | two-up folded-spread imposition with blank padding |
| Compare PDFs | `/tools/compare-pdfs` | pdf.js + Canvas (browser) | visual page-by-page report |

### Convert
| Tool | URL | Engine | Notes |
|---|---|---|---|
| Images → PDF | `/tools/images-to-pdf` | Rust→WASM `unfleece-core` (worker) | JPG/PNG, fit/A4/Letter |
| PDF → JPG | `/tools/pdf-to-jpg` | pdf.js + Canvas (browser) | per-page render → ZIP |
| PDF → PNG | `/tools/pdf-to-png` | pdf.js + Canvas (browser) | per-page render → ZIP |
| PDF → Text | `/tools/pdf-to-text` | pdf.js text API (browser) | honest label — **not** fake "PDF→Word" |
| PDF → EPUB | `/tools/pdf-to-epub` | pdf.js text API/Canvas + JSZip (browser) | reflowable text EPUB by default; fixed-layout page images for scans/complex layouts |
| Extract to Word | `/tools/extract-pdf-to-word` | pdf.js text API + styled DOCX ZIP (browser) | text only — no layout claims |
| Extract to Excel | `/tools/extract-pdf-to-excel` | pdf.js text positions + styled XLSX ZIP (browser) | best-effort rows/columns, honestly labeled |
| Extract to PowerPoint | `/tools/extract-pdf-to-powerpoint` | pdf.js text API + styled PPTX ZIP (browser) | one slide per page, text only — no layout claims |
| OCR searchable PDF | `/tools/ocr-searchable-pdf` | tesseract-wasm + pdf.js + Rust→WASM text layer (browser) | English OCR model, invisible searchable text layer |
| PDF/A export | `/tools/export-pdfa` | pdf.js + Rust→WASM `unfleece-core` (krilla) | raster visual PDF/A-2b export, text becomes non-selectable |
| Convert image | `/tools/convert-image` | Canvas + jSquash + magick-wasm (browser) | PNG/JPEG/WebP plus AVIF/JPEG XL encode; TIFF/PSD/BMP/GIF-style inputs |
| HTML / Markdown → PDF | `/tools/html-markdown-to-pdf` | Rust→WASM `unfleece-core` | text-focused generator, not a browser layout engine |

### Edit (incl. the direct-manipulation editors)
The direct-manipulation editors render the actual page (pdf.js → HiDPI canvas) with a live
overlay on a shared `PdfStage` (pager + 0.5–3× zoom, mobile pan):

| Tool | URL | Engine | Notes |
|---|---|---|---|
| Page numbers | `/tools/add-page-numbers` | editor + Rust→WASM `unfleece-core` | tap-a-zone placement, live format preview |
| Bates numbering | `/tools/bates-numbering` | Rust→WASM `unfleece-core` (worker) | prefix + padded sequential page stamps |
| Watermark | `/tools/watermark-pdf` | editor + Rust→WASM `unfleece-core` | live opacity/angle/size preview |
| Crop | `/tools/crop-pdf` | editor + Rust→WASM `unfleece-core` | 8-handle crop rect + numeric margins |
| Auto-crop margins | `/tools/auto-crop-pdf` | pdf.js + Rust→WASM CropBox writer (browser) | pixel-detects white margins and sets per-page crop boxes |
| Sign | `/tools/sign-pdf` | editor + Rust→WASM image stamping | Draw/Type/Upload modal, **local signature library** (IndexedDB), snap guides, multi-page multi-placement, applied in one pass |
| Edit metadata | `/tools/edit-pdf-metadata` | Rust→WASM `unfleece-core` (worker) | title/author/subject/keywords round-trip |

### Optimize
| Tool | URL | Engine | Notes |
|---|---|---|---|
| Optimize | `/tools/optimize-pdf` | **Rust→WASM `unfleece-core` (lopdf)** | lossless object-graph rewrite |
| Compress | `/tools/compress-pdf` | Ghostscript-WASM (`@okathira/ghostpdl-wasm`) + raster fallback | `pdfwrite` presets for image-heavy PDFs; fallback raster mode is explicitly labeled |
| Compress image | `/tools/compress-image` | Canvas + jSquash + magick-wasm (browser) | resize and re-encode PNG/JPEG/WebP/AVIF/JPEG XL; TIFF/PSD/BMP/GIF-style inputs |

### Forms
| Tool | URL | Engine | Notes |
|---|---|---|---|
| Fill form | `/tools/fill-pdf-form` | Rust→WASM `unfleece-core` | AcroForm detect + fill (no XFA — detected and refused honestly) |
| Flatten | `/tools/flatten-pdf` | Rust→WASM `unfleece-core` | |

### Security & Privacy
| Tool | URL | Engine | Notes |
|---|---|---|---|
| Remove metadata | `/tools/remove-pdf-metadata` | Rust→WASM `unfleece-core` (worker) | clears document-info metadata fields |
| Protect PDF | `/tools/protect-pdf` | Rust→WASM `unfleece-core` (worker) | Standard Security password encryption + permission flags |
| Unlock PDF | `/tools/unlock-pdf` | Rust→WASM `unfleece-core` (worker) | opens with password and saves an unencrypted copy |
| Sanitize PDF | `/tools/sanitize-pdf` | Rust→WASM `unfleece-core` (worker) | strips metadata, scripts, embedded files, page actions, optional annotations/forms |
| Redact PDF | `/tools/redact-pdf` | pdf.js + Canvas + Rust→WASM image-PDF assembly (browser) | draw boxes, rasterize pages, rebuild image-only PDF |

## Honest reframes that shipped

- **"Extract to Word" is text-only DOCX**, not high-fidelity PDF→Word — client-side
  extraction can't reconstruct layout, so we don't pretend it can.
- **"Extract to Excel" is best-effort row extraction**, not guaranteed table recovery.
- **"Extract to PowerPoint" is text-only PPTX**, not high-fidelity PDF→PowerPoint.
- **"PDF → EPUB" offers two honest modes** — reflowable text when selectable text exists,
  fixed-layout page images when visual fidelity matters more than adjustable text.
- **"Compress" uses Ghostscript first and labels its raster fallback** — **Optimize** is
  still the lossless path when users need selectable text guaranteed.
- **Sign is "a visual signature mark, not a cryptographic e-signature"** — stated in the UI.
- **Redact is image-only by design** — it rasterizes and rebuilds pages so text under boxes
  is not preserved, trading editability/searchability for privacy.

---

## Hard or Skip 🔴 — keep refusing these

| Tool | Reality | Decision |
|---|---|---|
| PDF → Word (high fidelity) | Needs LibreOffice-WASM (~250 MB–1 GB) or a server | shipped as text-only "Extract to Word" instead |
| PDF → PowerPoint (high fidelity) | Only image-per-slide gimmick without a heavy layout engine | shipped as text-only "Extract to PowerPoint" instead |
| DOCX/XLSX/PPTX → PDF (faithful) | LibreOffice-WASM ~1 GB or paid `docx-wasm` | **Skip** |
| Cryptographic PAdES/LTV sign | TSA + OCSP/CRL are network-bound | **Skip** |
| AI summarize / chat / translate | Needs an LLM → breaks no-upload promise | **Out of scope** |

### ⚠️ Redaction constraint
Drawing a rectangle over text does **not** remove it; select-all/copy or any parser
recovers it. The shipped Redact tool uses the defensible client-side method:
rasterize page → paint → rebuild as an image-only PDF. That destroys selectable text and
can bloat files, so the UI labels it as rebuilding pages rather than promising a
text-preserving secure edit. See `docs/08-risks.md`.

---

## The full function universe (~110, for reference)

Beyond the shipped set and candidates above, the surveyed suites (iLovePDF, Smallpdf,
Adobe, PDF24, Stirling-PDF, Sejda) collectively offer a long tail we may cherry-pick
from later: split-by-text, split-in-half (2-up scans), alternate & mix,
poster/page-splitting, auto-split on divider/QR pages, create bookmarks/TOC, header &
footer, color invert/grayscale, deskew, remove blank pages, repair PDF,
linearize/web-optimize, get-info
(fonts/sizes/security), manage attachments, generate QR, scan-to-PDF (camera), and format
converters for TXT/RTF/HTML/EPUB/CBZ/ODF/EML. Each maps onto the same primitives —
**most are thin UI wrappers over "manipulate the object graph" (build) or "render a page"
(wrap)**, which is why the engine strategy in `docs/03-engine.md` matters more than the
tool count.
