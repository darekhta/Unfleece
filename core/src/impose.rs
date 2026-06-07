//! Imposition: N-up handout sheets and folded-booklet spreads.
//!
//! Both operations re-express every source page as a Form XObject *inside the
//! same document* (mirroring pdf-lib's `embedPdf`: BBox = normalized MediaBox,
//! Matrix translates the box origin to (0,0), content streams joined with
//! newlines, `/Rotate` ignored), then replace the page tree with freshly built
//! sheets whose content streams place those XObjects. Unreferenced source
//! objects are pruned on save, exactly like `select_pages_native`.
//!
//! Behavior parity targets `nUpPdf` / `bookletPdf` in `src/lib/tools/organize.ts`.

use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};
use serde::Deserialize;

use crate::util::{effective_media_box, inherited_page_value, save_compact};

/// A4 portrait `[width, height]` in PDF points (parity with `PAGE_SIZES.a4`).
const A4: (f32, f32) = (595.28, 841.89);
/// US Letter portrait in PDF points (parity with `PAGE_SIZES.letter`).
const LETTER: (f32, f32) = (612.0, 792.0);
/// Gap between and around N-up cells, in points (parity with organize.ts).
const N_UP_GAP: f32 = 8.0;

/// Reserved for future N-up options; parsing it surfaces malformed JSON early.
#[derive(Deserialize, Default)]
struct NUpOptions {}

/// Options for [`booklet_native`], mirroring `BookletOptions` in organize.ts.
#[derive(Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct BookletOptions {
    /// `"a4"` or `"letter"`; anything else falls back to A4.
    page_size: String,
    /// `"left"` (default) or `"right"`.
    binding: String,
    /// Sheet margin in points; negative values are clamped to 0.
    margin: f32,
}

impl Default for BookletOptions {
    fn default() -> Self {
        Self { page_size: "a4".to_string(), binding: "left".to_string(), margin: 18.0 }
    }
}

/// A source page converted to a Form XObject, with its visible size.
struct EmbeddedPage {
    id: ObjectId,
    width: f32,
    height: f32,
}

/// A target slot rectangle on an output sheet (PDF coords, origin bottom-left).
struct CellBox {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

fn parse_opts<T: serde::de::DeserializeOwned + Default>(json: &str) -> Result<T, String> {
    let trimmed = json.trim();
    if trimmed.is_empty() {
        return Ok(T::default());
    }
    serde_json::from_str(trimmed).map_err(|e| format!("Invalid options: {e}"))
}

/// Convert every page of `doc` into a Form XObject within the same document.
///
/// pdf-lib parity: BBox is the normalized (min/max) MediaBox, the Matrix
/// translates the box origin to (0,0), all content streams are decompressed
/// and joined with newlines, and Resources are carried over (own or inherited).
fn embed_pages(doc: &mut Document) -> Result<Vec<EmbeddedPage>, String> {
    let page_ids: Vec<ObjectId> = doc.page_iter().collect();
    let mut embedded = Vec::with_capacity(page_ids.len());

    for page_id in page_ids {
        let media = effective_media_box(doc, page_id).map_err(|e| e.to_string())?;
        let left = media[0].min(media[2]);
        let bottom = media[1].min(media[3]);
        let right = media[0].max(media[2]);
        let top = media[1].max(media[3]);

        let mut content = Vec::new();
        for stream_id in doc.get_page_contents(page_id) {
            if let Ok(stream) = doc.get_object(stream_id).and_then(Object::as_stream) {
                match stream.decompressed_content() {
                    Ok(data) => content.extend_from_slice(&data),
                    Err(_) => content.extend_from_slice(&stream.content),
                }
                content.push(b'\n');
            }
        }

        let resources = match doc.get_dictionary(page_id).map_err(|e| e.to_string())?.get(b"Resources") {
            Ok(value) => Some(value.clone()),
            Err(_) => inherited_page_value(doc, page_id, b"Resources").map_err(|e| e.to_string())?,
        };

        let mut dict = dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "FormType" => 1,
            "BBox" => vec![left.into(), bottom.into(), right.into(), top.into()],
            "Matrix" => vec![
                1f32.into(), 0f32.into(), 0f32.into(), 1f32.into(),
                (-left).into(), (-bottom).into(),
            ],
        };
        if let Some(resources) = resources {
            dict.set("Resources", resources);
        }

        let id = doc.add_object(Stream::new(dict, content));
        embedded.push(EmbeddedPage { id, width: right - left, height: top - bottom });
    }

    Ok(embedded)
}

/// Add one output sheet that draws each `(page, cell)` placement scaled to fit
/// its cell (aspect-preserving) and centered, like organize.ts `drawFittedPage`.
fn add_sheet(
    doc: &mut Document,
    parent: ObjectId,
    sheet_w: f32,
    sheet_h: f32,
    placements: &[(&EmbeddedPage, CellBox)],
) -> Result<ObjectId, String> {
    let mut operations = Vec::new();
    let mut xobjects = Dictionary::new();

    for (slot, (page, cell)) in placements.iter().enumerate() {
        let scale = (cell.w / page.width).min(cell.h / page.height);
        if !scale.is_finite() || scale <= 0.0 {
            continue; // degenerate (zero-size) source page: leave the cell blank
        }
        let w = page.width * scale;
        let h = page.height * scale;
        let x = cell.x + (cell.w - w) / 2.0;
        let y = cell.y + (cell.h - h) / 2.0;

        let name = format!("P{slot}");
        xobjects.set(name.clone().into_bytes(), Object::Reference(page.id));
        operations.push(Operation::new("q", vec![]));
        operations.push(Operation::new(
            "cm",
            vec![scale.into(), 0f32.into(), 0f32.into(), scale.into(), x.into(), y.into()],
        ));
        operations.push(Operation::new("Do", vec![Object::Name(name.into_bytes())]));
        operations.push(Operation::new("Q", vec![]));
    }

    let content = Content { operations };
    let content_id = doc.add_object(Stream::new(
        dictionary! {},
        content.encode().map_err(|e| e.to_string())?,
    ));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => parent,
        "MediaBox" => vec![0f32.into(), 0f32.into(), sheet_w.into(), sheet_h.into()],
        "Resources" => dictionary! { "XObject" => xobjects },
        "Contents" => content_id,
    });
    Ok(page_id)
}

/// Point the catalog at a fresh /Pages node containing only the new sheets.
///
/// A fresh node guarantees no inheritable attributes (Rotate, CropBox, …) leak
/// from the source page tree onto the sheets. Document-level structures that
/// can reference the removed source pages are dropped, matching pdf-lib's
/// "build a brand-new document" behavior (and `select_pages_native`).
fn install_sheets(doc: &mut Document, pages_id: ObjectId, sheet_ids: &[ObjectId]) -> Result<(), String> {
    let kids: Vec<Object> = sheet_ids.iter().map(|&id| Object::Reference(id)).collect();
    let count = kids.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }),
    );

    let catalog = doc.catalog_mut().map_err(|e| e.to_string())?;
    catalog.set("Pages", Object::Reference(pages_id));
    for key in [
        b"Outlines".as_slice(),
        b"PageLabels".as_slice(),
        b"OpenAction".as_slice(),
        b"AA".as_slice(),
        b"Names".as_slice(),
        b"Dests".as_slice(),
        b"AcroForm".as_slice(),
    ] {
        catalog.remove(key);
    }
    Ok(())
}

/// Place `per_sheet` source pages onto each A4 portrait sheet (handout-style
/// N-up). `per_sheet` must be one of 2, 4, 6, 8, 9 or 16 (the UI offers
/// 2/4/6/9; 8 and 16 are accepted for parity with organize.ts's grid table).
///
/// `opts_json` is reserved for future options; pass `""` or `"{}"`.
pub fn n_up_native(data: &[u8], per_sheet: u32, opts_json: &str) -> Result<Vec<u8>, String> {
    // (cols, rows) — parity with organize.ts `const [cols, rows] = GRID[perSheet]`.
    let (cols, rows) = match per_sheet {
        2 => (1usize, 2usize),
        4 => (2, 2),
        6 => (2, 3),
        8 => (2, 4),
        9 => (3, 3),
        16 => (4, 4),
        other => return Err(format!("Unsupported pages-per-sheet: {other}")),
    };
    let _opts: NUpOptions = parse_opts(opts_json)?;

    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    let embedded = embed_pages(&mut doc)?;

    let (sheet_w, sheet_h) = A4;
    let cell_w = (sheet_w - N_UP_GAP * (cols as f32 + 1.0)) / cols as f32;
    let cell_h = (sheet_h - N_UP_GAP * (rows as f32 + 1.0)) / rows as f32;

    let pages_id = doc.new_object_id();
    let mut sheets = Vec::new();
    let per = per_sheet as usize;
    let mut i = 0;
    while i < embedded.len() {
        let mut placements = Vec::new();
        for j in 0..per {
            let Some(page) = embedded.get(i + j) else { break };
            let col = (j % cols) as f32;
            let row = (j / cols) as f32;
            // Rows fill top-to-bottom; PDF origin is bottom-left (parity with TS).
            placements.push((
                page,
                CellBox {
                    x: N_UP_GAP + col * (cell_w + N_UP_GAP),
                    y: sheet_h - N_UP_GAP - (row + 1.0) * cell_h - row * N_UP_GAP,
                    w: cell_w,
                    h: cell_h,
                },
            ));
        }
        sheets.push(add_sheet(&mut doc, pages_id, sheet_w, sheet_h, &placements)?);
        i += per;
    }

    install_sheets(&mut doc, pages_id, &sheets)?;
    save_compact(doc).map_err(|e| e.to_string())
}

/// The classic saddle-stitch imposition order: each physical sheet holds four
/// page instances; spreads are built outside-in so the folded stack reads in
/// order. Returns `[left-slot, right-slot]` source indices per output sheet.
fn left_bound_spread_order(padded_page_count: usize) -> Vec<[usize; 2]> {
    let mut spreads = Vec::new();
    for sheet in 0..padded_page_count / 4 {
        spreads.push([padded_page_count - 1 - sheet * 2, sheet * 2]);
        spreads.push([sheet * 2 + 1, padded_page_count - 2 - sheet * 2]);
    }
    spreads
}

/// Reorder pages into two-up booklet spreads on landscape sheets.
///
/// `opts_json` mirrors organize.ts `BookletOptions`:
/// `{"pageSize":"a4"|"letter","binding":"left"|"right","margin":18}` — all
/// fields optional; unknown page sizes fall back to A4, negative margins clamp
/// to 0. The page count is padded up to a multiple of 4 with blank slots.
pub fn booklet_native(data: &[u8], opts_json: &str) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    if doc.page_iter().next().is_none() {
        return Err("No pages found".to_string());
    }
    let opts: BookletOptions = parse_opts(opts_json)?;

    let (portrait_w, portrait_h) = match opts.page_size.as_str() {
        "letter" => LETTER,
        _ => A4,
    };
    let (sheet_w, sheet_h) = (portrait_h, portrait_w); // landscape
    let margin = opts.margin.max(0.0);
    let printable_w = (sheet_w - margin * 2.0).max(1.0);
    let printable_h = (sheet_h - margin * 2.0).max(1.0);
    let cell_w = printable_w / 2.0;
    let cell_h = printable_h;

    let embedded = embed_pages(&mut doc)?;
    let padded_page_count = embedded.len().div_ceil(4) * 4;
    let spreads = left_bound_spread_order(padded_page_count);
    let right_bound = opts.binding == "right";

    let pages_id = doc.new_object_id();
    let mut sheets = Vec::new();
    for spread in spreads {
        let ordered = if right_bound { [spread[1], spread[0]] } else { spread };
        let mut placements = Vec::new();
        for (slot, &page_index) in ordered.iter().enumerate() {
            let Some(page) = embedded.get(page_index) else { continue }; // padding: blank slot
            placements.push((
                page,
                CellBox { x: margin + slot as f32 * cell_w, y: margin, w: cell_w, h: cell_h },
            ));
        }
        sheets.push(add_sheet(&mut doc, pages_id, sheet_w, sheet_h, &placements)?);
    }

    install_sheets(&mut doc, pages_id, &sheets)?;
    save_compact(doc).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fixtures::{sample, sample_with_text};
    use crate::util::number_as_f32;
    use lopdf::content::Content;
    use lopdf::{Document, Object};

    const A4_W: f32 = 595.28;
    const A4_H: f32 = 841.89;
    const EPS: f32 = 1e-3;

    fn load(data: &[u8]) -> Document {
        Document::load_mem(data).unwrap()
    }

    fn page_count(data: &[u8]) -> usize {
        load(data).get_pages().len()
    }

    fn page_size(data: &[u8], n: usize) -> (f32, f32) {
        let doc = load(data);
        let pages: Vec<_> = doc.page_iter().collect();
        let mb = crate::util::effective_media_box(&doc, pages[n]).unwrap();
        (mb[2] - mb[0], mb[3] - mb[1])
    }

    /// Decoded (operator, numeric operands) list for output page `n`.
    fn ops(data: &[u8], n: usize) -> Vec<(String, Vec<f32>)> {
        let doc = load(data);
        let pages: Vec<_> = doc.page_iter().collect();
        let content = doc.get_page_content(pages[n]).unwrap();
        Content::decode(&content)
            .unwrap()
            .operations
            .into_iter()
            .map(|op| {
                let nums = op.operands.iter().filter_map(number_as_f32).collect();
                (op.operator, nums)
            })
            .collect()
    }

    fn do_count(data: &[u8], n: usize) -> usize {
        ops(data, n).iter().filter(|(op, _)| op == "Do").count()
    }

    fn cm_ops(data: &[u8], n: usize) -> Vec<Vec<f32>> {
        ops(data, n)
            .into_iter()
            .filter(|(op, _)| op == "cm")
            .map(|(_, nums)| nums)
            .collect()
    }

    /// The Form XObject behind slot `name` on output page `n`.
    fn slot_form(data: &[u8], n: usize, name: &str) -> lopdf::Stream {
        let doc = load(data);
        let pages: Vec<_> = doc.page_iter().collect();
        let resources = doc
            .get_dictionary(pages[n])
            .unwrap()
            .get(b"Resources")
            .unwrap()
            .as_dict()
            .unwrap()
            .clone();
        let id = resources
            .get(b"XObject")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(name.as_bytes())
            .unwrap()
            .as_reference()
            .unwrap();
        doc.get_object(id).unwrap().as_stream().unwrap().clone()
    }

    fn form_numbers(form: &lopdf::Stream, key: &[u8]) -> Vec<f32> {
        form.dict
            .get(key)
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|o| number_as_f32(o).unwrap())
            .collect()
    }

    fn assert_close(actual: f32, expected: f32, what: &str) {
        assert!(
            (actual - expected).abs() < EPS,
            "{what}: expected {expected}, got {actual}"
        );
    }

    /// sample(n) page i is 300+i wide, 400 tall, with MediaBox origin (0,0).
    fn fixture_size(i: usize) -> (f32, f32) {
        (300.0 + i as f32, 400.0)
    }

    /// Expected fitted placement of a page inside a cell (mirrors the TS math).
    fn fitted(pw: f32, ph: f32, cell: &CellBox) -> (f32, f32, f32) {
        let scale = (cell.w / pw).min(cell.h / ph);
        let w = pw * scale;
        let h = ph * scale;
        (scale, cell.x + (cell.w - w) / 2.0, cell.y + (cell.h - h) / 2.0)
    }

    // ------------------------------------------------------------------ n-up

    #[test]
    fn n_up_4_makes_two_a4_sheets_from_8_pages() {
        let out = n_up_native(&sample(8), 4, "{}").unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count(&out), 2);
        let (w, h) = page_size(&out, 0);
        assert_close(w, A4_W, "sheet width");
        assert_close(h, A4_H, "sheet height");
        assert_eq!(do_count(&out, 0), 4);
        assert_eq!(do_count(&out, 1), 4);
    }

    #[test]
    fn n_up_2_stacks_pages_vertically_on_portrait_a4() {
        // GRID[2] = [1, 2] -> 1 column, 2 rows.
        let out = n_up_native(&sample(4), 2, "{}").unwrap();
        assert_eq!(page_count(&out), 2);

        let cell_w = A4_W - N_UP_GAP * 2.0;
        let cell_h = (A4_H - N_UP_GAP * 3.0) / 2.0;
        let cms = cm_ops(&out, 0);
        assert_eq!(cms.len(), 2);

        // Page 0 (300x400) in the top cell.
        let top_cell = CellBox { x: N_UP_GAP, y: A4_H - N_UP_GAP - cell_h, w: cell_w, h: cell_h };
        let (s0, x0, y0) = fitted(300.0, 400.0, &top_cell);
        assert_close(cms[0][0], s0, "top scale");
        assert_close(cms[0][4], x0, "top x");
        assert_close(cms[0][5], y0, "top y");

        // Page 1 (301x400) in the bottom cell — strictly below the top one.
        let bottom_cell =
            CellBox { x: N_UP_GAP, y: A4_H - N_UP_GAP - 2.0 * cell_h - N_UP_GAP, w: cell_w, h: cell_h };
        let (s1, x1, y1) = fitted(301.0, 400.0, &bottom_cell);
        assert_close(cms[1][0], s1, "bottom scale");
        assert_close(cms[1][4], x1, "bottom x");
        assert_close(cms[1][5], y1, "bottom y");
        assert!(cms[1][5] < cms[0][5], "row 1 must sit below row 0");
    }

    #[test]
    fn n_up_9_fills_a_single_3x3_sheet() {
        let out = n_up_native(&sample(9), 9, "{}").unwrap();
        assert_eq!(page_count(&out), 1);
        assert_eq!(do_count(&out, 0), 9);
        // Every placement j lands fitted+centered in grid cell (j%3, j/3).
        let cell_w = (A4_W - N_UP_GAP * 4.0) / 3.0;
        let cell_h = (A4_H - N_UP_GAP * 4.0) / 3.0;
        let cms = cm_ops(&out, 0);
        for (j, cm) in cms.iter().enumerate() {
            let col = (j % 3) as f32;
            let row = (j / 3) as f32;
            let cell = CellBox {
                x: N_UP_GAP + col * (cell_w + N_UP_GAP),
                y: A4_H - N_UP_GAP - (row + 1.0) * cell_h - row * N_UP_GAP,
                w: cell_w,
                h: cell_h,
            };
            let (pw, ph) = fixture_size(j);
            let (s, x, y) = fitted(pw, ph, &cell);
            assert_close(cm[0], s, &format!("cell {j} scale"));
            assert_close(cm[4], x, &format!("cell {j} x"));
            assert_close(cm[5], y, &format!("cell {j} y"));
        }
    }

    #[test]
    fn n_up_rejects_unsupported_per_sheet_values() {
        for bad in [0u32, 1, 3, 5, 7, 10, 12] {
            let err = n_up_native(&sample(2), bad, "{}").unwrap_err();
            assert_eq!(err, format!("Unsupported pages-per-sheet: {bad}"));
        }
    }

    #[test]
    fn n_up_partial_last_sheet_leaves_blank_cells() {
        let out = n_up_native(&sample(7), 4, "{}").unwrap();
        assert_eq!(page_count(&out), 2);
        assert_eq!(do_count(&out, 0), 4);
        assert_eq!(do_count(&out, 1), 3); // 4th cell blank, no Do op
    }

    #[test]
    fn n_up_empty_pdf_yields_valid_zero_sheet_pdf() {
        let out = n_up_native(&sample(0), 4, "{}").unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count(&out), 0);
    }

    #[test]
    fn n_up_single_page_fits_centered_in_top_left_cell() {
        let out = n_up_native(&sample(1), 4, "{}").unwrap();
        assert_eq!(page_count(&out), 1);
        assert_eq!(do_count(&out, 0), 1);

        let cms = cm_ops(&out, 0);
        let cm = &cms[0];
        // Aspect preserved: cm = [s 0 0 s x y] with uniform scale and no skew.
        assert_close(cm[0], cm[3], "sx == sy");
        assert_close(cm[1], 0.0, "no skew b");
        assert_close(cm[2], 0.0, "no skew c");

        let cell_w = (A4_W - N_UP_GAP * 3.0) / 2.0;
        let cell_h = (A4_H - N_UP_GAP * 3.0) / 2.0;
        let cell = CellBox { x: N_UP_GAP, y: A4_H - N_UP_GAP - cell_h, w: cell_w, h: cell_h };
        let (s, x, y) = fitted(300.0, 400.0, &cell);
        assert_close(cm[0], s, "scale");
        assert_close(cm[4], x, "x");
        assert_close(cm[5], y, "y");
        // The 300x400 page is width-limited: it fills the cell width exactly,
        // so x sits flush at the gap while the height is centered.
        assert_close(cm[4], N_UP_GAP, "x flush with cell");
        let h = 400.0 * s;
        assert_close(cm[5], cell.y + (cell_h - h) / 2.0, "y centered");
        // Top-left cell: left of horizontal center, above vertical center.
        assert!(cm[4] < A4_W / 2.0);
        assert!(cm[5] > A4_H / 2.0);
    }

    #[test]
    fn n_up_option_json_handling() {
        // Empty string and empty object are fine; unknown fields are ignored.
        assert!(n_up_native(&sample(1), 2, "").is_ok());
        assert!(n_up_native(&sample(1), 2, "{}").is_ok());
        assert!(n_up_native(&sample(1), 2, r#"{"future":true}"#).is_ok());
        // Malformed JSON errors cleanly (no panic).
        let err = n_up_native(&sample(1), 2, "{not json").unwrap_err();
        assert!(err.starts_with("Invalid options:"), "got: {err}");
    }

    #[test]
    fn n_up_6_distributes_ten_pages_over_two_sheets() {
        let out = n_up_native(&sample(10), 6, "{}").unwrap();
        assert_eq!(page_count(&out), 2);
        assert_eq!(do_count(&out, 0), 6);
        assert_eq!(do_count(&out, 1), 4);
    }

    #[test]
    fn n_up_supports_grid_8_and_16_for_ts_parity() {
        let out = n_up_native(&sample(8), 8, "{}").unwrap();
        assert_eq!(page_count(&out), 1);
        assert_eq!(do_count(&out, 0), 8);

        let out = n_up_native(&sample(16), 16, "{}").unwrap();
        assert_eq!(page_count(&out), 1);
        assert_eq!(do_count(&out, 0), 16);
    }

    #[test]
    fn n_up_carries_page_resources_and_content_into_forms() {
        let out = n_up_native(&sample_with_text(2, Some("Hello")), 2, "{}").unwrap();
        for (i, name) in ["P0", "P1"].iter().enumerate() {
            let form = slot_form(&out, 0, name);
            assert_eq!(form.dict.get(b"Subtype").unwrap().as_name().unwrap(), b"Form");
            assert!(form.dict.has(b"Resources"), "form must carry page Resources");
            // Tiny streams may legitimately stay uncompressed (lopdf only adds
            // a Filter when flate actually shrinks them).
            let body = form.decompressed_content().unwrap_or_else(|_| form.content.clone());
            let text = String::from_utf8_lossy(&body);
            assert!(text.contains("Tj"), "form content must keep text ops: {text}");
            assert!(text.contains(&format!("Hello {i}")), "form {name} draws its page text");
        }
    }

    /// Rewrite page 0's MediaBox of a 1-page sample and return the PDF bytes.
    fn with_media_box(media_box: [f32; 4]) -> Vec<u8> {
        let mut doc = Document::load_mem(&sample(1)).unwrap();
        let page = doc.page_iter().next().unwrap();
        let array: Vec<Object> = media_box.iter().map(|&v| Object::Real(v)).collect();
        doc.get_object_mut(page).unwrap().as_dict_mut().unwrap().set("MediaBox", array);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    #[test]
    fn n_up_normalizes_bbox_and_translates_origin_like_pdf_lib() {
        // Reversed corners normalize to [100 50 400 450]; Matrix moves origin to (0,0).
        for mb in [[100.0, 50.0, 400.0, 450.0], [400.0, 450.0, 100.0, 50.0]] {
            let out = n_up_native(&with_media_box(mb), 2, "{}").unwrap();
            let form = slot_form(&out, 0, "P0");
            assert_eq!(form_numbers(&form, b"BBox"), vec![100.0, 50.0, 400.0, 450.0]);
            assert_eq!(form_numbers(&form, b"Matrix"), vec![1.0, 0.0, 0.0, 1.0, -100.0, -50.0]);
            // Placement math sees a 300x400 page, same as the zero-origin fixture.
            let zero = n_up_native(&sample(1), 2, "{}").unwrap();
            assert_eq!(cm_ops(&out, 0), cm_ops(&zero, 0));
        }
    }

    #[test]
    fn n_up_skips_degenerate_zero_size_pages() {
        let out = n_up_native(&with_media_box([0.0, 0.0, 0.0, 0.0]), 2, "{}").unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count(&out), 1);
        assert_eq!(do_count(&out, 0), 0); // blank cell instead of NaN coordinates
        let text: String = ops(&out, 0).iter().map(|(op, _)| op.clone()).collect();
        assert!(!text.to_lowercase().contains("nan"));
    }

    // --------------------------------------------------------------- booklet

    #[test]
    fn booklet_spread_order_matches_ts_formula() {
        assert_eq!(left_bound_spread_order(4), vec![[3, 0], [1, 2]]);
        assert_eq!(left_bound_spread_order(8), vec![[7, 0], [1, 6], [5, 2], [3, 4]]);
    }

    #[test]
    fn booklet_four_pages_makes_two_landscape_spreads() {
        let out = booklet_native(&sample(4), "{}").unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count(&out), 2);
        let (w, h) = page_size(&out, 0);
        assert_close(w, A4_H, "landscape width");
        assert_close(h, A4_W, "landscape height");
        assert!(w > h);

        // Spread 0 = [3, 0]: page 3 (width 303) left, page 0 (width 300) right.
        let widths = |n: usize, name: &str| {
            let bb = form_numbers(&slot_form(&out, n, name), b"BBox");
            bb[2] - bb[0]
        };
        assert_close(widths(0, "P0"), 303.0, "spread 0 left");
        assert_close(widths(0, "P1"), 300.0, "spread 0 right");
        // Spread 1 = [1, 2].
        assert_close(widths(1, "P0"), 301.0, "spread 1 left");
        assert_close(widths(1, "P1"), 302.0, "spread 1 right");
    }

    #[test]
    fn booklet_five_pages_pads_to_eight_with_blank_slots() {
        let out = booklet_native(&sample(5), "{}").unwrap();
        // padded = 8 -> spreads [7,0] [1,6] [5,2] [3,4]; indices >= 5 are blanks.
        assert_eq!(page_count(&out), 4);
        assert_eq!(do_count(&out, 0), 1);
        assert_eq!(do_count(&out, 1), 1);
        assert_eq!(do_count(&out, 2), 1);
        assert_eq!(do_count(&out, 3), 2);
    }

    #[test]
    fn booklet_right_binding_swaps_slots() {
        let out = booklet_native(&sample(4), r#"{"binding":"right"}"#).unwrap();
        // Spread 0 ordered = [0, 3]: page 0 (300) left, page 3 (303) right.
        let widths = |name: &str| {
            let bb = form_numbers(&slot_form(&out, 0, name), b"BBox");
            bb[2] - bb[0]
        };
        assert_close(widths("P0"), 300.0, "right-bound left slot");
        assert_close(widths("P1"), 303.0, "right-bound right slot");
        // Right slot box starts at margin + cellW.
        let cell_w = (A4_H - 18.0 * 2.0) / 2.0;
        let cms = cm_ops(&out, 0);
        assert!(cms[1][4] >= 18.0 + cell_w - EPS, "second placement in right half");
    }

    #[test]
    fn booklet_letter_sheet_is_landscape_letter() {
        let out = booklet_native(&sample(4), r#"{"pageSize":"letter"}"#).unwrap();
        let (w, h) = page_size(&out, 0);
        assert_close(w, 792.0, "letter landscape width");
        assert_close(h, 612.0, "letter landscape height");

        // Combined with right binding: still letter-sized, slots swapped.
        let out = booklet_native(&sample(4), r#"{"pageSize":"letter","binding":"right"}"#).unwrap();
        let (w, h) = page_size(&out, 0);
        assert_close(w, 792.0, "letter+right width");
        assert_close(h, 612.0, "letter+right height");
        let bb = form_numbers(&slot_form(&out, 0, "P0"), b"BBox");
        assert_close(bb[2] - bb[0], 300.0, "letter+right left slot is page 0");
    }

    #[test]
    fn booklet_margin_moves_and_shrinks_cells() {
        let out = booklet_native(&sample(4), r#"{"margin":30}"#).unwrap();
        let margin = 30.0;
        let cell_w = (A4_H - margin * 2.0) / 2.0;
        let cell_h = A4_W - margin * 2.0;

        // Spread 0 left slot holds page 3 (303x400): width-limited fit.
        let cell = CellBox { x: margin, y: margin, w: cell_w, h: cell_h };
        let (s, x, y) = fitted(303.0, 400.0, &cell);
        let cms = cm_ops(&out, 0);
        assert_close(cms[0][0], s, "scale");
        assert_close(cms[0][4], x, "x = margin (width fills cell)");
        assert_close(cms[0][5], y, "y centered in printable height");
        assert_close(x, margin, "width-limited page sits flush at margin");
    }

    #[test]
    fn booklet_zero_margin_uses_full_sheet() {
        let out = booklet_native(&sample(4), r#"{"margin":0}"#).unwrap();
        let cell_w = A4_H / 2.0;
        let cms = cm_ops(&out, 0);
        // Left slot (page 3) flush at x = 0; right slot (page 0) at x = cellW.
        assert_close(cms[0][4], 0.0, "left slot x");
        assert_close(cms[1][4], cell_w, "right slot x");
    }

    #[test]
    fn booklet_negative_margin_clamps_to_zero() {
        let neg = booklet_native(&sample(4), r#"{"margin":-50}"#).unwrap();
        let zero = booklet_native(&sample(4), r#"{"margin":0}"#).unwrap();
        assert_eq!(ops(&neg, 0), ops(&zero, 0));
        assert_eq!(ops(&neg, 1), ops(&zero, 1));
    }

    #[test]
    fn booklet_empty_pdf_is_rejected() {
        let err = booklet_native(&sample(0), "{}").unwrap_err();
        assert_eq!(err, "No pages found");
    }

    #[test]
    fn booklet_option_parsing_and_fallbacks() {
        // Empty JSON -> defaults (a4, left, margin 18): left page fills its cell
        // width so its x is exactly the default margin.
        let out = booklet_native(&sample(4), "").unwrap();
        let (w, h) = page_size(&out, 0);
        assert_close(w, A4_H, "default a4 width");
        assert_close(h, A4_W, "default a4 height");
        assert_close(cm_ops(&out, 0)[0][4], 18.0, "default margin 18");

        // Unknown page size falls back to a4; unknown binding falls back to left.
        let out = booklet_native(&sample(4), r#"{"pageSize":"tabloid","binding":"middle"}"#).unwrap();
        let (w, _) = page_size(&out, 0);
        assert_close(w, A4_H, "fallback a4 width");
        let bb = form_numbers(&slot_form(&out, 0, "P0"), b"BBox");
        assert_close(bb[2] - bb[0], 303.0, "left binding: page 3 in left slot");

        // Malformed JSON errors cleanly.
        let err = booklet_native(&sample(4), "{oops").unwrap_err();
        assert!(err.starts_with("Invalid options:"), "got: {err}");
    }

    #[test]
    fn booklet_single_page_pads_to_one_folded_sheet() {
        let out = booklet_native(&sample(1), "{}").unwrap();
        // padded = 4 -> spreads [3,0] and [1,2]; only page 0 exists.
        assert_eq!(page_count(&out), 2);
        assert_eq!(do_count(&out, 0), 1);
        assert_eq!(do_count(&out, 1), 0);
        // The lone page sits in the right slot of spread 0.
        let cell_w = (A4_H - 36.0) / 2.0;
        assert!(cm_ops(&out, 0)[0][4] >= 18.0 + cell_w - EPS);
    }

    #[test]
    fn booklet_output_reparses_and_keeps_text_resources() {
        let out = booklet_native(&sample_with_text(4, Some("Spread")), "{}").unwrap();
        assert_eq!(page_count(&out), 2);
        let form = slot_form(&out, 0, "P1"); // page 0
        assert!(form.dict.has(b"Resources"));
        let body = form.decompressed_content().unwrap_or_else(|_| form.content.clone());
        assert!(String::from_utf8_lossy(&body).contains("Spread 0"));
    }
}
