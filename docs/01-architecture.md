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
│   │  lazy-load the engine for THIS tool                    │    │
│   │   • JS:     pdf-lib / pdf.js / Canvas / jszip          │    │
│   │   • WASM:   lopdf / krilla / Ghostscript / Tesseract   │    │
│   │  process page-by-page; return bytes to main thread    │    │
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
- **No upload** → privacy is structural, not policy.
- **Infinite horizontal scale** → every user brings their own CPU.

## Layers

| Layer | Tech | Notes |
|---|---|---|
| Host | **Cloudflare Pages** (free) | Unlimited bandwidth/requests; free SSL; free `*.pages.dev` |
| Shell | **Astro 6** static | One static, SEO-indexable route per tool; ships HTML by default |
| Interactivity | **Svelte 5 islands**, `client:load` | Tool widgets hydrate as focused islands; generic runner and special editors are split |
| Off-thread compute | **Web Worker** + **Comlink** | Keeps UI responsive during render/OCR/compress |
| Offline shell | **PWA manifest + Service Worker** | Installable app; static pages and assets are cached from the sitemap for offline use |
| Engine loading | `import()` + worker/module chunks | 0 KB heavy engine cost until a tool needs it; HTTP-cached after |
| Engines today | `@cantoo/pdf-lib`, `pdf.js`, Canvas, `jszip`, `lopdf` WASM optimize/rotate, `krilla`, `tesseract-wasm`, Ghostscript-WASM | See `docs/03-engine.md` for build-vs-wrap strategy |
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
  not the page); detect input size up front and warn above large local-memory thresholds;
  process page-by-page; terminate OCR workers between jobs to reclaim heap.

Full risk register + mitigations: `docs/08-risks.md`.

## Optional tiny backend? Deliberately none

Cloudflare Workers (free: 100k req/day) cap CPU at **10 ms/invocation** — they
*cannot* do PDF work, and adding one would break the no-upload promise. Workers/R2 are
reserved strictly for **app assets**, never user files. If a feature truly needs a
server (high-fidelity Office conversion), we don't build it — we reframe or drop it.

## Privacy & security posture

- No file or file-content ever transmitted. Any analytics must be content-blind.
- Immutable caching for built assets.
- Production `_headers` set CSP, `Referrer-Policy: no-referrer`, `X-Frame-Options: DENY`,
  `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Resource-Policy: same-origin`,
  and a restrictive `Permissions-Policy`.
