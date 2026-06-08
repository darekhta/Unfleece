//! Text stamping — page numbers (incl. Bates-style numbering) and text watermarks.
//!
//! Rust port of `src/lib/tools/annotate.ts` (`addPageNumbers`, `addTextWatermark`)
//! with behavior parity:
//! - **Page numbers**: `{n}`/`{total}` format template, 6-zone position
//!   (`bottom-center` default), Helvetica at `fontSize` pt in colour
//!   `rgb(0.1, 0.1, 0.1)`, plus `startAt`/`padTo` offsets. These two options also
//!   power the **Bates numbering** tool — the TS app has no separate Bates
//!   implementation; `run.ts` calls page numbers with `format: "PREFIX{n}"`,
//!   `startAt` and `padTo`, which this port supports identically.
//! - **Watermark**: Helvetica-Bold text whose baseline midpoint is anchored at
//!   the page center, rotated by `angle` degrees, with `/ca` + `/CA` opacity via
//!   an ExtGState (defaults: 50 pt, 0.25 opacity, 45°, colour 0.6/0.6/0.6).
//!
//! Text is measured with embedded Adobe Core-14 AFM width tables (WinAnsi
//! encoding) — the same metrics pdf-lib uses. Characters outside WinAnsi are
//! stamped as `?` and control characters are stripped; AFM kern pairs are
//! ignored (they shift pdf-lib's centring by well under a point).
//!
//! Each stamp is appended as a fresh content stream; any existing page content
//! is first wrapped in `q`/`Q` so unbalanced graphics state in the original
//! streams cannot displace the stamp (mirrors pdf-lib's page normalization).

use lopdf::content::Operation;
use lopdf::{dictionary, Document, Object, ObjectId};
use serde::Deserialize;

use crate::content::{
    add_content_stream, add_resource_entry, append_to_page_contents, wrap_stream_ids,
};
use crate::util::{effective_media_box, materialize_inherited_page_attrs, save_compact};

/// The 6 supported stamp zones.
const POSITIONS: [&str; 6] = [
    "bottom-center",
    "bottom-right",
    "bottom-left",
    "top-center",
    "top-right",
    "top-left",
];

/// Page-number colour used by the TS tool: `rgb(0.1, 0.1, 0.1)`.
const PAGE_NUMBER_COLOR: [f32; 3] = [0.1, 0.1, 0.1];

// ---------------------------------------------------------------------------
// Adobe Core-14 AFM width tables (WinAnsi encoding, codes 0x20..=0xFF),
// in 1/1000 em units. Codes undefined in WinAnsi are 0 (never emitted).
// ---------------------------------------------------------------------------

/// Helvetica glyph advance widths for WinAnsi codes 0x20..=0xFF.
pub(crate) const HELVETICA_WIDTHS: [u16; 224] = [
    // 0x20 (space) .. 0x2F
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278,
    // 0x30 (0) .. 0x3F (?)
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556,
    // 0x40 (@) .. 0x4F (O)
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778,
    // 0x50 (P) .. 0x5F (_)
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556,
    // 0x60 (`) .. 0x6F (o)
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556,
    // 0x70 (p) .. 0x7F
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584, 0,
    // 0x80 (Euro) .. 0x8F
    556, 0, 222, 556, 333, 1000, 556, 556, 333, 1000, 667, 333, 1000, 0, 611, 0,
    // 0x90 .. 0x9F (Ydieresis)
    0, 222, 222, 333, 333, 350, 556, 1000, 333, 1000, 500, 333, 944, 0, 500, 667,
    // 0xA0 (nbsp) .. 0xAF (macron)
    278, 333, 556, 556, 556, 556, 260, 556, 333, 737, 370, 556, 584, 333, 737, 333,
    // 0xB0 (degree) .. 0xBF (questiondown)
    400, 584, 333, 333, 333, 556, 537, 278, 333, 333, 365, 556, 834, 834, 834, 611,
    // 0xC0 (Agrave) .. 0xCF (Idieresis)
    667, 667, 667, 667, 667, 667, 1000, 722, 667, 667, 667, 667, 278, 278, 278, 278,
    // 0xD0 (Eth) .. 0xDF (germandbls)
    722, 722, 778, 778, 778, 778, 778, 584, 778, 722, 722, 722, 722, 667, 667, 611,
    // 0xE0 (agrave) .. 0xEF (idieresis)
    556, 556, 556, 556, 556, 556, 889, 500, 556, 556, 556, 556, 278, 278, 278, 278,
    // 0xF0 (eth) .. 0xFF (ydieresis)
    556, 556, 556, 556, 556, 556, 556, 584, 611, 556, 556, 556, 556, 500, 556, 500,
];

/// Helvetica-Bold glyph advance widths for WinAnsi codes 0x20..=0xFF.
pub(crate) const HELVETICA_BOLD_WIDTHS: [u16; 224] = [
    // 0x20 (space) .. 0x2F
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278,
    // 0x30 (0) .. 0x3F (?)
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611,
    // 0x40 (@) .. 0x4F (O)
    975, 722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778,
    // 0x50 (P) .. 0x5F (_)
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 333, 278, 333, 584, 556,
    // 0x60 (`) .. 0x6F (o)
    333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, 611, 611,
    // 0x70 (p) .. 0x7F
    611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584, 0,
    // 0x80 (Euro) .. 0x8F
    556, 0, 278, 556, 500, 1000, 556, 556, 333, 1000, 667, 333, 1000, 0, 611, 0,
    // 0x90 .. 0x9F (Ydieresis)
    0, 278, 278, 500, 500, 350, 556, 1000, 333, 1000, 556, 333, 944, 0, 500, 667,
    // 0xA0 (nbsp) .. 0xAF (macron)
    278, 333, 556, 556, 556, 556, 280, 556, 333, 737, 370, 556, 584, 333, 737, 333,
    // 0xB0 (degree) .. 0xBF (questiondown)
    400, 584, 333, 333, 333, 611, 556, 278, 333, 333, 365, 556, 834, 834, 834, 611,
    // 0xC0 (Agrave) .. 0xCF (Idieresis)
    722, 722, 722, 722, 722, 722, 1000, 722, 667, 667, 667, 667, 278, 278, 278, 278,
    // 0xD0 (Eth) .. 0xDF (germandbls)
    722, 722, 778, 778, 778, 778, 778, 584, 778, 722, 722, 722, 722, 667, 667, 611,
    // 0xE0 (agrave) .. 0xEF (idieresis)
    556, 556, 556, 556, 556, 556, 889, 556, 556, 556, 556, 556, 278, 278, 278, 278,
    // 0xF0 (eth) .. 0xFF (ydieresis)
    611, 611, 611, 611, 611, 611, 611, 584, 611, 611, 611, 611, 611, 556, 611, 556,
];

// ---------------------------------------------------------------------------
// Options (camelCase JSON, mirroring annotate.ts)
// ---------------------------------------------------------------------------

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageNumberOpts {
    /// Template; `{n}` = page number, `{total}` = page count. Default `"{n}"`.
    format: Option<String>,
    /// One of the 6 [`POSITIONS`]. Default `"bottom-center"`.
    position: Option<String>,
    /// Default 10 pt.
    font_size: Option<f32>,
    /// Distance from the page edge, default 24 pt.
    margin: Option<f32>,
    /// Number shown on the first page, default 1.
    start_at: Option<i64>,
    /// Zero-pad numbers to this many characters, default 0 (no padding).
    pad_to: Option<f64>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WatermarkOpts {
    /// Required. Empty/missing text is rejected.
    text: Option<String>,
    /// Default 50 pt.
    font_size: Option<f32>,
    /// Fill + stroke alpha, default 0.25, clamped to [0, 1].
    opacity: Option<f32>,
    /// Rotation in degrees (counter-clockwise), default 45.
    angle: Option<f32>,
    /// Default grey `{ r: 0.6, g: 0.6, b: 0.6 }`; channels clamped to [0, 1].
    color: Option<ColorOpt>,
}

#[derive(Deserialize)]
#[serde(default)]
struct ColorOpt {
    r: f32,
    g: f32,
    b: f32,
}

impl Default for ColorOpt {
    fn default() -> Self {
        Self {
            r: 0.6,
            g: 0.6,
            b: 0.6,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Stamp page numbers onto every page.
///
/// `opts_json` mirrors the TS `PageNumberOptions`:
/// `{ format?, position?, fontSize?, margin?, startAt?, padTo? }`. The format
/// template substitutes `{n}` (zero-padded to `padTo` characters when > 0) and
/// `{total}`; unknown placeholders are left verbatim. Bates numbering is this
/// same call with `format: "PREFIX{n}"`, `startAt` and `padTo`.
pub fn add_page_numbers_native(data: &[u8], opts_json: &str) -> Result<Vec<u8>, String> {
    let opts: PageNumberOpts =
        serde_json::from_str(opts_json).map_err(|e| format!("Invalid page number options: {e}"))?;
    let format = opts.format.unwrap_or_else(|| "{n}".to_string());
    let position = opts.position.unwrap_or_else(|| "bottom-center".to_string());
    if !POSITIONS.contains(&position.as_str()) {
        return Err(format!(
            "Invalid position '{position}'; expected one of: {}",
            POSITIONS.join(", ")
        ));
    }
    let font_size = opts.font_size.unwrap_or(10.0);
    let margin = opts.margin.unwrap_or(24.0);
    let start_at = opts.start_at.unwrap_or(1);
    let pad_to = opts.pad_to.unwrap_or(0.0);

    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    let pages: Vec<ObjectId> = doc.page_iter().collect();
    let total = pages.len().to_string();

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });
    let (push_id, pop_id) = wrap_stream_ids(&mut doc)?;

    for (i, &page_id) in pages.iter().enumerate() {
        let n = pad_number((i as i64).saturating_add(start_at), pad_to);
        let text = format.replace("{n}", &n).replace("{total}", &total);
        let encoded = encode_winansi(&text);
        let text_width = text_width_pt(&encoded, &HELVETICA_WIDTHS, font_size);

        materialize_inherited_page_attrs(&mut doc, page_id).map_err(|e| e.to_string())?;
        let mb = effective_media_box(&doc, page_id).map_err(|e| e.to_string())?;
        let (width, height) = (mb[2] - mb[0], mb[3] - mb[1]);

        let x = if position.ends_with("center") {
            (width - text_width) / 2.0
        } else if position.ends_with("right") {
            width - margin - text_width
        } else {
            margin
        };
        let y = if position.starts_with("top") {
            height - margin - font_size
        } else {
            margin
        };

        let font_key =
            add_resource_entry(&mut doc, page_id, "Font", "UFt", Object::Reference(font_id))?;
        let ops = text_stamp_ops(
            &font_key,
            None,
            font_size,
            PAGE_NUMBER_COLOR,
            [1.0, 0.0, 0.0, 1.0, x, y],
            encoded,
        );
        let stamp_id = add_content_stream(&mut doc, ops)?;
        append_to_page_contents(&mut doc, page_id, stamp_id, push_id, pop_id)?;
    }

    save_compact(doc).map_err(|e| e.to_string())
}

/// Draw a rotated text watermark centered on every page.
///
/// `opts_json` mirrors the TS `WatermarkOptions`:
/// `{ text, fontSize?, opacity?, angle?, color? }`. The text's baseline midpoint
/// is anchored at the page center and rotated by `angle` degrees; opacity is
/// applied through a shared `/ca` + `/CA` ExtGState.
pub fn add_watermark_native(data: &[u8], opts_json: &str) -> Result<Vec<u8>, String> {
    let opts: WatermarkOpts =
        serde_json::from_str(opts_json).map_err(|e| format!("Invalid watermark options: {e}"))?;
    let text = opts.text.unwrap_or_default();
    if text.is_empty() {
        return Err("Watermark text is required".to_string());
    }
    let font_size = opts.font_size.unwrap_or(50.0);
    let opacity = opts.opacity.unwrap_or(0.25).clamp(0.0, 1.0);
    let angle = opts.angle.unwrap_or(45.0);
    let color = opts.color.unwrap_or_default();
    let rgb = [
        color.r.clamp(0.0, 1.0),
        color.g.clamp(0.0, 1.0),
        color.b.clamp(0.0, 1.0),
    ];

    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    let pages: Vec<ObjectId> = doc.page_iter().collect();

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica-Bold",
        "Encoding" => "WinAnsiEncoding",
    });
    let gs_id = doc.add_object(dictionary! {
        "Type" => "ExtGState",
        "ca" => opacity,
        "CA" => opacity,
    });
    let (push_id, pop_id) = wrap_stream_ids(&mut doc)?;

    let rad = angle.to_radians();
    let (sin, cos) = rad.sin_cos();
    let encoded = encode_winansi(&text);
    let text_width = text_width_pt(&encoded, &HELVETICA_BOLD_WIDTHS, font_size);

    for &page_id in &pages {
        materialize_inherited_page_attrs(&mut doc, page_id).map_err(|e| e.to_string())?;
        let mb = effective_media_box(&doc, page_id).map_err(|e| e.to_string())?;
        let (width, height) = (mb[2] - mb[0], mb[3] - mb[1]);

        // Anchor so the text's baseline midpoint sits at the page center.
        let x = width / 2.0 - (text_width / 2.0) * cos;
        let y = height / 2.0 - (text_width / 2.0) * sin;

        let font_key =
            add_resource_entry(&mut doc, page_id, "Font", "UFt", Object::Reference(font_id))?;
        let gs_key = add_resource_entry(
            &mut doc,
            page_id,
            "ExtGState",
            "UFgs",
            Object::Reference(gs_id),
        )?;
        let ops = text_stamp_ops(
            &font_key,
            Some(&gs_key),
            font_size,
            rgb,
            [cos, sin, -sin, cos, x, y],
            encoded.clone(),
        );
        let stamp_id = add_content_stream(&mut doc, ops)?;
        append_to_page_contents(&mut doc, page_id, stamp_id, push_id, pop_id)?;
    }

    save_compact(doc).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Text encoding + measurement
// ---------------------------------------------------------------------------

/// Map a char to its WinAnsi (CP-1252) code; `None` when unrepresentable.
fn winansi_code(ch: char) -> Option<u8> {
    match ch as u32 {
        cp @ 0x20..=0x7E => Some(cp as u8),
        cp @ 0xA0..=0xFF => Some(cp as u8),
        _ => match ch {
            '\u{20AC}' => Some(0x80), // €
            '\u{201A}' => Some(0x82), // ‚
            '\u{0192}' => Some(0x83), // ƒ
            '\u{201E}' => Some(0x84), // „
            '\u{2026}' => Some(0x85), // …
            '\u{2020}' => Some(0x86), // †
            '\u{2021}' => Some(0x87), // ‡
            '\u{02C6}' => Some(0x88), // ˆ
            '\u{2030}' => Some(0x89), // ‰
            '\u{0160}' => Some(0x8A), // Š
            '\u{2039}' => Some(0x8B), // ‹
            '\u{0152}' => Some(0x8C), // Œ
            '\u{017D}' => Some(0x8E), // Ž
            '\u{2018}' => Some(0x91), // '
            '\u{2019}' => Some(0x92), // '
            '\u{201C}' => Some(0x93), // "
            '\u{201D}' => Some(0x94), // "
            '\u{2022}' => Some(0x95), // •
            '\u{2013}' => Some(0x96), // –
            '\u{2014}' => Some(0x97), // —
            '\u{02DC}' => Some(0x98), // ˜
            '\u{2122}' => Some(0x99), // ™
            '\u{0161}' => Some(0x9A), // š
            '\u{203A}' => Some(0x9B), // ›
            '\u{0153}' => Some(0x9C), // œ
            '\u{017E}' => Some(0x9E), // ž
            '\u{0178}' => Some(0x9F), // Ÿ
            _ => None,
        },
    }
}

/// Encode text as WinAnsi bytes: control characters are stripped and
/// unrepresentable characters become `?` (pdf-lib would throw instead).
pub(crate) fn encode_winansi(text: &str) -> Vec<u8> {
    text.chars()
        .filter(|c| !c.is_control())
        .map(|c| winansi_code(c).unwrap_or(b'?'))
        .collect()
}

/// Advance width in pt of WinAnsi-encoded text in the given AFM table.
pub(crate) fn text_width_pt(encoded: &[u8], widths: &[u16; 224], font_size: f32) -> f32 {
    let units: u32 = encoded
        .iter()
        .map(|&b| u32::from(widths[(b - 0x20) as usize]))
        .sum();
    units as f32 / 1000.0 * font_size
}

/// JS-parity zero padding: `String(n).padStart(max(0, floor(padTo)), '0')` —
/// the pad width counts every character, including any minus sign.
fn pad_number(n: i64, pad_to: f64) -> String {
    let s = n.to_string();
    let pad = pad_to.floor().clamp(0.0, 1000.0) as usize;
    if s.len() >= pad {
        s
    } else {
        format!("{}{s}", "0".repeat(pad - s.len()))
    }
}

// ---------------------------------------------------------------------------
// Text stamp operators (content/resource plumbing lives in `crate::content`)
// ---------------------------------------------------------------------------

/// The self-contained operator sequence for one text stamp, mirroring
/// pdf-lib's `drawText`: `q [gs] BT rg Tf Tm Tj ET Q`.
fn text_stamp_ops(
    font_key: &str,
    gs_key: Option<&str>,
    font_size: f32,
    color: [f32; 3],
    tm: [f32; 6],
    encoded_text: Vec<u8>,
) -> Vec<Operation> {
    let mut ops = vec![Operation::new("q", vec![])];
    if let Some(gs) = gs_key {
        ops.push(Operation::new(
            "gs",
            vec![Object::Name(gs.as_bytes().to_vec())],
        ));
    }
    ops.extend([
        Operation::new("BT", vec![]),
        Operation::new(
            "rg",
            vec![
                Object::Real(color[0]),
                Object::Real(color[1]),
                Object::Real(color[2]),
            ],
        ),
        Operation::new(
            "Tf",
            vec![
                Object::Name(font_key.as_bytes().to_vec()),
                Object::Real(font_size),
            ],
        ),
        Operation::new("Tm", tm.iter().map(|&v| Object::Real(v)).collect()),
        Operation::new("Tj", vec![Object::string_literal(encoded_text)]),
        Operation::new("ET", vec![]),
        Operation::new("Q", vec![]),
    ]);
    ops
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fixtures::{page_content_text, sample, sample_with_text};
    use crate::util::number_as_f32;
    use lopdf::content::Content;
    use lopdf::{Dictionary, Stream};

    // -- helpers ------------------------------------------------------------

    fn n_pages(data: &[u8]) -> usize {
        Document::load_mem(data).unwrap().get_pages().len()
    }

    fn ops_of(data: &[u8], page: usize) -> Vec<Operation> {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let content = doc.get_page_content(pages[page]).unwrap();
        Content::decode(&content).unwrap().operations
    }

    fn last_op<'a>(ops: &'a [Operation], name: &str) -> &'a Operation {
        ops.iter()
            .rev()
            .find(|o| o.operator == name)
            .unwrap_or_else(|| panic!("no {name} op"))
    }

    fn count_op(ops: &[Operation], name: &str) -> usize {
        ops.iter().filter(|o| o.operator == name).count()
    }

    fn fnum(o: &Object) -> f32 {
        number_as_f32(o).unwrap()
    }

    #[track_caller]
    fn assert_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.05,
            "expected {expected}, got {actual}"
        );
    }

    /// `(ca, CA)` of the first ExtGState reachable from page `page`'s Resources.
    fn extgstate_alpha(data: &[u8], page: usize) -> (f32, f32) {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let page_dict = doc.get_dictionary(pages[page]).unwrap();
        let res = match page_dict.get(b"Resources").unwrap() {
            Object::Reference(id) => doc.get_dictionary(*id).unwrap(),
            Object::Dictionary(d) => d,
            other => panic!("unexpected Resources: {other:?}"),
        };
        let gs = res.get(b"ExtGState").unwrap().as_dict().unwrap();
        let (_, first) = gs.iter().next().unwrap();
        let gd = match first {
            Object::Reference(id) => doc.get_dictionary(*id).unwrap(),
            Object::Dictionary(d) => d,
            other => panic!("unexpected ExtGState entry: {other:?}"),
        };
        (fnum(gd.get(b"ca").unwrap()), fnum(gd.get(b"CA").unwrap()))
    }

    /// One page (300×400), `/Contents` already an array of two streams.
    fn pdf_with_contents_array() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let s1 = doc.add_object(Stream::new(dictionary! {}, b"q\n".to_vec()));
        let s2 = doc.add_object(Stream::new(dictionary! {}, b"Q\n".to_vec()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => vec![s1.into(), s2.into()],
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 400.into()],
        });
        let pages = dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    /// One page with NO `/Contents` and an *indirect* (referenced) Resources dict.
    fn pdf_without_contents_indirect_resources() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let res_id = doc.add_object(dictionary! {});
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Resources" => res_id,
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 400.into()],
        });
        let pages = dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    fn one_page_pdf(mut page: Dictionary) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        page.set("Type", "Page");
        page.set("Parent", pages_id);
        if page.get(b"MediaBox").is_err() {
            page.set("MediaBox", vec![0.into(), 0.into(), 300.into(), 400.into()]);
        }
        let page_id = doc.add_object(page);
        let pages = dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    fn pdf_with_resource_reference_to_non_dict() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let bad_resources_id = doc.add_object(Object::Integer(42));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Resources" => bad_resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 400.into()],
        });
        let pages = dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    fn pdf_with_font_category_reference_to_non_dict() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let bad_font_id = doc.add_object(Object::Integer(42));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Resources" => dictionary! { "Font" => bad_font_id },
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 400.into()],
        });
        let pages = dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    fn pdf_with_indirect_font_category_collision() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_category_id = doc.add_object(dictionary! {
            "UFt" => dictionary! {
                "Type" => "Font",
                "Subtype" => "Type1",
                "BaseFont" => "Courier",
            },
        });
        let resources_id = doc.add_object(dictionary! { "Font" => font_category_id });
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 400.into()],
        });
        let pages = dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    fn tf_font_name(data: &[u8]) -> Vec<u8> {
        let ops = ops_of(data, 0);
        match &last_op(&ops, "Tf").operands[0] {
            Object::Name(name) => name.clone(),
            other => panic!("unexpected Tf font operand: {other:?}"),
        }
    }

    // -- page numbers ---------------------------------------------------------

    #[test]
    fn page_numbers_stamp_every_page_with_defaults() {
        let out = add_page_numbers_native(&sample(3), "{}").unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(n_pages(&out), 3);
        for (i, expected) in ["(1)", "(2)", "(3)"].iter().enumerate() {
            let text = page_content_text(&out, i);
            assert!(
                text.contains(expected),
                "page {i} missing {expected}: {text}"
            );
        }
        // Defaults: 10 pt Helvetica in rgb(0.1, 0.1, 0.1).
        let ops = ops_of(&out, 0);
        assert_near(fnum(&last_op(&ops, "Tf").operands[1]), 10.0);
        let rg = last_op(&ops, "rg");
        for ch in 0..3 {
            assert_near(fnum(&rg.operands[ch]), 0.1);
        }
        assert!(out.windows(b"Helvetica".len()).any(|w| w == b"Helvetica"));
    }

    #[test]
    fn page_numbers_format_supports_total_and_literals() {
        let out =
            add_page_numbers_native(&sample(3), r#"{"format":"Page {n} of {total}"}"#).unwrap();
        assert!(page_content_text(&out, 1).contains("(Page 2 of 3)"));
        // Unknown placeholders are left verbatim (JS only replaces {n}/{total}).
        let out = add_page_numbers_native(&sample(1), r#"{"format":"{n}{x}"}"#).unwrap();
        assert!(page_content_text(&out, 0).contains("1{x}"));
    }

    #[test]
    fn page_numbers_all_six_positions_place_text_correctly() {
        // Page 300×400; "1" at 10 pt = 5.56 pt wide; margin 24.
        let cases = [
            ("bottom-center", 147.22, 24.0),
            ("bottom-right", 270.44, 24.0),
            ("bottom-left", 24.0, 24.0),
            ("top-center", 147.22, 366.0),
            ("top-right", 270.44, 366.0),
            ("top-left", 24.0, 366.0),
        ];
        for (position, x, y) in cases {
            let opts = format!(r#"{{"position":"{position}"}}"#);
            let out = add_page_numbers_native(&sample(1), &opts).unwrap();
            let ops = ops_of(&out, 0);
            let tm = last_op(&ops, "Tm");
            assert_near(fnum(&tm.operands[4]), x);
            assert_near(fnum(&tm.operands[5]), y);
        }
    }

    #[test]
    fn page_numbers_invalid_position_is_rejected() {
        let err = add_page_numbers_native(&sample(1), r#"{"position":"middle"}"#).unwrap_err();
        assert!(err.contains("Invalid position 'middle'"), "{err}");
    }

    #[test]
    fn page_numbers_start_at_offsets_numbering() {
        let out = add_page_numbers_native(&sample(2), r#"{"startAt":5}"#).unwrap();
        assert!(page_content_text(&out, 0).contains("(5)"));
        assert!(page_content_text(&out, 1).contains("(6)"));
    }

    #[test]
    fn page_numbers_pad_to_zero_pads() {
        let out = add_page_numbers_native(&sample(1), r#"{"padTo":3}"#).unwrap();
        assert!(page_content_text(&out, 0).contains("(001)"));
        // padTo 0 (default) and fractional padTo floor like JS Math.floor.
        let out = add_page_numbers_native(&sample(1), r#"{"padTo":2.9}"#).unwrap();
        assert!(page_content_text(&out, 0).contains("(01)"));
    }

    #[test]
    fn page_numbers_bates_style_prefix_pad() {
        // Exactly how run.ts drives the Bates tool: format "PREFIX{n}" + startAt/padTo.
        let opts = r#"{"format":"BATES-{n}","position":"bottom-right","fontSize":9,"margin":24,"startAt":1,"padTo":6}"#;
        let out = add_page_numbers_native(&sample(2), opts).unwrap();
        assert!(page_content_text(&out, 0).contains("(BATES-000001)"));
        assert!(page_content_text(&out, 1).contains("(BATES-000002)"));
    }

    #[test]
    fn page_numbers_camel_case_font_size_and_margin_options() {
        let out = add_page_numbers_native(
            &sample(1),
            r#"{"fontSize":8,"margin":10,"position":"top-left"}"#,
        )
        .unwrap();
        let ops = ops_of(&out, 0);
        assert_near(fnum(&last_op(&ops, "Tf").operands[1]), 8.0);
        let tm = last_op(&ops, "Tm");
        assert_near(fnum(&tm.operands[4]), 10.0); // x = margin
        assert_near(fnum(&tm.operands[5]), 400.0 - 10.0 - 8.0); // y = h - margin - fontSize
    }

    #[test]
    fn page_numbers_preserve_existing_content_wrapped_in_q_q() {
        let out = add_page_numbers_native(&sample_with_text(1, Some("Hello")), "{}").unwrap();
        let text = page_content_text(&out, 0);
        assert!(text.contains("Hello 0"), "original content lost: {text}");
        let ops = ops_of(&out, 0);
        assert_eq!(ops[0].operator, "q", "existing content not bracketed");
        assert_eq!(count_op(&ops, "Tj"), 2); // original text + stamp
                                             // The stamp is the LAST Tj and shows "1".
        match &last_op(&ops, "Tj").operands[0] {
            Object::String(bytes, _) => assert_eq!(bytes, b"1"),
            other => panic!("unexpected Tj operand: {other:?}"),
        }
    }

    #[test]
    fn page_numbers_handle_contents_array_pages() {
        let out = add_page_numbers_native(&pdf_with_contents_array(), "{}").unwrap();
        assert_eq!(n_pages(&out), 1);
        let ops = ops_of(&out, 0);
        // wrapper q + original q + stamp q; wrapper Q + original Q + stamp Q.
        assert_eq!(count_op(&ops, "q"), 3);
        assert_eq!(count_op(&ops, "Q"), 3);
        assert!(page_content_text(&out, 0).contains("(1)"));
    }

    #[test]
    fn page_numbers_handle_missing_contents_and_indirect_resources() {
        let out =
            add_page_numbers_native(&pdf_without_contents_indirect_resources(), "{}").unwrap();
        assert_eq!(n_pages(&out), 1);
        assert!(page_content_text(&out, 0).contains("(1)"));
        // The font landed in the (indirect) Resources dict and is reachable.
        let doc = Document::load_mem(&out).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let res = match doc
            .get_dictionary(pages[0])
            .unwrap()
            .get(b"Resources")
            .unwrap()
        {
            Object::Reference(id) => doc.get_dictionary(*id).unwrap(),
            Object::Dictionary(d) => d,
            other => panic!("unexpected Resources: {other:?}"),
        };
        assert_eq!(res.get(b"Font").unwrap().as_dict().unwrap().len(), 1);
    }

    #[test]
    fn page_numbers_zero_page_pdf_is_noop() {
        let out = add_page_numbers_native(&sample(0), "{}").unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(n_pages(&out), 0);
    }

    #[test]
    fn page_numbers_invalid_json_options_rejected() {
        let err = add_page_numbers_native(&sample(1), "not json").unwrap_err();
        assert!(err.starts_with("Invalid page number options:"), "{err}");
    }

    #[test]
    fn page_numbers_invalid_pdf_rejected() {
        assert!(add_page_numbers_native(b"not a pdf", "{}").is_err());
    }

    #[test]
    fn page_numbers_winansi_encoding_and_fallback() {
        // U+2116 (№) is outside WinAnsi -> '?'.
        let out = add_page_numbers_native(&sample(1), r#"{"format":"№{n}"}"#).unwrap();
        assert!(page_content_text(&out, 0).contains("(?1)"));
        // U+2013 (en dash) maps to WinAnsi 0x96.
        let out = add_page_numbers_native(&sample(1), r#"{"format":"–{n}"}"#).unwrap();
        let ops = ops_of(&out, 0);
        match &last_op(&ops, "Tj").operands[0] {
            Object::String(bytes, _) => assert_eq!(bytes, &[0x96, b'1']),
            other => panic!("unexpected Tj operand: {other:?}"),
        }
    }

    #[test]
    fn winansi_encoding_maps_cp1252_specials_and_filters_controls() {
        let special = concat!(
            "\u{20AC}", "\u{201A}", "\u{0192}", "\u{201E}", "\u{2026}", "\u{2020}", "\u{2021}",
            "\u{02C6}", "\u{2030}", "\u{0160}", "\u{2039}", "\u{0152}", "\u{017D}", "\u{2018}",
            "\u{2019}", "\u{201C}", "\u{201D}", "\u{2022}", "\u{2013}", "\u{2014}", "\u{02DC}",
            "\u{2122}", "\u{0161}", "\u{203A}", "\u{0153}", "\u{017E}", "\u{0178}"
        );
        assert_eq!(
            encode_winansi(special),
            vec![
                0x80, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x8B, 0x8C, 0x8E, 0x91,
                0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0x9B, 0x9C, 0x9E, 0x9F,
            ]
        );
        assert_eq!(encode_winansi("A\n№B"), b"A?B");
    }

    #[test]
    fn page_numbers_reject_malformed_resource_shapes() {
        let err = add_page_numbers_native(&one_page_pdf(dictionary! { "Resources" => 42 }), "{}")
            .unwrap_err();
        assert_eq!(err, "Page /Resources is not a dictionary");

        let err =
            add_page_numbers_native(&pdf_with_resource_reference_to_non_dict(), "{}").unwrap_err();
        assert_eq!(err, "Page /Resources reference is not a dictionary");

        let err = add_page_numbers_native(
            &one_page_pdf(dictionary! { "Resources" => dictionary! { "Font" => 42 } }),
            "{}",
        )
        .unwrap_err();
        assert_eq!(err, "Page /Font resource entry is not a dictionary");

        let err = add_page_numbers_native(&pdf_with_font_category_reference_to_non_dict(), "{}")
            .unwrap_err();
        assert_eq!(err, "Page /Font resource reference is not a dictionary");
    }

    #[test]
    fn page_numbers_avoid_font_key_collisions_in_inline_resources() {
        let pdf = one_page_pdf(dictionary! {
            "Resources" => dictionary! {
                "Font" => dictionary! {
                    "UFt" => dictionary! {
                        "Type" => "Font",
                        "Subtype" => "Type1",
                        "BaseFont" => "Courier",
                    },
                },
            },
        });
        let out = add_page_numbers_native(&pdf, "{}").unwrap();
        assert_eq!(tf_font_name(&out), b"UFt1");
    }

    #[test]
    fn page_numbers_avoid_font_key_collisions_in_indirect_resources() {
        let out =
            add_page_numbers_native(&pdf_with_indirect_font_category_collision(), "{}").unwrap();
        assert_eq!(tf_font_name(&out), b"UFt1");

        let doc = Document::load_mem(&out).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let resources_id = doc
            .get_dictionary(pages[0])
            .unwrap()
            .get(b"Resources")
            .unwrap()
            .as_reference()
            .unwrap();
        let font_id = doc
            .get_dictionary(resources_id)
            .unwrap()
            .get(b"Font")
            .unwrap()
            .as_reference()
            .unwrap();
        let fonts = doc.get_dictionary(font_id).unwrap();
        assert!(fonts.get(b"UFt").is_ok());
        assert!(fonts.get(b"UFt1").is_ok());
    }

    #[test]
    fn page_numbers_reject_invalid_contents_object() {
        let err = add_page_numbers_native(&one_page_pdf(dictionary! { "Contents" => 42 }), "{}")
            .unwrap_err();
        assert_eq!(err, "Page /Contents is not a stream or array");
    }

    #[test]
    fn watermark_rejects_malformed_extgstate_resources() {
        let err = add_watermark_native(
            &one_page_pdf(dictionary! {
                "Resources" => dictionary! { "ExtGState" => 42 },
            }),
            r#"{"text":"X"}"#,
        )
        .unwrap_err();
        assert_eq!(err, "Page /ExtGState resource entry is not a dictionary");
    }

    #[test]
    fn watermark_avoids_extgstate_key_collisions() {
        let out = add_watermark_native(
            &one_page_pdf(dictionary! {
                "Resources" => dictionary! {
                    "ExtGState" => dictionary! {
                        "UFgs" => dictionary! { "ca" => 0.5, "CA" => 0.5 },
                    },
                },
            }),
            r#"{"text":"X"}"#,
        )
        .unwrap();
        let ops = ops_of(&out, 0);
        match &last_op(&ops, "gs").operands[0] {
            Object::Name(name) => assert_eq!(name, b"UFgs1"),
            other => panic!("unexpected gs operand: {other:?}"),
        }
    }

    #[test]
    fn append_contents_materializes_inline_streams() {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => Object::Stream(Stream::new(dictionary! {}, b"q\n".to_vec())),
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 400.into()],
        });
        let stamp_id = doc.add_object(Stream::new(dictionary! {}, b"BT (1) Tj ET\n".to_vec()));
        let (push_id, pop_id) = wrap_stream_ids(&mut doc).unwrap();

        append_to_page_contents(&mut doc, page_id, stamp_id, push_id, pop_id).unwrap();

        let contents = doc
            .get_dictionary(page_id)
            .unwrap()
            .get(b"Contents")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(contents.len(), 4);
        assert_eq!(contents[0].as_reference().unwrap(), push_id);
        assert_eq!(contents[2].as_reference().unwrap(), pop_id);
        assert_eq!(contents[3].as_reference().unwrap(), stamp_id);

        let inline_id = contents[1].as_reference().unwrap();
        assert_eq!(
            doc.get_object(inline_id)
                .unwrap()
                .as_stream()
                .unwrap()
                .content,
            b"q\n"
        );
    }

    #[test]
    fn width_metrics_match_adobe_afm() {
        // Helvetica "Hello World" @ 12 pt = 5167/1000 * 12 = 62.004 pt.
        assert_near(
            text_width_pt(&encode_winansi("Hello World"), &HELVETICA_WIDTHS, 12.0),
            62.004,
        );
        // Helvetica-Bold "WM" @ 50 pt = (944 + 833)/1000 * 50 = 88.85 pt.
        assert_near(
            text_width_pt(&encode_winansi("WM"), &HELVETICA_BOLD_WIDTHS, 50.0),
            88.85,
        );
    }

    // -- watermark ------------------------------------------------------------

    #[test]
    fn watermark_stamps_every_page_with_defaults() {
        let out = add_watermark_native(&sample(2), r#"{"text":"DRAFT"}"#).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(n_pages(&out), 2);
        for i in 0..2 {
            assert!(page_content_text(&out, i).contains("(DRAFT)"));
            let ops = ops_of(&out, i);
            assert_eq!(count_op(&ops, "gs"), 1, "opacity gs op missing on page {i}");
            assert_near(fnum(&last_op(&ops, "Tf").operands[1]), 50.0);
            // Default colour 0.6 grey.
            let rg = last_op(&ops, "rg");
            for ch in 0..3 {
                assert_near(fnum(&rg.operands[ch]), 0.6);
            }
        }
        assert!(out
            .windows(b"Helvetica-Bold".len())
            .any(|w| w == b"Helvetica-Bold"));
    }

    #[test]
    fn watermark_missing_or_empty_text_rejected_with_exact_message() {
        assert_eq!(
            add_watermark_native(&sample(1), "{}").unwrap_err(),
            "Watermark text is required"
        );
        assert_eq!(
            add_watermark_native(&sample(1), r#"{"text":""}"#).unwrap_err(),
            "Watermark text is required"
        );
    }

    #[test]
    fn watermark_default_opacity_extgstate() {
        let out = add_watermark_native(&sample(1), r#"{"text":"X"}"#).unwrap();
        let (ca, cap) = extgstate_alpha(&out, 0);
        assert_near(ca, 0.25);
        assert_near(cap, 0.25);
    }

    #[test]
    fn watermark_opacity_clamped_to_unit_range() {
        let out = add_watermark_native(&sample(1), r#"{"text":"X","opacity":5}"#).unwrap();
        assert_near(extgstate_alpha(&out, 0).0, 1.0);
        let out = add_watermark_native(&sample(1), r#"{"text":"X","opacity":-3}"#).unwrap();
        assert_near(extgstate_alpha(&out, 0).0, 0.0);
    }

    #[test]
    fn watermark_angle_matrix_and_anchor_math() {
        // "WM" @ 50 pt Helvetica-Bold = 88.85 pt wide; page 300×400; half = 44.425.
        // Tm = [cos, sin, -sin, cos, w/2 - half*cos, h/2 - half*sin].
        let r = std::f32::consts::FRAC_1_SQRT_2; // cos 45° = sin 45°
        let cases: [(f32, [f32; 6]); 4] = [
            (0.0, [1.0, 0.0, 0.0, 1.0, 105.575, 200.0]),
            (45.0, [r, r, -r, r, 118.5868, 168.5868]),
            (90.0, [0.0, 1.0, -1.0, 0.0, 150.0, 155.575]),
            (-30.0, [0.86603, -0.5, 0.5, 0.86603, 111.5258, 222.2125]),
        ];
        for (angle, expected) in cases {
            let opts = format!(r#"{{"text":"WM","angle":{angle}}}"#);
            let out = add_watermark_native(&sample(1), &opts).unwrap();
            let ops = ops_of(&out, 0);
            let tm = last_op(&ops, "Tm");
            for (idx, &want) in expected.iter().enumerate() {
                assert_near(fnum(&tm.operands[idx]), want);
            }
        }
    }

    #[test]
    fn watermark_custom_color_and_clamping() {
        let out = add_watermark_native(
            &sample(1),
            r#"{"text":"X","color":{"r":0.2,"g":0.4,"b":0.8}}"#,
        )
        .unwrap();
        let ops = ops_of(&out, 0);
        let rg = last_op(&ops, "rg");
        assert_near(fnum(&rg.operands[0]), 0.2);
        assert_near(fnum(&rg.operands[1]), 0.4);
        assert_near(fnum(&rg.operands[2]), 0.8);
        // Out-of-range channels are clamped to [0, 1].
        let out =
            add_watermark_native(&sample(1), r#"{"text":"X","color":{"r":5,"g":-1,"b":0.5}}"#)
                .unwrap();
        let ops = ops_of(&out, 0);
        let rg = last_op(&ops, "rg");
        assert_near(fnum(&rg.operands[0]), 1.0);
        assert_near(fnum(&rg.operands[1]), 0.0);
        assert_near(fnum(&rg.operands[2]), 0.5);
    }

    #[test]
    fn watermark_custom_font_size_scales_anchor() {
        // "WM" @ 100 pt = 177.7 pt wide; angle 0 -> x = 150 - 88.85 = 61.15, y = 200.
        let out =
            add_watermark_native(&sample(1), r#"{"text":"WM","fontSize":100,"angle":0}"#).unwrap();
        let ops = ops_of(&out, 0);
        assert_near(fnum(&last_op(&ops, "Tf").operands[1]), 100.0);
        let tm = last_op(&ops, "Tm");
        assert_near(fnum(&tm.operands[4]), 61.15);
        assert_near(fnum(&tm.operands[5]), 200.0);
    }

    #[test]
    fn watermark_preserves_existing_content() {
        let out = add_watermark_native(&sample_with_text(2, Some("Body")), r#"{"text":"SECRET"}"#)
            .unwrap();
        for i in 0..2 {
            let text = page_content_text(&out, i);
            assert!(
                text.contains(&format!("Body {i}")),
                "original content lost: {text}"
            );
            assert!(text.contains("(SECRET)"));
            assert_eq!(count_op(&ops_of(&out, i), "Tj"), 2);
        }
    }

    #[test]
    fn watermark_invalid_json_rejected() {
        let err = add_watermark_native(&sample(1), "{").unwrap_err();
        assert!(err.starts_with("Invalid watermark options:"), "{err}");
    }
}
