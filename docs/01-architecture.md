# 01 — Architecture

> **Status: as built.** This is the architecture of the live product at
> <https://unfleece.com> (Cloudflare Pages project `unfleece`, deploy alias
> `unfleece.pages.dev`).

## One picture

```
┌──────────────────────────────────────────────────────────────┐
│  Cloudflare Pages  ($0 · unlimited bandwidth · free SSL)       │
│                                                                │
│  Astro 6 static shell — localized URLs per tool (38 live)      │
│   /tools/merge-pdf  /uk/tools/merge-pdf  /it/tools/merge-pdf   │
│   • static HTML per page (great for SEO + first paint)         │
│   • Svelte islands split by generic runner vs special editors  │
│                                                                │
│        user drops a file ─┐                                    │
│                           ▼                                    │
│   ┌────────────── Web Worker (Comlink) ──────────────────┐    │
│   │  object/generate engine: lopdf / krilla / crypto / zip │    │
│   └──────────────────────────────────────────────────────┘    │
│   ┌────────────── Browser render paths ──────────────────┐    │
│   │  pdf.js worker + Canvas islands + Tesseract / GS       │    │
│   └──────────────────────────────────────────────────────┘    │
│                           │                                    │
│              download result ◄┘   (Blob, never uploaded)       │
│                                                                │
│  NO BACKEND.  FILES NEVER LEAVE THE DEVICE.                    │
└──────────────────────────────────────────────────────────────┘
   Oversized future assets (extra OCR lang packs, WASM > 25 MiB)
        → Cloudflare R2 (zero egress, $0)
          engines; the shipped app needs none of it
```

## Why this shape

The product promise ("free + private") and the technical design are the same decision.
Because all work happens in the browser:

- **No compute cost** → nothing to bill for → no paywall needed.
- **No file upload** → privacy is structural, not policy.
- **Infinite horizontal scale** → every user brings their own CPU.

## Two engines and the seam between them

Unfleece is honestly a **two-engine** system, and the boundary between them is a
first-class part of the architecture (it is where this project's nastiest bugs
have lived):

1. **The object / generate engine — Rust → WebAssembly (`unfleece-core`).**
   Everything that manipulates the PDF object graph or *generates* a container:
   merge, split, rotate, crop, metadata, sanitize, forms, page numbers,
   watermark, image stamping, images→PDF, N-up/booklet, optimize, PDF/A
   (krilla), Office/EPUB ZIPs, and Standard-Security encrypt/decrypt. Pure,
   host-testable, no DOM.

2. **The render / recognize engine — pdf.js + Canvas (+ Tesseract, Ghostscript).**
   Everything that needs to *rasterize a page or read its visual content*:
   page rendering, text extraction, OCR, visual compare, redaction burn-in,
   auto-crop detection, raster compression, image transcoding. This half needs
   the browser (Canvas/OffscreenCanvas) and cannot move into Rust.

**The seam** is where bytes cross from one engine to the other. pdf.js *detaches*
the `ArrayBuffer` it is given, so handing it a buffer that the Rust core later
reuses empties it — the bug class behind the OCR detached-buffer issue. The seam
is therefore funneled through a single clone-safe doorway, `loadPdfDocument()` /
`dataForPdfjs()` in `src/lib/browser/pdfjs.ts`, which always clones; no call site
hands pdf.js a buffer it does not own. The invariant is pinned by
`tests/unit/seam.test.ts`.

## Threading model (what actually runs where)

Being precise, because earlier docs over-claimed "all heavy work in a Web Worker":

- **pdf.js parsing** runs in pdf.js's own dedicated worker (off the main thread).
- **The object/generate engine** (`unfleece-core` WASM) runs inside the app's
  **Comlink Web Worker** (`pdf.worker.ts`) for the registry-driven tools, so
  object-graph work never blocks the UI.
- **Rasterization (Canvas) and the render-engine assemblers** (OCR, compare,
  redact, auto-crop, raster-compress, render-to-images) currently run on the
  **main thread / island**. They are chunked page-by-page and `await` between
  pages so the UI stays responsive and Cancel works, but a very large scan can
  still cost main-thread time. Tesseract OCR recognition runs in its own worker.

This is a deliberate, documented state — not "everything is in a worker."
**Planned next step:** move rasterization into a worker via `OffscreenCanvas`
(pdf.js supports rendering to an `OffscreenCanvas` from a worker), starting with
the shared `renderToImages` path.

## Layers

| Layer | Tech | Notes |
|---|---|---|
| Host | **Cloudflare Pages** (free) | Unlimited bandwidth/requests; free SSL; free `*.pages.dev` |
| Shell | **Astro 6** static | One static, SEO-indexable route per tool; ships HTML by default |
| Interactivity | **Svelte 5 islands**, `client:load` | Tool widgets hydrate as focused islands; generic runner and special editors are split |
| Off-thread compute | **Web Worker** + **Comlink** | Hosts the Rust→WASM object/generate engine; pdf.js + Tesseract use their own workers. Rasterization is still main-thread (see "Threading model"). |
| Offline shell | **PWA manifest + Service Worker** | Installable app; static pages and assets are cached from the sitemap for offline use |
| Engine loading | `import()` + worker/module chunks | 0 KB heavy engine cost until a tool needs it; HTTP-cached after |
| Object/generate engine | **`unfleece-core`** (Rust→WASM: lopdf + krilla + RustCrypto + zip + image) | Owns every non-rendering operation; see `docs/03-engine.md` |
| Render/recognize engine | **pdf.js** + Canvas, **tesseract-wasm**, **Ghostscript-WASM**, jSquash/magick-wasm | Rasterize, extract, OCR, compress, image codecs (pdf-lib is gone from production — dev-only test oracle) |
| Oversized assets | **Cloudflare R2** (zero egress) | Reserved for extra OCR lang packs and future heavier engines; current large WASM engines are lazy app assets |

## Hosting constraints to design around

Cloudflare Pages free tier (verified 2026):

- ✅ **Unlimited bandwidth & requests** (the killer feature — GitHub/Netlify/Vercel all
  cap ~100 GB/mo).
- ⚠️ **25 MiB max per single asset.** PDFium (~4.4 MB), Ghostscript (~14.8 MB), MuPDF
  (~9.5 MB) all fit. Tesseract `.traineddata` language packs can approach/exceed it →
  host those on **R2** or a CDN and fetch at runtime.
- ⚠️ **20,000 files per site.** Programmatic-SEO pages × locales × per-tool chunks can
  add up; budget the count, keep big assets on R2.
- ✅ **500 builds/mo, 20-min timeout, 1 concurrent.** Ample; for heavy CI, build
  elsewhere and push prebuilt assets (doesn't consume the build quota).

## The real ceiling: the browser, not the host

Cloudflare imposes no runtime limit (nothing runs on their compute). The limits are
client-side and **must be handled in the WASM/JS layer**:

- **Memory:** ~300 MB reliable on mobile Chrome; 2–4 GB desktop. WASM linear memory is
  32-bit (~4 GB hard wall) and loads whole files in. **Never promise "no size limits."**
- **iOS Safari:** single canvas ≤ 16,777,216 px; total canvas memory ~384 MB. Cap render
  DPI, use `OffscreenCanvas`, dispose canvases, prefer PDFium Pixmap→PNG over `<canvas>`
  for high-DPI.
- **Mitigations:** all heavy PDF object work in a Web Worker (a crash kills the worker,
  not the page); raster paths chunk page-by-page and yield; detect input size up front
  and warn above large local-memory thresholds; terminate OCR workers between jobs to
  reclaim heap.

Full risk register + mitigations: `docs/08-risks.md`.

## Optional tiny backend? Deliberately none

Cloudflare Workers (free: 100k req/day) cap CPU at **10 ms/invocation** — they
*cannot* do PDF work, and adding one would break the no-upload promise. Workers/R2 are
reserved strictly for **app assets**, never user files. If a feature truly needs a
server (high-fidelity Office conversion), we don't build it — we reframe or drop it.

## Privacy & security posture

- No file or file-content ever transmitted. The only telemetry is the opt-out,
  content-blind error beacon documented in ADR-016.
- Immutable caching for built assets.
- Production `_headers` set CSP, `Referrer-Policy: no-referrer`, `X-Frame-Options: DENY`,
  `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Resource-Policy: same-origin`,
  and a restrictive `Permissions-Policy`.
