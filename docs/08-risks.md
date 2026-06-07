# 08 — Risks & Honest Gotchas

A risk register from an adversarial review. The tech mostly works; the things most likely
to sink the project are **licensing**, **the browser memory ceiling**, and **expectation
gaps**, not capability. Severity = how badly it bites.

## HIGH

### 1. Browser memory ceiling on large/many files
**Who:** anyone merging big scans or batch-processing on mobile.
**Reality:** the JS heap / WASM linear memory (~300 MB reliable mobile, 2–4 GB desktop,
~4 GB hard wall) loads whole files in. A demo with 2 MB invoices silently crashes on a
150 MB set — and the failure mode is a dead tab, not a graceful error.
**Mitigation:** all heavy work in a Web Worker (crash kills the worker, not the page);
detect input size and warn above ~100 MB desktop / ~40 MB mobile *before* processing;
catch OOM → "file too large for in-browser processing"; process page-by-page. **Never
market "no file size limits."**

### 2. PDF→Word/Excel/PowerPoint fidelity (expectation gap)
**Who:** the highest-search-volume visitors ("pdf to word", "pdf to powerpoint").
**Reality:** the only client-side path is text extraction — broken spacing, no real
tables/columns/slides, nothing for scanned PDFs. Users comparing to Smallpdf's server-side
reconstruction will judge it broken.
**Mitigation:** label these tools as **"Extract"** to Word/Excel/PowerPoint, set
expectations before upload, skip high-fidelity PDF→Office/PPTX claims, and compete on
merge/split/compress/OCR/image where we win.

### 3. AGPL licensing
**Who:** the project itself.
**Reality:** the only real client-side compressor (Ghostscript) and password engine
(MuPDF) are AGPL — using them obligates open-sourcing the whole app.
**Mitigation:** decided — **app is AGPL-3.0** (which we want anyway), permissive core crate
kept AGPL-free, **`mupdf-rs` banned from core**, prefer PDFium (BSD). See `docs/04`.

### 4. Redaction = legal liability if mislabeled
**Who:** exactly the privacy/compliance users we attract.
**Reality:** a black box over text leaves it recoverable (select-all/copy) — GDPR/HIPAA
malpractice if shipped as "redaction."
**Mitigation:** ship Ghostscript `pdfwrite` for compression and keep rasterize→repaint→
rebuild-as-image as an explicit fallback when used; label raster output as image-only and
never say it preserves selectable text. See `docs/02`.

### 5. SEO vs entrenched incumbents
**Who:** growth — where the project quietly dies if under-respected.
**Reality:** DA-80 incumbents own the terms; a new domain won't rank on head terms for a
long time; the privacy niche is already crowded.
**Mitigation:** treat SEO as a 6–18 mo grind; target long-tail/intent + locales; clear the
thin-content bar; use launch spikes for the first base. See `docs/07`.

## MEDIUM-HIGH

### 6. Mobile / iOS Safari constraints
iOS caps one canvas at 16.7M px and ~384 MB total canvas memory; Tesseract's WASM heap
grows but never shrinks (one full-res photo can OOM the next op).
**Mitigation:** cap render DPI, dispose canvases, `OffscreenCanvas`, prefer PDFium
Pixmap→PNG; downscale OCR input and **terminate the OCR worker between jobs**. Test on a
real mid-tier iPhone — the desktop demo lies.

### 7. WASM first-load tax
WASM compresses poorly (Ghostscript ~15 MB raw, mupdf ~4.3, magick ~4.8, Tesseract core
~2 + lang packs 3–25 MB each). Eager-loading tanks LCP.
**Mitigation:** lazy-load per tool on first file-drop, streaming-compile, cache immutable,
host lang packs on R2, show a determinate progress bar.

### 8. Password-protect / encrypt compatibility
Protect/Unlock now ship through `@cantoo/pdf-lib` Standard Security support, but encrypted
PDFs have many real-world edge cases. Full compatibility may still need MuPDF (AGPL) or
careful RustCrypto work in `unfleece-core`.
**Mitigation:** keep the UI honest, test common RC4/AES variants, and use the AGPL
`unfleece-wrap` layer only if the permissive path is insufficient.

## MEDIUM

### 9. Non-Latin / form-flatten fidelity
Drawn text (page numbers, watermarks, filled fields) in Cyrillic/CJK needs an embedded
Unicode TTF or it tofus; `flatten()` has appearance-stream edge cases; XFA forms fail.
**Mitigation:** embed a Unicode font by default for any text-drawing tool; detect XFA and
show "unsupported form type"; test flatten per template. (Directly relevant given
Ukrainian/Cyrillic users.)

### 10. Cloudflare free-tier edges
25 MiB per-asset cap (some Tesseract `.traineddata` approach it) and 20,000 files/site
(tools × locales × chunks).
**Mitigation:** keep oversized assets on R2; budget file count for localization. Not a
blocker — just don't discover it on deploy day.

### 11. (learned in production) WebKit IndexedDB Blob persistence
**Who:** any feature persisting binary data locally (hit by the signature library).
**Reality:** iOS Safari / WebKit can **silently fail to persist `Blob` values in
IndexedDB** — the write errors, the in-memory state looks fine, and the data is gone on
the next reload. Chromium does not reproduce it; only real WebKit does.
**Mitigation (shipped):** store **ArrayBuffers**, never Blobs, in IndexedDB; never
swallow IDB write errors (surface "saved for this session only"); keep a
reload-persistence E2E. Rule of thumb: anything storage-related must be verified on
WebKit, not just Chromium.

## Bottom line
The two things most likely to sink Unfleece were **the AGPL decision** (resolved:
AGPL-3.0, full text vendored) and **the SEO grind** (resolved: long-tail + launch spikes
+ patience). Everything else is an engineering mitigation already shipped in the live
product — including a few, like #11, that were only discoverable by shipping.
