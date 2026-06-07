# 04 — Licensing

> **Status: in force.** The app is **AGPL-3.0** and the full canonical license text is
> vendored in `LICENSE`. The shipped bundle now includes Ghostscript/ghostpdl through
> `@okathira/ghostpdl-wasm` for the Compress tool, which is allowed because the whole
> app is AGPL.

Licensing is the **single biggest non-obvious constraint** on this project — it silently
decides which features we can ship. Read this before adding any dependency.

## The core tension

The hardest versions of features users expect from a "compress PDF" or "protected PDF"
button still lean on **AGPL-3.0** engines:

| Capability | Only real client-side engine | License |
|---|---|---|
| Photographic compression (40–70%) | Ghostscript-WASM | **AGPL-3.0** |
| Broad encrypted-PDF compatibility fallback | MuPDF | **AGPL-3.0** |
| True decrypt-and-edit edge cases | MuPDF | **AGPL-3.0** |
| Searchable-PDF (purpose-built) | Scribe.js | **AGPL-3.0** |

AGPL is **network-use copyleft**: serving AGPL code from a hosted app obligates you to
release the *entire combined work's* source to users — or buy a commercial license
(Ghostscript/MuPDF are sold commercially by Artifex).

## The decision: Unfleece app = AGPL-3.0

We go **fully open-source under AGPL-3.0** for the application. Rationale:

1. **It unlocks the AGPL engines** (Ghostscript compression, MuPDF fallback) legally and
   for free — no Artifex license needed.
2. **It reinforces the product promise.** AGPL means "the source you run is the source
   you can read, and anyone hosting a modified version must publish their changes." That
   *is* the "watch the Network tab / trust by inspection" ethos, encoded in the license.
3. The downside of AGPL (deterring proprietary reuse) is irrelevant — we *want* this to
   be open and verifiable.

## The exception: keep a permissive core

The standalone **`unfleece-core`** crate (pure Rust: lopdf + krilla + printpdf +
pdfium-render — all MIT/Apache/BSD) is intended to be **dual-licensed MIT OR Apache-2.0**
so the broader ecosystem can reuse our PDF object-graph + generation work.

**Hard rule:** `unfleece-core` must never pull in an AGPL dependency. In particular
**`mupdf-rs` is banned** from it. AGPL engines live only in the separate `unfleece-wrap`
layer, which is AGPL like the app. (See the crate layout in `docs/03-engine.md`.)

## Per-dependency license map

| Dependency | License | Safe? | Notes |
|---|---|---|---|
| lopdf | MIT | ✅ | core object model |
| krilla | MIT/Apache | ✅ | generation, PDF/A |
| printpdf | MIT | ✅ | generation/layout |
| PDFium (via pdfium-render) | **BSD-3** | ✅ | render — permissive! use `paulocoutinhox` WASM build |
| @cantoo/pdf-lib | MIT | ✅ | JS fallback (maintained fork of abandoned Hopding pdf-lib) |
| pdf.js | Apache-2.0 | ✅ | JS render/text fallback |
| jSquash | Apache-2.0 | ✅ | image codecs |
| magick-wasm | Apache-2.0 | ✅ | exotic image formats |
| tesseract-wasm | BSD-2-Clause | ✅ | OCR |
| RustCrypto crates | MIT/Apache | ✅ | encryption |
| **Ghostscript-WASM** | **AGPL-3.0** | ⚠️ allowed | only because app is AGPL; isolate in `unfleece-wrap` |
| **MuPDF / mupdf-rs** | **AGPL-3.0** | ⚠️ allowed in app, ⛔ in core | prefer PDFium wherever possible |
| Scribe.js | **AGPL-3.0** | ⚠️ optional | DIY tesseract+lopdf path is permissive and preferred |

## Practical rules

- **Prefer PDFium (BSD) over MuPDF (AGPL)** for render/parse whenever both work — it
  keeps more of the codebase permissive and avoids dependency on AGPL where unnecessary.
- Every PR adding a dependency must state its license (see `CONTRIBUTING.md`).
- ✅ Done: the **full canonical AGPL-3.0 text** is vendored in `LICENSE`, with
  third-party notices in `THIRD-PARTY-NOTICES.md`.
- Ghostscript runs **purely in the user's browser** (not on our servers) — the cleanest
  posture; still, ship our source to satisfy AGPL and document it.
