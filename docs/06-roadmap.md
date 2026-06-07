# 06 — Shipped Scope & What's Next

> **Status: v1 is finished and live at <https://unfleece.com>.** Everything in the
> "Shipped" section below is deployed, tested, and free. The phases of the original
> roadmap (scaffold → weekend MVP → v1) are complete; this document now records the
> delivered scope and the candidate pool for future work.

## ✅ Shipped (live)

**Platform**
- Astro 6 static shell on Cloudflare Pages ($0 hosting, unlimited bandwidth), hardened
  `_headers` (CSP, COOP/CORP, no-referrer), English, Ukrainian and Italian tool URLs
  with canonical/hreflang alternates
- Installable PWA shell: manifest, shortcuts, service worker, and offline cache for
  static tool pages/assets
- **unfleece.com** purchased (Cloudflare Registrar) and live with SSL; `unfleece.pages.dev`
  remains as the deploy alias
- Svelte 5 islands + Web Worker (Comlink) compute; engines lazy-load on first file drop
- `unfleece-core` Rust crate (lopdf + krilla) → WASM via wasm-pack; powers lossless
  **Optimize**, all-page **Rotate**, PDF/A export, and Rust page selection for
  split/extract/remove/reorder
- Hand-authored CSS design system ("quiet, engineered, reassuring"): light-first theming
  with persisted dark toggle, emerald accent, glass + bento, custom monoline icon set,
  emerald-sheep logo
- IndexedDB hero→tool file handoff (drop a file on the homepage, pick a tool)
- Full AGPL-3.0 text vendored in `LICENSE`; CI (audit/check/build/unit/Rust/E2E) +
  Dependabot

**The 38 tools** (see `docs/02-features.md` for the full table)
- Organize: merge, split/extract/remove/reorder (Rust path with pdf-lib fallback),
  rotate (Rust path for all pages), N-up, booklet, compare PDFs
- Convert: images→PDF, PDF→JPG, PDF→PNG, PDF→Text, PDF→EPUB, Extract to Word,
  Extract to Excel, Extract to PowerPoint, OCR searchable PDF, PDF/A export,
  convert image, HTML/Markdown→PDF
- Edit: page numbers, Bates numbering, watermark, crop, auto-crop margins, sign,
  edit metadata
- Optimize: optimize (Rust/WASM, lossless), compress PDF (Ghostscript-WASM with explicit
  raster fallback), compress image
- Forms: fill AcroForm, flatten · Security: remove metadata, protect, unlock,
  sanitize PDF, redact PDF

**Direct-manipulation editors** (research-driven UX)
- Shared `PdfStage`: real page render (pdf.js, HiDPI), absolute overlay, pager,
  **0.5–3× zoom** with mobile panning
- Crop: 8-handle rect + numeric margins · Page numbers: tap-a-zone + live preview ·
  Watermark: live opacity/angle/size preview
- **Sign**: "Create signature" modal (Draw / Type / Upload), **local signature library**
  (IndexedDB, document-independent, survives reloads — stored as ArrayBuffers for
  WebKit), tap-to-place, snap guides while dragging, multi-page multi-placement,
  drag/resize/remove per stamp, single-pass stamping
- **Redact**: draw/resize per-page boxes, then rasterize and rebuild an image-only PDF so
  covered text is not left underneath
- Mobile-first interactions throughout: pointer events, 44 px targets, bottom-sheet modal

**Quality**
- 79 vitest unit + 6 cargo + 47 Playwright E2E (incl. a "nothing uploads" network
  assertion and a WebKit-regression guard for signature persistence) — all green
- Honest error mapping, focus management, keyboard paths, aria-live statuses

## ▶ Remaining launch steps (decided, not yet executed)

- [ ] Push the repo public at `github.com/darekhta/Unfleece` (footer already links to it)
- [ ] Launch spikes: Show HN, r/privacy, r/selfhosted, r/opensource, Product Hunt
      (playbook in `docs/07-gtm-seo.md`)

## 🔮 Future ideas (v2+, candidate pool)

- Expand `unfleece-core` further where it clearly wins: deeper merge/object remapping,
  metadata/forms/security object-graph paths, and more generation primitives

## Explicitly NOT on the roadmap
- Any server that processes files · high-fidelity Office conversion · AI summarize/chat ·
  cryptographic PAdES/LTV signatures. See `docs/00-vision.md` non-goals.
