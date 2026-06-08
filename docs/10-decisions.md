# 10 — Decision Log (ADR-style)

Concise record of the big calls and their rationale. Newest at the bottom.

---

### ADR-001 — Fully client-side, no backend
**Decision:** all file processing runs in the browser via WASM; there is no server that
touches user files.
**Why:** it's the only design that is simultaneously *free* (no compute/bandwidth bill),
*private* (files never leave the device), and *infinitely scalable*. It's the whole
premise. **Consequence:** features that genuinely need a server (high-fidelity Office
conversion, AI) are reframed or dropped, not built.

### ADR-002 — Host on Cloudflare Pages free tier
**Decision:** static hosting on Cloudflare Pages; R2 for oversized assets.
**Why:** uniquely *unlimited* bandwidth at $0 (GitHub/Netlify/Vercel cap ~100 GB/mo) — and
a download-heavy WASM app needs that. **Consequence:** design around 25 MiB/file + 20k
files limits; big WASM/lang packs go on R2 (zero egress).

### ADR-003 — Rust → WebAssembly for the engine
**Decision:** write our own engine code in Rust, compiled to WASM, run in Web Workers.
**Why:** team strength; the pure-Rust write/structure layer is mature and wasm-native
(proven by Typst/krilla). **Consequence:** adopt `lopdf` + `krilla` as foundations.

### ADR-004 — Build the moat, wrap the tar pit
**Decision:** BUILD page-object ops + generation + privacy ops (lopdf/krilla); WRAP
render/OCR/compress/office with proven engines.
**Why:** ~60 of the ~90 "tools" reduce to two brutally hard primitives (render,
robust-parse) that PDFium already solved over 20 years; re-implementing them is a
person-year sink. **Consequence:** PDFium via `pdfium-render` for render; Ghostscript-WASM
for compression; tesseract-wasm for OCR.

### ADR-005 — Use the `paulocoutinhox/pdfium-lib` WASM build
**Decision:** for PDFium-in-WASM, use the growable-heap build, not `bblanchon/pdfium-binaries`.
**Why:** the bblanchon WASM build has a non-growable heap that OOMs past a few pages.
**Consequence:** render pipeline survives real-world multi-page documents.

### ADR-006 — App licensed AGPL-3.0; core crate permissive
**Decision:** the Unfleece app is AGPL-3.0; the pure-Rust `unfleece-core` crate is
dual MIT/Apache and kept free of AGPL deps (`mupdf-rs` banned from it).
**Why:** AGPL unlocks the AGPL-only engines (Ghostscript compression, MuPDF fallback) for
free *and* encodes the "source you run = source you can read" trust promise. The permissive
core lets others reuse our object-graph/generation work. **Consequence:** AGPL engines live
only in the isolated `unfleece-wrap` layer; prefer BSD PDFium over AGPL MuPDF.

### ADR-007 — Honest scoping of conversions
**Decision:** ship "Extract text/tables/slides to Word/Excel/PowerPoint" (not fake
"PDF→Word" or high-fidelity "PDF→PowerPoint"); drop faithful Office→PDF; ship redaction
only as a rasterize-and-rebuild tool.
**Why:** no good client-side path exists for high-fidelity Office conversion (LibreOffice-WASM
is ~250 MB–1 GB); mislabeled redaction is a legal liability. **Consequence:** honesty
becomes a brand asset; we don't chase the "pdf to word" head term on fidelity.

### ADR-008 — Frontend = Astro + Svelte islands
**Decision:** Astro static shell (one URL per tool) + Svelte islands + Web Workers via
Comlink. The shipped app is Astro 6, Svelte 5, and a local CSS design system.
**Why:** satisfies both static SEO pages and heavy-engine-on-demand; Svelte ships lean
islands; Astro islands avoid SSR/worker friction. **Consequence:** per-tool lazy loading
is natural; programmatic-SEO pages are cheap. The original Tailwind plan was dropped
because the hand-authored design system is smaller and easier to audit.

### ADR-009 — Name = Unfleece
**Decision:** brand is **Unfleece**; deploy on `unfleece.pages.dev`, secure
`unfleece.com`/`.app` at launch.
**Why:** on-mission, friendly, low trademark risk, unique for SEO, and the entire domain
set was available. **Consequence:** taglines carry the "PDF tools" category + "never leaves
your browser" wedge.

### ADR-010 — Light-first design system, emerald sheep mark
**Decision:** ship the "quiet, engineered, reassuring" hand-authored CSS system with
**light as the default theme** (dark via persisted toggle), emerald `#2ECC8F` accent,
custom monoline icons, and a generated emerald-sheep logo as the brand mark.
**Why:** light default reads more trustworthy for a document tool; one accent color keeps
the UI calm; owned CSS/icons keep the bundle auditable and font requests local.
**Consequence:** the original dark-glassmorphism direction survives as the *toggle*, not
the default.

### ADR-011 — Direct-manipulation editors on a shared PdfStage
**Decision:** sign / crop / watermark / page-numbers are real editors: pdf.js renders the
actual page to a HiDPI canvas with an absolute HTML overlay; geometry is stored as
**page fractions**; interactions use Pointer Events with 44 px hit targets; the stage
provides paging and 0.5–3× zoom (overlay `touch-action: pan-x pan-y` so zoomed pages pan
on mobile while handles still drag).
**Why:** forms-with-numeric-fields UX loses to every modern competitor; fractions survive
resize/zoom for free; one stage component keeps all editors consistent.
**Consequence:** stage dims publish only after pdf.js finishes rendering — consumers must
await stage-readiness rather than assume canvas-visible means ready.

### ADR-012 — Signature objects are document-independent; never store Blobs in IndexedDB
**Decision:** signatures are created in a Draw/Type/Upload modal and live in a local
library (own IndexedDB database) that exists before, during and after any document;
placements snapshot a *finished* signature. Library records store **ArrayBuffers**, not
Blobs.
**Why:** mirrors the DocuSign/Adobe pattern users already know; placing only finished
signatures killed a real bug (placement captured mid-drawing after the first stroke);
WebKit **silently drops Blob values written to IndexedDB**, which destroyed saved
signatures on iOS — ArrayBuffers persist everywhere.
**Consequence:** storage-touching features must be verified on real WebKit, and IDB write
errors are surfaced, never swallowed.

### ADR-013 — unfleece.com is the canonical origin
**Decision:** `unfleece.com` purchased at Cloudflare Registrar and attached (with `www`)
to the Pages project; all canonical URLs, sitemap, robots and OG metadata point to it;
`unfleece.pages.dev` stays as deploy alias.
**Why:** owned domain is the brand; pages.dev remains a $0 fallback and preview surface.
**Consequence:** the only recurring infrastructure cost is the ~$10/yr domain.

### ADR-014 — Rust-first engine: TypeScript reduced to UI + rendering glue
**Decision:** every PDF operation that does not require page *rendering* lives in the
Rust `unfleece-core` crate (lopdf/krilla → WASM), organized as one module per operation
family (`pages`, `merge`, `boxes`, `meta`, `sanitize`, `stamp_text`, `stamp_image`,
`images_to_pdf`, `impose`, `pdfa`) behind `*_native` host-testable functions with thin
`wasm_bindgen` wrappers. Multi-part inputs cross the boundary as length-prefixed
little-endian packs (`core/src/pack.rs` ↔ `src/lib/wasm/pack.ts` — kept in lockstep).
TypeScript keeps: UI, worker RPC, pdf.js rendering paths, Canvas/OCR/Ghostscript glue,
and pdf-lib **only** for AcroForm fill/flatten and protect/unlock (wave 2 candidates).
**Why:** the owned moat belongs in Rust (one engine, host-testable, no JS library
drift); lopdf object-graph surgery outperforms and out-tests the pdf-lib equivalents;
WASM is loaded once and shared by all tools.
**Consequence:** ~6,900 lines of Rust with 230 cargo tests own the engine; the eight
migrated TS tool files shrank from ~790 to ~420 lines of orchestration; unit tests run
against the real WASM in Node (the silent pdf-lib fallback that previously masked WASM
load failures was removed); the wasm binary grew only ~0.3 MB (PNG/JPEG codecs included).
Known facts encoded in tests: lopdf must never receive duplicate page indices in a
same-document rewrite; lopdf does not reject encrypted PDFs (a TS `/Encrypt` pre-check
guards sanitize); Standard-14 Helvetica metrics are embedded AFM tables (kern-free,
matching what is actually painted).

### ADR-015 — Wave 2: pdf-lib fully removed; the engine is 100% Rust
**Decision:** port the last pdf-lib-backed operations into `unfleece-core` and drop
`@cantoo/pdf-lib` from production. New Rust modules: `forms` (AcroForm list/fill/flatten
with regenerated appearance streams), `crypto` (Standard Security protect/unlock — a
from-scratch RC4 + AESV2 + AESV3 handler on RustCrypto, because lopdf's own `decrypt`
panics on real files), `office` (DOCX/XLSX/PPTX containers + row/column reconstruction
via the `zip` crate), `epub` (reflowable + fixed-layout EPUB), `text_pdf` (text→PDF
layout), and `assemble` (image-only PDF builder, invisible OCR text layer, per-page
CropBox) which lets the browser assemblers — redact, raster-compress, OCR, auto-crop —
stop assembling with pdf-lib. pdf.js still renders; `@cantoo/pdf-lib` is now a
**devDependency only**, kept as an independent test oracle.
**Why:** one owned, host-testable engine; lopdf object-graph work beats and out-tests
the pdf-lib equivalents; removing pdf-lib from the bundle finishes the Rust-first goal.
**Consequence:** the core is ~13k lines of Rust with **540 cargo tests** (cross-engine
pdf-lib/qpdf-style fixtures, proptest round-trips, and a panic-fuzz harness that feeds
hostile bytes to every entry point — it already caught a `u32` index-overflow panic),
plus 169 vitest (through real WASM) and 47 E2E. wasm grew 2.05 → 2.59 MB (crypto + zip +
image codecs). GOTCHA pinned: pdf.js detaches the input ArrayBuffer, so any tool that
re-uses the bytes after `getDocument` must pass `bytes.slice()` (bit OCR).

### ADR-016 — Content-blind error beacon (the one deliberate server touchpoint)
**Decision:** report errors — and only errors — with a strictly allowlisted,
content-free payload `{tool, errorClass, engine, browser, locale}`. It can never
carry file names, file bytes, error messages, stack traces, URLs, cookies or any
per-user identifier. It honors Do-Not-Track, has a one-click opt-out in the
About page's privacy section (`uf-telemetry`), sends at most one fire-and-forget `sendBeacon`,
skips user-cancellations, and fails silent. Sink: a Cloudflare Pages Function
(`functions/api/err.ts`) that re-validates against the allowlist and writes one
Analytics Engine data point (when the `ERRORS` binding is configured), else 204.
**Why:** on-device processing means production bugs are otherwise invisible until
a user reports them (cf. the WebKit IndexedDB-Blob loss and the OCR
detached-buffer bug). This is the conscious choice over flying blind — observability
without betraying the privacy promise. `errorClass()` (lib/errors.ts) derives a
fixed token from the error *category*, never its text.
**Consequence:** one Pages Function — the project's sole server touchpoint — sees
counts, never content. To collect data, bind an Analytics Engine dataset named
`ERRORS` in the Pages project; without it the beacon is a no-op 204.

### ADR-017 — Two named engines and a clone-safe seam; honest threading model
**Decision:** name the architecture as two engines — the Rust→WASM object/generate
engine and the pdf.js+Canvas render/recognize engine (see docs/01) — and funnel
every crossing through one clone-safe doorway (`loadPdfDocument`/`dataForPdfjs`
in browser/pdfjs.ts) so pdf.js can never detach a buffer the Rust core reuses.
Pin the invariant with a seam test. Correct the docs to stop claiming "all heavy
work is in a Web Worker": object-graph work and pdf.js parsing are off-thread,
but Canvas rasterization is still main-thread (chunked + yielding), with an
OffscreenCanvas-in-worker move documented as the next step.
**Why:** the seam was the source of the session's worst bugs (OCR detached
buffer, WebKit Blob); making it a single guarded chokepoint removes the bug class
structurally, and honest threading docs keep the architecture legible.
**Consequence:** new render-engine code must load PDFs via `loadPdfDocument`,
never raw `getDocument`.

### ADR-018 — wasm split: measure first; carve PDF/A (krilla + fonts), not crypto/zip
**Measured** (twiggy on the unstripped 4.24 MB wasm; proportions carry to the
2.59 MB shipped artifact):

| Feature | ~bytes (raw) | Used by |
|---|---:|---|
| **Fonts** (skrifa + read_fonts + write_fonts) | **~715 KB** | krilla only |
| **krilla** + pdf_writer | ~245 KB | **PDF/A export only** |
| Image codecs (zune + image + png + weezl) | ~260 KB | images→PDF, stamp/sign, redact, assemble |
| lopdf | ~59 KB | almost every tool |
| zip + miniz/flate | ~50 KB (shared) | Office/EPUB |
| **Crypto** (aes+sha2+md5+rc4+cipher+digest) | **~37 KB** | protect/unlock |

**Decision:** the original hunch ("carve crypto/krilla/zip") was wrong on the
data — crypto (~37 KB) and zip (~25 KB) are negligible and shared. The whole-mass
is **krilla + its font stack ≈ 960 KB (~40% of the bundle), for the single
rarest tool (PDF/A)**. So the one worthwhile split is to extract PDF/A into a
**lazy second wasm module** (`unfleece-pdfa`, its own `cdylib` + wasm-pack pkg,
dynamic-`import()`ed only from the PDF/A path — the bundler-safe pattern, since
wasm-bindgen's native module-splitting breaks Vite). Image codecs stay in core
(shared by common tools); crypto/zip stay (too small to bother).
**Expected:** merge/rotate/split/crop/etc. — the common tools — drop from ~2.59 MB
toward ~1.6 MB first-use; PDF/A pays the ~1 MB only when chosen.
**Status:** measured + decided; the extraction is the scoped next build-system
change (it touches the crate layout + the npm wasm build + the pdfa bridge, so it
lands on its own once the core consolidation has settled).
