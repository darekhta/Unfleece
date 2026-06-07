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
