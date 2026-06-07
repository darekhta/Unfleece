# Unfleece

**Free, private PDF tools — your files never leave your browser.**

🟢 **Live at [unfleece.com](https://unfleece.com)**

Unfleece is the honest, open-source antidote to "fleeceware" PDF sites — the ones that
bait you with a $1 trial and then quietly bill ~$50/week. Every operation runs
**100% in your browser** via WebAssembly. No uploads. No account. No limits. No catch.

> Because the dirty secret of those sites is that 90% of this can happen on your own
> machine — so it should, and it should be free.

---

## Why Unfleece exists

The big PDF utility sites (iLovePDF, Smallpdf, pdfSimpli, …) make money two ways that
Unfleece rejects:

1. **Subscription traps** — "free" tools funnel you into a $1.95 trial → ~$49.95/mo bill.
2. **Your files on their servers** — even the honest ones upload your documents to do
   the work, which is a privacy and compliance problem (GDPR / HIPAA / legal).

Unfleece's architecture removes both: there is **no server**. Files are processed
locally in a Web Worker and never transmitted. You can verify it yourself — open your
browser's Network tab and watch: nothing uploads.

## Principles

- **Free forever.** No trial, no subscription, no "premium" gate on core tools.
- **Private by architecture.** Files never leave the device. Not a promise — a fact you
  can inspect.
- **Fast & light.** Heavy engines load only when you actually use a tool.
- **Honest.** We label what each tool really does (e.g. "extract text to Word", not a
  fake "perfect PDF→Word"). We never call something "secure" that isn't.
- **Open source.** The whole app is auditable. Trust is verifiable, not asserted.

## How it's free (and stays free)

All processing is client-side, so there is no compute or bandwidth bill to pay. The
static app is hosted on **Cloudflare Pages**, whose free tier uniquely offers
*unlimited* bandwidth. Total recurring infrastructure cost: **$0/month** (plus an
optional ~$10/yr for the domain).

See [`docs/01-architecture.md`](docs/01-architecture.md) for the full picture.

## Tech at a glance

| Layer | Choice |
|---|---|
| Host | Cloudflare Pages (free, unlimited bandwidth) |
| Shell | Astro 6 (static, localized indexable URLs per tool) |
| Interactive tools | Svelte islands, hydrated on demand |
| Compute | Web Workers (Comlink) + lazy Rust → WebAssembly where it is mature |
| Offline | Installable PWA shell with service-worker caching for static tool pages |
| PDF engine today | `@cantoo/pdf-lib`, `pdf.js`, Canvas, `lopdf`/`krilla` WASM, and Ghostscript-WASM |
| Engine direction | More owned Rust/WASM operations; evaluate wrapped engines only where they beat browser primitives |
| Styling | Hand-authored CSS design system, no third-party font requests |

The strategy: **build the ~20% that's our moat** (page-object manipulation, generation,
privacy-sensitive ops) in pure Rust, and **wrap the ~80% that's a 20-year tar pit**
(rendering, OCR, compression) with proven engines. See
[`docs/03-engine.md`](docs/03-engine.md).

## Documentation

| Doc | What's in it |
|---|---|
| [00 — Vision](docs/00-vision.md) | Problem, mission, principles, audience, differentiators |
| [01 — Architecture](docs/01-architecture.md) | Client-side WASM model, $0 hosting, data flow, constraints |
| [02 — Features](docs/02-features.md) | The 37 shipped tools + future candidates from the ~110-function catalog |
| [03 — Engine (build vs wrap)](docs/03-engine.md) | Rust→WASM strategy, crates, the PDFium trap |
| [04 — Licensing](docs/04-licensing.md) | AGPL analysis, the open-source decision, per-dep licenses |
| [05 — Stack](docs/05-stack.md) | Frontend + WASM toolchain, libraries, versions |
| [06 — Roadmap](docs/06-roadmap.md) | Shipped scope (v1 complete) + future ideas |
| [07 — GTM & SEO](docs/07-gtm-seo.md) | Programmatic SEO, launch channels, differentiation |
| [08 — Risks](docs/08-risks.md) | Honest gotchas + mitigations (memory, mobile, fidelity, legal) |
| [09 — Branding](docs/09-branding.md) | The name, domains, voice, taglines |
| [10 — Decisions](docs/10-decisions.md) | ADR-style log of the big calls |

## Status

🟢 **Finished and live at [unfleece.com](https://unfleece.com)** (Cloudflare Pages;
`unfleece.pages.dev` is the deploy alias). 37 working tools, full test suite green.
Engine v1 is TypeScript (`@cantoo/pdf-lib` + `pdf.js` + Canvas) plus Rust→WASM
`unfleece-core` for lossless **Optimize PDF**, all-page **Rotate PDF**, and Rust page
selection for split/extract/remove/reorder, with `tesseract-wasm` for local OCR
searchable PDFs, `krilla` for PDF/A export, and Ghostscript-WASM for lossy PDF
compression.
Sign, crop, watermark, page numbers and redaction are direct-manipulation editors on a real
page preview (zoom, mobile-first touch interactions), and Sign includes a Draw/Type/Upload
signature modal with a local, document-independent signature library. The production
shell has no third-party font requests and ships hardened Cloudflare Pages headers.
See [`docs/06-roadmap.md`](docs/06-roadmap.md).

| | |
|---|---|
| Tools | 37 (organize, convert, edit, optimize, forms, security) |
| Tests | 76 unit (vitest) · 6 Rust (cargo) · 44 E2E (Playwright/Chromium) |
| Engines | TypeScript (pdf-lib/pdf.js) + Rust→WASM (lopdf/krilla) + Ghostscript-WASM |
| Cost | $0/month hosting · ~$10/yr domain |

## Getting started

```bash
npm install
npm run dev          # local dev server
npm run build        # static build → ./dist
npm run preview      # serve the build locally

# Tests
npm run test:unit    # vitest (pure tool logic, Node)
npm run test:rust    # cargo test (Rust core)
npm run test:e2e     # Playwright (full UI → worker → download in Chromium)
npm run test:all     # everything
npm run verify       # audit + type check + build + unit/Rust/E2E gates

# Rust → WASM engine (output committed under src/lib/wasm/pkg)
npm run build:wasm   # requires the Rust toolchain + wasm-pack

# Deploy (Cloudflare Pages)
npm run deploy
```

CI runs the same audit/check/build/test gates on pushes and pull requests, and Dependabot
tracks npm, Cargo and GitHub Actions updates weekly.

## License

**AGPL-3.0** — see [`LICENSE`](LICENSE) and [`docs/04-licensing.md`](docs/04-licensing.md)
for why (it lets us legally ship AGPL WASM engines like Ghostscript/MuPDF *and* makes the
"your files never leave" claim verifiable). The standalone pure-Rust core engine crate is
intended to be dual-licensed MIT/Apache-2.0 so others can reuse it.
