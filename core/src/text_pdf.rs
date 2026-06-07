//! Plain text → PDF: lay out preprocessed plain text into a paged document.
//!
//! Rust port of `src/lib/tools/documentPdf.ts` (`textToPdf`, pdf-lib based) with
//! behaviour parity. The HTML/Markdown → plain-text conversion
//! (`sourceToPlainText`) stays **TS-side**; this engine receives the already
//! flattened plain text (paragraphs separated by blank lines) plus a small JSON
//! options object, and produces the laid-out PDF bytes.
//!
//! Layout (mirrors the TS algorithm exactly):
//! - Helvetica (body) + Helvetica-Bold (title), both `WinAnsiEncoding`.
//! - `lineHeight = fontSize * 1.45`, `maxWidth = pageWidth - 2*margin`.
//! - Page 1 opens with the title in bold at `fontSize + 5` pt; `y` then drops by
//!   `lineHeight * 1.8`.
//! - Paragraphs (split on runs of 2+ newlines) are split into lines (`\n`), each
//!   greedily word-wrapped to `maxWidth`; a word wider than `maxWidth` is broken
//!   character-by-character. Between paragraphs `y` drops an extra
//!   `lineHeight * 0.5`. A page break (`y < margin`) starts a fresh page.
//! - All text is drawn in `rgb(0.08, 0.1, 0.16)`.
//! - `/Info` gets `Title` (the safe title) and `Creator = "Unfleece"`.
//!
//! Text is measured with the embedded Adobe Core-14 AFM width tables and encoded
//! to WinAnsi via the shared [`crate::stamp_text`] helpers, so wrap points match
//! pdf-lib's. The body keeps the full WinAnsi character set (e.g. `é`); the
//! **title** additionally passes through `safe_text` (ASCII-only, everything else
//! → `?`), matching the TS `safeText()` — see the note in [`safe_text`] about the
//! NFKD step, which is performed TS-side.

use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};
use serde::Deserialize;

use crate::stamp_text::{encode_winansi, text_width_pt, HELVETICA_WIDTHS};
use crate::util::save_compact;

/// Supported page sizes in points, mirroring `PAGE_SIZES` in the TS layer.
const PAGE_SIZES: &[(&str, (f32, f32))] = &[("a4", (595.28, 841.89)), ("letter", (612.0, 792.0))];

/// Body + title fill colour, `rgb(0.08, 0.1, 0.16)`.
const TEXT_COLOR: [f32; 3] = [0.08, 0.1, 0.16];

/// Resource keys for the two fonts (kept stable for downstream tooling).
const FONT_REGULAR: &str = "F1";
const FONT_BOLD: &str = "F2";

// ---------------------------------------------------------------------------
// Options (camelCase JSON, mirroring documentPdf.ts `TextToPdfOptions`)
// ---------------------------------------------------------------------------

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct TextToPdfOpts {
    /// Document title. When absent the body uses `"Document"`; the TS layer
    /// supplies the filename-derived default here.
    title: Option<String>,
    /// `"a4"` (default) or `"letter"`.
    page_size: Option<String>,
    /// Body text size in pt, default 11. Must be > 0.
    font_size: Option<f32>,
    /// All four margins in pt, default 54. Must be >= 0.
    margin: Option<f32>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Lay out preprocessed plain `text` into a PDF.
///
/// `opts_json` mirrors the TS `TextToPdfOptions`:
/// `{ title?, pageSize?: "a4" | "letter", fontSize?, margin? }`. All fields are
/// optional; defaults are title `"Document"`, page size `a4`, font size 11 pt and
/// margin 54 pt. `text` must already be plain text (HTML/Markdown stripped by the
/// TS `sourceToPlainText`), with paragraphs separated by blank lines.
pub fn text_to_pdf_native(text: &str, opts_json: &str) -> Result<Vec<u8>, String> {
    let opts: TextToPdfOpts =
        serde_json::from_str(opts_json).map_err(|e| format!("Invalid text_to_pdf options: {e}"))?;

    let page_size = opts.page_size.unwrap_or_else(|| "a4".to_string());
    let (page_w, page_h) = PAGE_SIZES
        .iter()
        .find(|(name, _)| *name == page_size)
        .map(|(_, dims)| *dims)
        .ok_or_else(|| format!("Unknown page size '{page_size}'; expected 'a4' or 'letter'"))?;

    let font_size = opts.font_size.unwrap_or(11.0);
    if font_size.is_nan() || font_size <= 0.0 {
        return Err("Font size must be positive".to_string());
    }
    let margin = opts.margin.unwrap_or(54.0);
    if margin < 0.0 {
        return Err("Margin must be non-negative".to_string());
    }

    let line_height = font_size * 1.45;
    let max_width = page_w - 2.0 * margin;
    if max_width <= 0.0 {
        // pdf-lib would degrade to one glyph per line; reject instead of emitting
        // a pathologically large document.
        return Err("Margin too large: page content width must be positive".to_string());
    }

    // The title is ASCII-folded (NFKD applied TS-side); the body keeps WinAnsi.
    let title_text = safe_text(opts.title.as_deref().unwrap_or("Document"));
    // TS normalizes newlines before calling; do it defensively too.
    let body = text.replace("\r\n", "\n").replace('\r', "\n");

    // Operations accumulate per page; `pages_ops` always holds >= 1 page.
    let mut pages_ops: Vec<Vec<Operation>> = vec![Vec::new()];
    let mut y = page_h - margin;

    // Title (bold, fontSize + 5), drawn unconditionally on page 1.
    pages_ops[0].extend(line_ops(
        FONT_BOLD,
        font_size + 5.0,
        margin,
        y,
        encode_winansi(&title_text),
    ));
    y -= line_height * 1.8;

    // Body paragraphs.
    for paragraph in split_paragraphs(&body) {
        for raw_line in paragraph.split('\n') {
            for segment in wrap_line(raw_line, max_width, &HELVETICA_WIDTHS, font_size) {
                if y < margin {
                    pages_ops.push(Vec::new());
                    y = page_h - margin;
                }
                let ops = line_ops(FONT_REGULAR, font_size, margin, y, encode_winansi(&segment));
                pages_ops.last_mut().expect("at least one page").extend(ops);
                y -= line_height;
            }
        }
        // Inter-paragraph gap.
        y -= line_height * 0.5;
    }

    build_document(pages_ops, page_w, page_h, &title_text)
}

// ---------------------------------------------------------------------------
// Text shaping
// ---------------------------------------------------------------------------

/// Sanitize a title the way the TS `safeText()` does, **minus** the leading NFKD
/// normalization (which has no pure-Rust implementation among our deps and is
/// applied TS-side before the title reaches us): keep tab/LF/CR and printable
/// ASCII (`0x20..=0x7E`), replace everything else with `?`.
///
/// Because the filter is idempotent on ASCII, running it after a TS-side
/// `safeText()` (NFKD + this same filter) reproduces the TS result exactly. When
/// a raw, un-normalized title is passed instead, accented Latin letters collapse
/// straight to `?` rather than to `base + ?` — see the module integration notes.
fn safe_text(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{09}' | '\u{0A}' | '\u{0D}' => c,
            c if ('\u{20}'..='\u{7E}').contains(&c) => c,
            _ => '?',
        })
        .collect()
}

/// Split text into paragraphs on runs of 2+ newlines, matching JS
/// `String.split(/\n{2,}/)` (including the empty-string and trailing-break
/// behaviours).
fn split_paragraphs(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            let run_start = i;
            let mut j = i;
            while j < bytes.len() && bytes[j] == b'\n' {
                j += 1;
            }
            if j - run_start >= 2 {
                // `\n` is ASCII, so these are valid UTF-8 char boundaries.
                out.push(&text[start..run_start]);
                start = j;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out.push(&text[start..]);
    out
}

/// Greedily word-wrap one logical line to `max_width` pt using the AFM `widths`
/// table at `font_size`. A word wider than `max_width` is broken
/// character-by-character. Whitespace-only / empty input yields a single empty
/// line (so blank source lines still advance `y`). The per-character fallback
/// guarantees termination even for absurdly small widths (a lone glyph is always
/// emitted).
fn wrap_line(line: &str, max_width: f32, widths: &[u16; 224], font_size: f32) -> Vec<String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.is_empty() {
        return vec![String::new()];
    }

    let measure = |s: &str| text_width_pt(&encode_winansi(s), widths, font_size);

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for word in words {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };

        if measure(&candidate) <= max_width {
            current = candidate;
            continue;
        }

        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }

        if measure(word) <= max_width {
            current = word.to_string();
        } else {
            // Hard break: chunk the word so each chunk fits (single glyphs may
            // still exceed `max_width`, which is unavoidable and terminating).
            let mut chunk = String::new();
            for ch in word.chars() {
                let candidate = format!("{chunk}{ch}");
                if measure(&candidate) > max_width && !chunk.is_empty() {
                    lines.push(std::mem::take(&mut chunk));
                    chunk.push(ch);
                } else {
                    chunk = candidate;
                }
            }
            current = chunk;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

// ---------------------------------------------------------------------------
// lopdf assembly
// ---------------------------------------------------------------------------

/// The operator sequence for one drawn line: `BT rg Tf Tm Tj ET`, positioned by
/// an absolute text matrix so the baseline sits at `(x, y)` (pdf-lib semantics).
fn line_ops(font_key: &str, font_size: f32, x: f32, y: f32, encoded: Vec<u8>) -> Vec<Operation> {
    vec![
        Operation::new("BT", vec![]),
        Operation::new(
            "rg",
            vec![
                Object::Real(TEXT_COLOR[0]),
                Object::Real(TEXT_COLOR[1]),
                Object::Real(TEXT_COLOR[2]),
            ],
        ),
        Operation::new(
            "Tf",
            vec![
                Object::Name(font_key.as_bytes().to_vec()),
                Object::Real(font_size),
            ],
        ),
        Operation::new(
            "Tm",
            vec![
                Object::Real(1.0),
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(1.0),
                Object::Real(x),
                Object::Real(y),
            ],
        ),
        Operation::new("Tj", vec![Object::string_literal(encoded)]),
        Operation::new("ET", vec![]),
    ]
}

/// Assemble the per-page operation lists into a finished, compacted PDF.
fn build_document(
    pages_ops: Vec<Vec<Operation>>,
    page_w: f32,
    page_h: f32,
    title_text: &str,
) -> Result<Vec<u8>, String> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();

    let font_regular_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });
    let font_bold_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica-Bold",
        "Encoding" => "WinAnsiEncoding",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            FONT_REGULAR => font_regular_id,
            FONT_BOLD => font_bold_id,
        },
    });

    let mut kids: Vec<Object> = Vec::with_capacity(pages_ops.len());
    for operations in pages_ops {
        let content = Content { operations };
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            content.encode().map_err(|e| e.to_string())?,
        ));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0f32.into(), 0f32.into(), page_w.into(), page_h.into()],
        });
        kids.push(page_id.into());
    }

    let count = kids.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => count,
        }),
    );

    let info_id = doc.add_object(dictionary! {
        "Title" => Object::string_literal(title_text.to_string()),
        "Creator" => Object::string_literal("Unfleece"),
    });
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    doc.trailer.set("Info", info_id);

    save_compact(doc).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::Document;
    use proptest::prelude::*;

    // -- helpers ------------------------------------------------------------

    fn load(data: &[u8]) -> Document {
        Document::load_mem(data).unwrap()
    }

    fn page_count(data: &[u8]) -> usize {
        load(data).get_pages().len()
    }

    fn page_ops(data: &[u8], page: usize) -> Vec<Operation> {
        let doc = load(data);
        let pages: Vec<_> = doc.page_iter().collect();
        let content = doc.get_page_content(pages[page]).unwrap();
        Content::decode(&content).unwrap().operations
    }

    fn all_ops(data: &[u8]) -> Vec<Operation> {
        let doc = load(data);
        let mut out = Vec::new();
        for page_id in doc.page_iter() {
            let content = doc.get_page_content(page_id).unwrap();
            out.extend(Content::decode(&content).unwrap().operations);
        }
        out
    }

    /// All `Tj` operand byte strings across every page, in draw order. The first
    /// entry is always the title; the rest are body lines.
    fn tj_strings(data: &[u8]) -> Vec<Vec<u8>> {
        all_ops(data)
            .iter()
            .filter(|o| o.operator == "Tj")
            .map(|o| match &o.operands[0] {
                Object::String(bytes, _) => bytes.clone(),
                other => panic!("unexpected Tj operand: {other:?}"),
            })
            .collect()
    }

    /// `(font_name, font_size)` of each drawn line across all pages, in order.
    fn line_fonts(data: &[u8]) -> Vec<(Vec<u8>, f32)> {
        let ops = all_ops(data);
        let mut out = Vec::new();
        let mut pending: Option<(Vec<u8>, f32)> = None;
        for op in &ops {
            match op.operator.as_str() {
                "Tf" => {
                    let name = match &op.operands[0] {
                        Object::Name(n) => n.clone(),
                        other => panic!("unexpected Tf font: {other:?}"),
                    };
                    let size = num(&op.operands[1]);
                    pending = Some((name, size));
                }
                "Tj" => out.push(pending.clone().expect("Tf before Tj")),
                _ => {}
            }
        }
        out
    }

    /// Baseline `y` of every drawn line (the `Tm` ty operand), in order.
    fn line_baselines(data: &[u8]) -> Vec<f32> {
        all_ops(data)
            .iter()
            .filter(|o| o.operator == "Tm")
            .map(|o| num(&o.operands[5]))
            .collect()
    }

    fn first_tm(data: &[u8], page: usize) -> [f32; 6] {
        let ops = page_ops(data, page);
        let tm = ops.iter().find(|o| o.operator == "Tm").expect("no Tm");
        [
            num(&tm.operands[0]),
            num(&tm.operands[1]),
            num(&tm.operands[2]),
            num(&tm.operands[3]),
            num(&tm.operands[4]),
            num(&tm.operands[5]),
        ]
    }

    fn media_box(data: &[u8], page: usize) -> [f32; 4] {
        let doc = load(data);
        let pages: Vec<_> = doc.page_iter().collect();
        let arr = doc
            .get_dictionary(pages[page])
            .unwrap()
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap();
        [num(&arr[0]), num(&arr[1]), num(&arr[2]), num(&arr[3])]
    }

    fn info_title(data: &[u8]) -> Vec<u8> {
        let doc = load(data);
        let info = match doc.trailer.get(b"Info").unwrap() {
            Object::Reference(id) => doc.get_dictionary(*id).unwrap(),
            Object::Dictionary(d) => d,
            other => panic!("unexpected Info: {other:?}"),
        };
        match info.get(b"Title").unwrap() {
            Object::String(bytes, _) => bytes.clone(),
            other => panic!("unexpected Title: {other:?}"),
        }
    }

    fn info_creator(data: &[u8]) -> Vec<u8> {
        let doc = load(data);
        let info = match doc.trailer.get(b"Info").unwrap() {
            Object::Reference(id) => doc.get_dictionary(*id).unwrap(),
            Object::Dictionary(d) => d,
            other => panic!("unexpected Info: {other:?}"),
        };
        match info.get(b"Creator").unwrap() {
            Object::String(bytes, _) => bytes.clone(),
            other => panic!("unexpected Creator: {other:?}"),
        }
    }

    fn num(o: &Object) -> f32 {
        crate::util::number_as_f32(o).unwrap()
    }

    #[track_caller]
    fn assert_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.05,
            "expected {expected}, got {actual}"
        );
    }

    // -- happy path / structure --------------------------------------------

    #[test]
    fn empty_text_creates_valid_one_page_pdf_with_title() {
        let out = text_to_pdf_native("", "{}").unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count(&out), 1);
        // Title is the first (bold) Tj.
        assert_eq!(tj_strings(&out)[0], b"Document");
    }

    #[test]
    fn whitespace_only_text_is_one_page() {
        let out = text_to_pdf_native("   \n  \t ", "{}").unwrap();
        assert_eq!(page_count(&out), 1);
    }

    #[test]
    fn pdf_bytes_start_with_pdf_magic() {
        let out = text_to_pdf_native("hello world", "{}").unwrap();
        assert!(out.starts_with(b"%PDF-"));
    }

    #[test]
    fn title_drawn_bold_at_top_with_fontsize_plus_5() {
        let out = text_to_pdf_native("body", "{}").unwrap();
        let fonts = line_fonts(&out);
        // First drawn line is the title: bold font, size 11 + 5 = 16.
        assert_eq!(fonts[0].0, FONT_BOLD.as_bytes());
        assert_near(fonts[0].1, 16.0);
        // Title baseline at top: y = pageHeight - margin (a4).
        let tm = first_tm(&out, 0);
        assert_near(tm[4], 54.0);
        assert_near(tm[5], 841.89 - 54.0);
    }

    #[test]
    fn body_uses_regular_helvetica_at_font_size() {
        let out = text_to_pdf_native("one line", "{}").unwrap();
        let fonts = line_fonts(&out);
        assert_eq!(fonts[1].0, FONT_REGULAR.as_bytes());
        assert_near(fonts[1].1, 11.0);
    }

    #[test]
    fn single_short_paragraph_fits_one_page() {
        let out = text_to_pdf_native("The quick brown fox jumps over the lazy dog.", "{}").unwrap();
        assert_eq!(page_count(&out), 1);
        // Title + a single wrapped body line.
        assert_eq!(tj_strings(&out).len(), 2);
    }

    #[test]
    fn helvetica_and_helvetica_bold_fonts_embedded() {
        let out = text_to_pdf_native("x", "{}").unwrap();
        assert!(out.windows(b"Helvetica".len()).any(|w| w == b"Helvetica"));
        assert!(out
            .windows(b"Helvetica-Bold".len())
            .any(|w| w == b"Helvetica-Bold"));
    }

    #[test]
    fn color_of_all_text_is_rgb_0_08_0_1_0_16() {
        let out = text_to_pdf_native("line one\nline two", "{}").unwrap();
        let rgs: Vec<_> = all_ops(&out)
            .into_iter()
            .filter(|o| o.operator == "rg")
            .collect();
        assert!(!rgs.is_empty());
        for rg in rgs {
            assert_near(num(&rg.operands[0]), 0.08);
            assert_near(num(&rg.operands[1]), 0.1);
            assert_near(num(&rg.operands[2]), 0.16);
        }
    }

    // -- page geometry ------------------------------------------------------

    #[test]
    fn a4_page_size_has_correct_mediabox() {
        let out = text_to_pdf_native("x", r#"{"pageSize":"a4"}"#).unwrap();
        let mb = media_box(&out, 0);
        assert_near(mb[2], 595.28);
        assert_near(mb[3], 841.89);
    }

    #[test]
    fn letter_page_size_has_correct_mediabox() {
        let out = text_to_pdf_native("x", r#"{"pageSize":"letter"}"#).unwrap();
        let mb = media_box(&out, 0);
        assert_near(mb[2], 612.0);
        assert_near(mb[3], 792.0);
    }

    #[test]
    fn default_page_size_is_a4() {
        let out = text_to_pdf_native("x", "{}").unwrap();
        let mb = media_box(&out, 0);
        assert_near(mb[2], 595.28);
    }

    // -- wrapping / paging --------------------------------------------------

    #[test]
    fn long_text_wraps_to_multiple_pages() {
        let body = "word ".repeat(4000);
        let out = text_to_pdf_native(&body, "{}").unwrap();
        assert!(
            page_count(&out) > 1,
            "expected overflow onto multiple pages"
        );
    }

    #[test]
    fn page_count_matches_expected_for_known_input() {
        // a4 default: 45 single-line body rows fill page 1 exactly; the 46th
        // triggers a second page (see module layout maths).
        let fill = (0..45).map(|_| "x").collect::<Vec<_>>().join("\n");
        assert_eq!(page_count(&text_to_pdf_native(&fill, "{}").unwrap()), 1);

        let overflow = (0..46).map(|_| "x").collect::<Vec<_>>().join("\n");
        assert_eq!(page_count(&text_to_pdf_native(&overflow, "{}").unwrap()), 2);
    }

    #[test]
    fn new_page_resets_y_to_top() {
        let overflow = (0..46).map(|_| "x").collect::<Vec<_>>().join("\n");
        let out = text_to_pdf_native(&overflow, "{}").unwrap();
        assert_eq!(page_count(&out), 2);
        // First line on page 2 has baseline pageHeight - margin (no title there).
        let tm = first_tm(&out, 1);
        assert_near(tm[5], 841.89 - 54.0);
    }

    #[test]
    fn word_wrapping_respects_max_width_helvetica_metrics() {
        // Page wide enough only for a few words: assert no body line exceeds
        // maxWidth, yet wrapping actually occurred.
        let body = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
        let out = text_to_pdf_native(body, r#"{"margin":240}"#).unwrap();
        let max_width = 595.28 - 2.0 * 240.0;
        let body_lines = &tj_strings(&out)[1..];
        assert!(body_lines.len() > 1, "expected multiple wrapped lines");
        for line in body_lines {
            let w = text_width_pt(line, &HELVETICA_WIDTHS, 11.0);
            assert!(w <= max_width + 0.001, "line too wide: {w} > {max_width}");
        }
    }

    #[test]
    fn line_spacing_equals_font_size_times_1_45() {
        let out = text_to_pdf_native("a\nb\nc", "{}").unwrap();
        let ys = line_baselines(&out);
        // ys[0] is the title; ys[1..] are body lines spaced by lineHeight.
        let lh = 11.0 * 1.45;
        assert_near(ys[1] - ys[2], lh);
        assert_near(ys[2] - ys[3], lh);
    }

    #[test]
    fn y_position_decreases_correctly_as_text_drawn() {
        let out = text_to_pdf_native("a\nb", "{}").unwrap();
        let ys = line_baselines(&out);
        // Title at top, then drop of lineHeight*1.8 to the first body line.
        let lh = 11.0 * 1.45;
        assert_near(ys[0], 841.89 - 54.0);
        assert_near(ys[0] - ys[1], lh * 1.8);
    }

    #[test]
    fn paragraph_spacing_adds_inter_paragraph_gap() {
        let out = text_to_pdf_native("a\n\nb", "{}").unwrap();
        let ys = line_baselines(&out);
        // Two paragraphs: gap between their lines is lineHeight (advance) plus
        // lineHeight*0.5 (inter-paragraph) = lineHeight*1.5.
        let lh = 11.0 * 1.45;
        assert_near(ys[1] - ys[2], lh * 1.5);
    }

    #[test]
    fn hard_break_single_long_word_no_infinite_loop() {
        let word = "a".repeat(5000);
        let out = text_to_pdf_native(&word, "{}").unwrap();
        assert!(page_count(&out) >= 1);
        // The word is chunked across many body lines.
        assert!(tj_strings(&out).len() > 2);
    }

    #[test]
    fn very_long_single_word_in_text_chunks_correctly() {
        let word = "x".repeat(300);
        let out = text_to_pdf_native(&word, "{}").unwrap();
        // Reassembling the body Tj strings recovers the original word.
        let body: Vec<u8> = tj_strings(&out)[1..].concat();
        assert_eq!(body, word.as_bytes());
    }

    // -- options / defaults -------------------------------------------------

    #[test]
    fn custom_font_size_affects_line_height_and_width() {
        let out = text_to_pdf_native("a\nb", r#"{"fontSize":20}"#).unwrap();
        let fonts = line_fonts(&out);
        assert_near(fonts[0].1, 25.0); // title = 20 + 5
        assert_near(fonts[1].1, 20.0); // body
        let ys = line_baselines(&out);
        assert_near(ys[1] - ys[2], 20.0 * 1.45);
    }

    #[test]
    fn custom_margin_affects_content_boundaries() {
        let out = text_to_pdf_native("body", r#"{"margin":100}"#).unwrap();
        let tm = first_tm(&out, 0);
        assert_near(tm[4], 100.0); // x = margin
        assert_near(tm[5], 841.89 - 100.0); // y = pageHeight - margin
    }

    #[test]
    fn title_defaults_to_document_when_not_provided() {
        let out = text_to_pdf_native("body", "{}").unwrap();
        assert_eq!(tj_strings(&out)[0], b"Document");
        assert_eq!(info_title(&out), b"Document");
    }

    #[test]
    fn custom_title_appears_in_pdf_metadata_and_drawn() {
        let out = text_to_pdf_native("body", r#"{"title":"My Report"}"#).unwrap();
        assert_eq!(tj_strings(&out)[0], b"My Report");
        assert_eq!(info_title(&out), b"My Report");
    }

    #[test]
    fn creator_metadata_is_unfleece() {
        let out = text_to_pdf_native("body", "{}").unwrap();
        assert_eq!(info_creator(&out), b"Unfleece");
    }

    // -- text safety --------------------------------------------------------

    #[test]
    fn unicode_outside_winansi_becomes_question_mark_in_body() {
        // U+2116 (№) is outside WinAnsi -> '?'.
        let out = text_to_pdf_native("\u{2116}x", "{}").unwrap();
        let body = &tj_strings(&out)[1];
        assert_eq!(body, b"?x");
    }

    #[test]
    fn body_keeps_winansi_high_characters() {
        // 'é' (U+00E9) maps to WinAnsi 0xE9 in the body.
        let out = text_to_pdf_native("caf\u{00E9}", "{}").unwrap();
        let body = &tj_strings(&out)[1];
        assert_eq!(body, &[b'c', b'a', b'f', 0xE9]);
    }

    #[test]
    fn safetext_applies_to_title_only() {
        // Same accented text in title and body: title is ASCII-folded to '?',
        // body keeps the WinAnsi byte.
        let out = text_to_pdf_native("caf\u{00E9}", r#"{"title":"café"}"#).unwrap();
        let tjs = tj_strings(&out);
        assert_eq!(tjs[0], b"caf?"); // title (no NFKD here -> whole char to '?')
        assert_eq!(tjs[1], &[b'c', b'a', b'f', 0xE9]); // body
    }

    #[test]
    fn control_characters_stripped_from_body() {
        // A non-whitespace control char (bell, U+0007) is removed from the
        // body by encode_winansi. (Whitespace controls like VT instead
        // collapse to a single space, matching JS `/\s+/` — see the next test.)
        let out = text_to_pdf_native("a\u{0007}b", "{}").unwrap();
        assert_eq!(tj_strings(&out)[1], b"ab");
    }
    #[test]
    fn whitespace_control_char_collapses_to_space_like_js() {
        // Vertical tab is Unicode whitespace, so "a\u{000B}b" splits into two
        // words and rejoins with a single space (JS `/\s+/` parity).
        let out = text_to_pdf_native("a\u{000B}b", "{}").unwrap();
        assert_eq!(tj_strings(&out)[1], b"a b");
    }

    #[test]
    fn crlf_newlines_are_normalized() {
        // "a\r\n\r\nb" must split into two paragraphs just like "a\n\nb".
        let out = text_to_pdf_native("a\r\n\r\nb", "{}").unwrap();
        let ys = line_baselines(&out);
        let lh = 11.0 * 1.45;
        assert_near(ys[1] - ys[2], lh * 1.5); // inter-paragraph gap present
    }

    // -- errors -------------------------------------------------------------

    #[test]
    fn invalid_page_size_rejected() {
        let err = text_to_pdf_native("x", r#"{"pageSize":"a3"}"#).unwrap_err();
        assert!(err.contains("Unknown page size 'a3'"), "{err}");
    }

    #[test]
    fn negative_font_size_rejected() {
        let err = text_to_pdf_native("x", r#"{"fontSize":-1}"#).unwrap_err();
        assert_eq!(err, "Font size must be positive");
    }

    #[test]
    fn zero_font_size_rejected() {
        let err = text_to_pdf_native("x", r#"{"fontSize":0}"#).unwrap_err();
        assert_eq!(err, "Font size must be positive");
    }

    #[test]
    fn negative_margin_rejected() {
        let err = text_to_pdf_native("x", r#"{"margin":-5}"#).unwrap_err();
        assert_eq!(err, "Margin must be non-negative");
    }

    #[test]
    fn margin_too_large_rejected() {
        let err = text_to_pdf_native("x", r#"{"margin":400}"#).unwrap_err();
        assert!(err.contains("Margin too large"), "{err}");
    }

    #[test]
    fn invalid_json_options_rejected() {
        let err = text_to_pdf_native("x", "not json").unwrap_err();
        assert!(err.starts_with("Invalid text_to_pdf options:"), "{err}");
    }

    // -- wrap_line unit tests ----------------------------------------------

    #[test]
    fn wrapline_empty_string_returns_single_empty_line() {
        assert_eq!(
            wrap_line("", 500.0, &HELVETICA_WIDTHS, 11.0),
            vec![String::new()]
        );
        assert_eq!(
            wrap_line("   \t ", 500.0, &HELVETICA_WIDTHS, 11.0),
            vec![String::new()]
        );
    }

    #[test]
    fn wrapline_single_word_fitting_returns_one_element() {
        assert_eq!(
            wrap_line("hello", 500.0, &HELVETICA_WIDTHS, 11.0),
            vec!["hello".to_string()]
        );
    }

    #[test]
    fn wrapline_multiple_words_fits_greedy_packing() {
        // "aa" = 12.232pt; "aa aa" = 27.522pt @ 11pt Helvetica.
        assert_eq!(
            wrap_line("aa aa", 20.0, &HELVETICA_WIDTHS, 11.0),
            vec!["aa".to_string(), "aa".to_string()]
        );
        assert_eq!(
            wrap_line("aa aa", 30.0, &HELVETICA_WIDTHS, 11.0),
            vec!["aa aa".to_string()]
        );
    }

    #[test]
    fn wrapline_word_too_wide_is_character_chunked() {
        // 'a' = 6.116pt @ 11pt; 3 fit in 20pt, 4 do not.
        assert_eq!(
            wrap_line("aaaaaaaaaa", 20.0, &HELVETICA_WIDTHS, 11.0),
            vec![
                "aaa".to_string(),
                "aaa".to_string(),
                "aaa".to_string(),
                "a".to_string(),
            ]
        );
    }

    #[test]
    fn split_paragraphs_matches_js_semantics() {
        assert_eq!(split_paragraphs(""), vec![""]);
        assert_eq!(split_paragraphs("a\nb"), vec!["a\nb"]);
        assert_eq!(split_paragraphs("a\n\nb"), vec!["a", "b"]);
        assert_eq!(split_paragraphs("a\n\n\n\nb"), vec!["a", "b"]);
        assert_eq!(split_paragraphs("\n\na"), vec!["", "a"]);
        assert_eq!(split_paragraphs("a\n\n"), vec!["a", ""]);
    }

    #[test]
    fn safe_text_folds_non_ascii_and_keeps_eol() {
        assert_eq!(safe_text("caf\u{00E9}"), "caf?");
        assert_eq!(safe_text("a\tb\nc\rd"), "a\tb\nc\rd");
        assert_eq!(safe_text("\u{2116}"), "?");
        // Non-EOL control chars (here U+0007 bell) become '?' (title path).
        assert_eq!(safe_text("x\u{0007}y"), "x?y");
    }

    // -- property tests -----------------------------------------------------

    proptest! {
        /// For any ASCII-letter/space/newline text, concatenating the body Tj
        /// strings (everything after the title) and dropping spaces recovers the
        /// input's words in order — wrapping only inserts breaks, never drops or
        /// reorders glyphs.
        #[test]
        fn round_trip_body_preserves_words(s in "[a-zA-Z \n]{0,200}") {
            let out = text_to_pdf_native(&s, "{}").unwrap();
            let tjs = tj_strings(&out);
            prop_assert!(!tjs.is_empty());
            // tjs[0] is the title ("Document"); the rest is the body.
            let body: Vec<u8> = tjs[1..]
                .concat()
                .into_iter()
                .filter(|&b| b != b' ')
                .collect();
            let words: Vec<u8> = s.split_whitespace().flat_map(|w| w.bytes()).collect();
            prop_assert_eq!(body, words);
        }

        /// Output is always a valid, >=1-page PDF for arbitrary unicode input.
        #[test]
        fn always_valid_pdf(s in ".{0,300}") {
            let out = text_to_pdf_native(&s, "{}").unwrap();
            prop_assert!(out.starts_with(b"%PDF-"));
            prop_assert!(page_count(&out) >= 1);
        }

        /// Every wrapped segment fits within max_width, unless it is a single
        /// glyph that intrinsically cannot (the hard-break terminal case).
        #[test]
        fn wrapline_segments_fit_or_single_char(
            line in "[a-zA-Z ]{0,80}",
            max_width in 5.0f32..400.0f32,
            font_size in 8.0f32..16.0f32,
        ) {
            for seg in wrap_line(&line, max_width, &HELVETICA_WIDTHS, font_size) {
                let w = text_width_pt(&encode_winansi(&seg), &HELVETICA_WIDTHS, font_size);
                prop_assert!(
                    w <= max_width + 0.001 || seg.chars().count() <= 1,
                    "segment {seg:?} width {w} exceeds {max_width}"
                );
            }
        }
    }
}
