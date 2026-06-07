# 05 — Tech Stack

## Frontend

| Concern | Choice | Why |
|---|---|---|
| Framework | **Astro 6** (static output) | One indexable URL per tool, ships static HTML by default, hydrates tool widgets on demand → satisfies both SEO and lazy-WASM goals |
| Interactive islands | **Svelte 5** | Small interactive bundles with runes; Astro islands are client-only so browser APIs, workers and WASM stay straightforward |
| Styling | **Hand-authored CSS design system** | No unused framework runtime, no third-party font requests, predictable component primitives in `src/styles/*` |
| Components | **Local Astro/Svelte components** | Dropzone, runner, editor stage, options, cards and icons are owned code so privacy and accessibility behavior is auditable |
| Off-thread bridge | **Comlink** (~4 KB) | Ergonomic Worker RPC; keeps UI smooth during render/OCR/compress |
| Build tool | **Vite** (via Astro) | First-class `?worker` + `new URL(..., import.meta.url)` WASM/worker patterns |

**UI direction:** restrained glass surfaces, bento-grid tool launcher, explicit privacy
proofs and route-level code splitting. Keep effects light so Core Web Vitals (LCP/INP)
stay green. See `docs/09-branding.md`.

## Engines (compute)

See `docs/03-engine.md` for the full build-vs-wrap reasoning.

| Job | Engine | License | Load |
|---|---|---|---|
| Object ops (split/extract/remove/reorder/rotate/optimize) | `lopdf` in `unfleece-core` where it wins; `@cantoo/pdf-lib` fallback/merge/forms | MIT / MIT-Apache | lazy worker |
| Lossless optimize | `lopdf` in `unfleece-core` (Rust→WASM) | MIT/Apache | lazy per tool |
| Render / view / PDF→image | `pdf.js` + Canvas today; PDFium remains an evaluated future option for harder rendering cases | Apache / BSD candidate | lazy |
| Image codecs | `jSquash` (per-codec) | Apache-2.0 | tiny, per-codec lazy |
| Exotic image formats | `magick-wasm` | Apache-2.0 | ~4.8 MB gz, lazy on demand |
| OCR | `tesseract-wasm` + `eng.traineddata` | BSD-2-Clause / Apache-2.0 | lazy; local English model |
| Photographic compress | `@okathira/ghostpdl-wasm` (Ghostscript/ghostpdl) | AGPL-3.0-or-later | ~15 MB, lazy, isolated |
| JS libraries | `@cantoo/pdf-lib`, `pdf.js`, `jszip` | MIT / Apache / MIT | loaded only by tool paths that need them |

## WASM toolchain (Rust)

- `wasm-bindgen` + `wasm-pack` to build `unfleece-core` / `unfleece-wrap` for the web.
- `wasm-opt` (binaryen) on release artifacts to shrink + speed up.
- Run inside a **Web Worker**; instantiate with `WebAssembly.instantiateStreaming` so
  fetch + compile happen off the main thread.
- Brotli-precompress `.wasm` for delivery (Cloudflare serves it; remember WASM compresses
  poorly — expect ~30% reduction only).

## Hosting & infra

| Concern | Choice |
|---|---|
| Static host | **Cloudflare Pages** (free, unlimited bandwidth, free SSL) |
| Oversized assets (> 25 MiB) | **Cloudflare R2** — reserved for extra OCR lang packs or larger engines; unused today |
| Deploy | `wrangler pages deploy` of prebuilt `dist/` (`npm run deploy`) |
| Domain | **unfleece.com** (live, Cloudflare Registrar) · `unfleece.pages.dev` deploy alias |
| Package manager | **npm** (`package-lock.json`) |
| CI | **GitHub Actions**: audit, Astro check, build, unit tests, Rust locked tests, Playwright E2E |
| Dependency automation | **Dependabot** for npm, Cargo and GitHub Actions |

## Performance rules (non-negotiable)

1. **Landing/tool pages ship focused JS** — Astro islands; generic runner and special
   editors are split so ordinary tools do not load sign/crop/watermark UI code.
2. **Lazy-load engines on first file-drop**, streaming-compile, cache immutable.
3. **All heavy work in a Web Worker**; process PDFs page-by-page; dispose canvases.
4. **Code-split by tool path**; a generic visitor must not download specialized editors,
   OCR, or codec engines.
5. **Cap render DPI** (150 screen / 300 print toggle with a warning); use `OffscreenCanvas`.
