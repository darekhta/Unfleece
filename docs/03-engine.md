# 03 — Engine Strategy: Build vs. Wrap

> **Status: executed.** The build-the-moat plan below is now reality: `unfleece-core`
> (Rust→WASM, ten modules, 230 cargo tests) owns every non-rendering operation —
> merge, page selection/rotation, crop boxes, metadata, sanitize, text + image
> stamping, images→PDF, N-up/booklet imposition, lossless optimize and PDF/A
> emission. pdf.js remains the renderer; Ghostscript-WASM the compressor; pdf-lib
> survives only behind forms fill/flatten and protect/unlock. See ADR-014.

> **TL;DR.** Build the ~20% that is our moat (page-object manipulation + PDF generation +
> privacy ops) in pure Rust→WASM. Wrap the ~80% that is a 20-year tar pit (render, OCR,
> compress, office) with proven engines. The single biggest trap is trying to build the
> renderer/parser ourselves.

## Why Rust → WASM

- Plays to the team's strengths.
- The pure-Rust *write/structure* layer is mature **and wasm-native** — proven in
  production: Typst compiles its PDF backend (krilla) to `wasm32` and exports PDFs in the
  browser today.
- Lets us own the hard, differentiated parts (privacy ops, generation, PDF/A) instead of
  inheriting whatever a JS library happens to do.

## The Rust PDF ecosystem (wasm32 lens)

| Crate | Role | License | wasm32 | Verdict |
|---|---|---|---|---|
| **lopdf** | low-level object model: parse/edit/write the PDF object graph | MIT | ✅ pure Rust | **Core foundation** — the workhorse |
| **krilla** | high-level PDF *writer*; PDF/A-1/2/3/4, PDF/UA, tagged PDF | MIT/Apache | ✅ (Typst's backend) | **Best-in-class generation / PDF-A** |
| **printpdf** | create/read/write + light layout; ships a browser demo | MIT | ✅ first-class | Generation + light layout (built on lopdf) |
| **pdfium-render** | idiomatic wrapper over Google's **PDFium** (C++) | wrapper MIT; **PDFium = BSD** | ✅ loads a PDFium WASM module | **The render/robust-parse answer — safe to ship** |
| **pdf / pdf-rs** | parser + Pathfinder renderer + fonts | MIT | parser ✅, renderer ✗ | parser bits only; don't bet rendering on it |
| **mupdf-rs** | bindings to MuPDF | **AGPL-3.0** | builds, heavy | ⛔ **Banned from the permissive core crate** (see licensing) |
| **typst / typst-pdf** | typesetting engine → PDF via krilla | Apache-2.0 | ✅ in browser | Proof the pure-Rust write path works in WASM |

**Net read:** the pure-Rust write/structure layer (lopdf + krilla + printpdf) is mature
and wasm-native. The render/robust-parse layer has exactly one production-grade answer —
**C++ PDFium compiled to WASM**, wrapped by `pdfium-render`. No pure-Rust crate renders
arbitrary real-world PDFs reliably in the browser today.

### ⚠️ The PDFium WASM heap trap (decide on day one)
The popular `bblanchon/pdfium-binaries` WASM build uses a **non-growable heap** — it OOMs
unrecoverably past a few pages. **Use the `paulocoutinhox/pdfium-lib` WASM build**, which
has a growable heap. This one config choice decides whether the render pipeline works.

## Build vs. wrap, by capability cluster

| Cluster | Verdict | Effort | Hard parts |
|---|---|---|---|
| Organize / merge / split / rotate / reorder / extract / N-up | **BUILD** (lopdf) | ~2–4 wk for the family | object-graph surgery: copy objects across docs without dup'ing shared resources; xref/object-stream rewrite; preserve bookmarks/links/annotations |
| Parse / render / rasterize / view / PDF→image / thumbnails | **WRAP** (PDFium) | 1–2 wk integrate | the tar pit: content-stream interpreter, CMaps, CID/Type3 fonts, color spaces, transparency, malformed files |
| Compress / optimize | **WRAP** (Ghostscript-WASM) + small **BUILD** lossless pass | wrap ~1 wk; build 1–2 wk | image downsample + recompress + font subset + object packing |
| Image → PDF | **BUILD** (krilla/printpdf + `image`) | 3–5 d | orientation/EXIF, margins; HEIC decode needs a wasm decoder |
| Office ↔ PDF | **HARD-AVOID building; WRAP if forced** | weeks (wrap) / person-years (build) | OOXML+ODF layout fidelity ≈ re-implement LibreOffice; only path is LibreOffice→WASM (~250 MB) |
| OCR → searchable | **WRAP** (`tesseract-wasm`) | ~1 wk | don't build an OCR engine; you build the invisible-text-layer injection (the value-add) |
| Forms (fill / flatten / create) | **BUILD** (lopdf) | fill+flatten 1–2 wk | AcroForm dict manipulation; **avoid XFA**; appearance-stream generation on flatten |
| Fonts / text extraction | **BUILD** extraction; **REUSE** font crates | 1–2 wk | CMap/ToUnicode, CID fonts, RTL — for *reliable* extraction lean on PDFium's text API; subsetting via `subsetter`/krilla |
| Encryption / security | **BUILD** (lopdf + RustCrypto) | protect/unlock 1 wk; sign 3–5 wk | RC4 + AES-256 standard handler; redaction is a correctness/liability trap (use re-raster fallback); signatures need `cms`/`rasn` |

## The 20% to BUILD (the moat)

1. **Page-object suite on lopdf** — merge, split (all variants), reorder, rotate, delete,
   extract, N-up, page numbers, watermark/stamp, crop, metadata. ~80% of tools by count,
   ~90% of everyday usage, and where "never leaves your browser" is a real differentiator.
2. **Generation / PDF-A** via krilla — images→PDF, HTML/Markdown→PDF, archival PDF/A (a
   feature Adobe gates behind paid).
3. **Privacy-sensitive ops** — forms fill/flatten, protect/unlock, redaction (re-raster),
   sanitize/metadata-strip. The wedge: *"the PDF tool that physically cannot leak your
   document."*

## The 80% to WRAP

| Wrap | With | Why never build |
|---|---|---|
| render / view / PDF→image | **PDFium (BSD)** via pdfium-render, `paulocoutinhox` build | 20+ yrs of font/CMap/colorspace edge cases |
| compress | **Ghostscript→WASM** (AGPL) | downsampling/recompression heuristics |
| OCR | **tesseract-wasm** | a whole CV/ML engine |
| office ↔ PDF | **LibreOffice/ZetaOffice WASM** *(or punt)* | re-implementing an office suite |

## The single biggest trap

The catalog *looks* like 90 small jobs; it's really **~30 easy object-graph jobs + 2
brutally hard primitives — render and robust-parse — that PDFium already owns.** ISO
32000's long tail (CMaps, CID/Type3 fonts, ICC/Separation color, transparency groups,
encryption variants, malformed-but-Acrobat-opens-it files) is a multi-person-year sink.
**Build the object editor and the generator; rent the renderer.**

## Suggested crate layout

```
unfleece-core/      # pure Rust → WASM, MIT/Apache (NO agpl deps)
  ├─ organize/      # merge, split, rotate, reorder, n-up …  (lopdf)
  ├─ generate/      # image→pdf, html/md→pdf, pdf/a          (krilla)
  ├─ forms/         # fill, flatten                          (lopdf)
  ├─ secure/        # protect/unlock, sanitize, redact       (lopdf + RustCrypto)
  └─ extract/       # text/metadata                          (lopdf + font crates)

unfleece-wrap/      # AGPL-zone: thin bindings, isolated
  ├─ render/        # PDFium (BSD) — render, rasterize, robust text
  ├─ compress/      # Ghostscript-WASM (AGPL)
  └─ ocr/           # tesseract-wasm
```
Keeping `unfleece-core` free of AGPL deps lets it be dual-licensed MIT/Apache and reused
by others. See `docs/04-licensing.md`.
