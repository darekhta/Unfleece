# Contributing to Unfleece

Thanks for helping build the PDF toolkit that doesn't fleece anyone. 🐑✂️🚫

## Ground rules (the non-negotiables)

These come straight from the product's reason to exist. PRs that violate them won't be
merged:

1. **Nothing uploads.** No tool may transmit user files or file contents off-device.
   No analytics that capture document data. If a feature *can't* be done client-side,
   it doesn't ship (or ships clearly labeled as unavailable) — see `docs/08-risks.md`.
2. **No dark patterns.** No fake "free" gates, no trial traps, no nagging, no bait UI.
3. **Honest labels.** A tool's name must match what it actually does. "Extract text to
   Word" ≠ "Convert PDF to Word". Never label something "secure"/"redacted" unless it
   provably is (see the redaction notes in `docs/02-features.md`).
4. **License hygiene.** Know the license of every dependency. AGPL engines
   (Ghostscript, MuPDF) are allowed because the app is AGPL-3.0, but **MuPDF/`mupdf-rs`
   must never be pulled into the standalone permissive core crate.** No proprietary
   blobs. See `docs/04-licensing.md`.

## How the codebase is organized

See `docs/01-architecture.md` and `docs/05-stack.md`. In short: an Astro static shell,
one route per tool, each hydrating a Svelte island that runs a Rust→WASM engine inside a
Web Worker.

## Adding a new tool

1. Check `docs/02-features.md` for the canonical name, tier, and the recommended
   library/approach.
2. Build vs. wrap: confirm against `docs/03-engine.md` before writing a new engine —
   most operations should ride `lopdf`/`krilla` (build) or PDFium (wrap), not new code.
3. Run heavy work in a Web Worker; lazy-load the WASM only on first file-drop.
4. Add an honest, indexable tool page (see `docs/07-gtm-seo.md` for the SEO bar).
5. Test on a real mid-tier phone — desktop hides the memory ceilings (`docs/08-risks.md`).

## Dev workflow

> Finalized with the MVP scaffold. Expected: `pnpm install`, `pnpm dev`, `pnpm build`.

Open an issue before large changes so we can align on approach.
