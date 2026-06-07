//! Page-box operations: margin cropping via the page CropBox.
//!
//! Port of `src/lib/tools/crop.ts` (pdf-lib `cropPdf`). The crop never touches
//! the MediaBox — it only sets each page's CropBox (the visible "window")
//! relative to that page's effective MediaBox, so all page content survives
//! and un-cropping remains possible in other tools.

use lopdf::{Document, Object};

use crate::util::{effective_media_box, save_compact};

/// Trim `top`/`right`/`bottom`/`left` points from every page by setting the
/// page CropBox relative to its effective (own or inherited) MediaBox.
///
/// Mirrors the TS `cropPdf` semantics exactly:
/// - every margin must be `>= 0`, otherwise `"Crop margins cannot be negative"`;
/// - per page, the remaining width/height must be `> 0`, otherwise
///   `"Crop margins exceed page size"`;
/// - the new CropBox is `[x0 + left, y0 + bottom, x1 - right, y1 - top]`
///   (PDF y-axis points up, so the *bottom* margin moves the y-origin);
/// - the MediaBox is left untouched.
pub fn crop_margins_native(data: &[u8], top: f32, right: f32, bottom: f32, left: f32) -> Result<Vec<u8>, String> {
    if top < 0.0 || right < 0.0 || bottom < 0.0 || left < 0.0 {
        return Err("Crop margins cannot be negative".to_string());
    }
    if !(top.is_finite() && right.is_finite() && bottom.is_finite() && left.is_finite()) {
        return Err("Crop margins must be finite numbers".to_string());
    }

    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    let pages: Vec<_> = doc.page_iter().collect();

    for page_id in pages {
        let [x0, y0, x1, y1] = effective_media_box(&doc, page_id).map_err(|e| e.to_string())?;
        let new_width = (x1 - x0) - left - right;
        let new_height = (y1 - y0) - top - bottom;
        if new_width <= 0.0 || new_height <= 0.0 {
            return Err("Crop margins exceed page size".to_string());
        }

        let crop_box = vec![
            Object::Real(x0 + left),
            Object::Real(y0 + bottom),
            Object::Real(x0 + left + new_width),
            Object::Real(y0 + bottom + new_height),
        ];
        doc.get_object_mut(page_id)
            .and_then(|obj| obj.as_dict_mut())
            .map_err(|e| e.to_string())?
            .set("CropBox", crop_box);
    }

    save_compact(doc).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fixtures::{page_content_text, page_width, sample, sample_with_text};
    use crate::util::number_as_f32;
    use lopdf::content::Content;
    use lopdf::{dictionary, Document, Object, Stream};

    /// CropBox of page `n` (0-based) as `[x0, y0, x1, y1]`.
    fn crop_box(data: &[u8], n: usize) -> [f32; 4] {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let arr = doc
            .get_dictionary(pages[n])
            .unwrap()
            .get(b"CropBox")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let mut out = [0f32; 4];
        for (i, obj) in arr.iter().enumerate() {
            out[i] = number_as_f32(obj).unwrap();
        }
        out
    }

    fn media_box(data: &[u8], n: usize) -> [f32; 4] {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        crate::util::effective_media_box(&doc, pages[n]).unwrap()
    }

    fn assert_box_eq(actual: [f32; 4], expected: [f32; 4]) {
        for (a, e) in actual.iter().zip(expected.iter()) {
            assert!((a - e).abs() < 1e-3, "box {actual:?} != expected {expected:?}");
        }
    }

    /// One-page PDF with the given MediaBox and (optionally) a pre-existing CropBox.
    fn sample_with_boxes(media: [f32; 4], crop: Option<[f32; 4]>) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            Content { operations: vec![] }.encode().unwrap(),
        ));
        let mut page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => media.iter().map(|&v| Object::Real(v)).collect::<Vec<_>>(),
        };
        if let Some(c) = crop {
            page.set("CropBox", c.iter().map(|&v| Object::Real(v)).collect::<Vec<_>>());
        }
        let page_id = doc.add_object(page);
        let pages_dict = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    /// One-page PDF whose MediaBox lives on the /Pages node (inherited).
    fn sample_inherited_media_box() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            Content { operations: vec![] }.encode().unwrap(),
        ));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        let pages_dict = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    // -- happy paths --------------------------------------------------------

    #[test]
    fn crops_single_left_margin() {
        // sample page 0 is 300x400 with origin (0, 0)
        let out = crop_margins_native(&sample(1), 0.0, 0.0, 0.0, 10.0).unwrap();
        assert_box_eq(crop_box(&out, 0), [10.0, 0.0, 300.0, 400.0]);
    }

    #[test]
    fn crops_all_four_margins() {
        // mirrors the TS test: 300x400 with left/right 20 and top/bottom 30 -> 260x340
        let out = crop_margins_native(&sample(1), 30.0, 20.0, 30.0, 20.0).unwrap();
        let cb = crop_box(&out, 0);
        assert_box_eq(cb, [20.0, 30.0, 280.0, 370.0]);
        assert!((cb[2] - cb[0] - 260.0).abs() < 1e-3);
        assert!((cb[3] - cb[1] - 340.0).abs() < 1e-3);
    }

    #[test]
    fn zero_margins_set_crop_box_equal_to_media_box() {
        let out = crop_margins_native(&sample(1), 0.0, 0.0, 0.0, 0.0).unwrap();
        assert_box_eq(crop_box(&out, 0), [0.0, 0.0, 300.0, 400.0]);
    }

    #[test]
    fn media_box_is_not_modified() {
        let out = crop_margins_native(&sample(1), 30.0, 20.0, 30.0, 20.0).unwrap();
        assert_box_eq(media_box(&out, 0), [0.0, 0.0, 300.0, 400.0]);
        assert_eq!(page_width(&out, 0), 300.0);
    }

    #[test]
    fn crops_every_page_independently() {
        // sample pages are 300/301/302 wide
        let out = crop_margins_native(&sample(3), 0.0, 50.0, 0.0, 50.0).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        assert_eq!(doc.get_pages().len(), 3);
        for (i, expected_width) in [200.0, 201.0, 202.0].iter().enumerate() {
            let cb = crop_box(&out, i);
            assert!((cb[2] - cb[0] - expected_width).abs() < 1e-3, "page {i}");
            assert_eq!(page_width(&out, i), 300.0 + i as f32); // MediaBox untouched
        }
    }

    #[test]
    fn preserves_page_content() {
        let out = crop_margins_native(&sample_with_text(2, Some("Hello")), 10.0, 10.0, 10.0, 10.0).unwrap();
        assert!(page_content_text(&out, 0).contains("Hello 0"));
        assert!(page_content_text(&out, 1).contains("Hello 1"));
    }

    #[test]
    fn shifts_origin_for_non_zero_media_box() {
        let pdf = sample_with_boxes([10.0, 20.0, 300.0, 420.0], None);
        let out = crop_margins_native(&pdf, 3.0, 4.0, 7.0, 5.0).unwrap();
        // [x0 + left, y0 + bottom, x1 - right, y1 - top]
        assert_box_eq(crop_box(&out, 0), [15.0, 27.0, 296.0, 417.0]);
    }

    #[test]
    fn uses_inherited_media_box() {
        let out = crop_margins_native(&sample_inherited_media_box(), 5.0, 5.0, 5.0, 5.0).unwrap();
        assert_box_eq(crop_box(&out, 0), [5.0, 5.0, 195.0, 95.0]);
    }

    #[test]
    fn overwrites_existing_crop_box_from_media_box() {
        let pdf = sample_with_boxes([0.0, 0.0, 300.0, 400.0], Some([5.0, 5.0, 100.0, 100.0]));
        let out = crop_margins_native(&pdf, 0.0, 0.0, 0.0, 10.0).unwrap();
        // computed from the MediaBox, not the stale CropBox
        assert_box_eq(crop_box(&out, 0), [10.0, 0.0, 300.0, 400.0]);
    }

    #[test]
    fn handles_fractional_margins() {
        let out = crop_margins_native(&sample(1), 0.1, 0.1, 0.1, 0.1).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_box_eq(crop_box(&out, 0), [0.1, 0.1, 299.9, 399.9]);
    }

    #[test]
    fn handles_large_pages() {
        let pdf = sample_with_boxes([0.0, 0.0, 1000.0, 1000.0], None);
        let out = crop_margins_native(&pdf, 10.0, 10.0, 10.0, 10.0).unwrap();
        assert_box_eq(crop_box(&out, 0), [10.0, 10.0, 990.0, 990.0]);
    }

    #[test]
    fn empty_document_is_passthrough() {
        let out = crop_margins_native(&sample(0), 50.0, 50.0, 50.0, 50.0).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(Document::load_mem(&out).unwrap().get_pages().len(), 0);
    }

    #[test]
    fn output_is_valid_pdf_with_same_page_count() {
        let out = crop_margins_native(&sample(5), 1.0, 2.0, 3.0, 4.0).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(Document::load_mem(&out).unwrap().get_pages().len(), 5);
    }

    // -- validation ---------------------------------------------------------

    #[test]
    fn rejects_negative_top() {
        let err = crop_margins_native(&sample(1), -5.0, 0.0, 0.0, 0.0).unwrap_err();
        assert_eq!(err, "Crop margins cannot be negative");
    }

    #[test]
    fn rejects_negative_right() {
        let err = crop_margins_native(&sample(1), 0.0, -1.0, 0.0, 0.0).unwrap_err();
        assert_eq!(err, "Crop margins cannot be negative");
    }

    #[test]
    fn rejects_negative_bottom() {
        let err = crop_margins_native(&sample(1), 0.0, 0.0, -0.1, 0.0).unwrap_err();
        assert_eq!(err, "Crop margins cannot be negative");
    }

    #[test]
    fn rejects_negative_left() {
        let err = crop_margins_native(&sample(1), 0.0, 0.0, 0.0, -999.0).unwrap_err();
        assert_eq!(err, "Crop margins cannot be negative");
    }

    #[test]
    fn validates_margins_before_parsing_the_pdf() {
        // TS parity: the negative-margin check fires before the PDF is loaded.
        let err = crop_margins_native(b"not a pdf", -1.0, 0.0, 0.0, 0.0).unwrap_err();
        assert_eq!(err, "Crop margins cannot be negative");
    }

    #[test]
    fn rejects_non_finite_margins() {
        let err = crop_margins_native(&sample(1), f32::NAN, 0.0, 0.0, 0.0).unwrap_err();
        assert_eq!(err, "Crop margins must be finite numbers");
        let err = crop_margins_native(&sample(1), 0.0, f32::INFINITY, 0.0, 0.0).unwrap_err();
        assert_eq!(err, "Crop margins must be finite numbers");
    }

    #[test]
    fn rejects_margins_exceeding_page_width() {
        // page 0 is 300 wide; 200 + 200 leaves negative width
        let err = crop_margins_native(&sample(1), 0.0, 200.0, 0.0, 200.0).unwrap_err();
        assert_eq!(err, "Crop margins exceed page size");
    }

    #[test]
    fn rejects_margins_exceeding_page_height() {
        // page is 400 tall; 250 + 200 leaves negative height
        let err = crop_margins_native(&sample(1), 250.0, 0.0, 200.0, 0.0).unwrap_err();
        assert_eq!(err, "Crop margins exceed page size");
    }

    #[test]
    fn rejects_margins_consuming_exact_page_size() {
        // 150 + 150 == 300 wide -> zero width is disallowed (TS uses <= 0)
        let err = crop_margins_native(&sample(1), 0.0, 150.0, 0.0, 150.0).unwrap_err();
        assert_eq!(err, "Crop margins exceed page size");
    }

    #[test]
    fn fails_when_any_page_is_too_small() {
        // pages are 300/301/302 wide; 151 + 151 exceeds only page 0
        let err = crop_margins_native(&sample(3), 0.0, 151.0, 0.0, 151.0).unwrap_err();
        assert_eq!(err, "Crop margins exceed page size");
    }

    #[test]
    fn rejects_invalid_pdf_input() {
        assert!(crop_margins_native(&[], 0.0, 0.0, 0.0, 0.0).is_err());
        assert!(crop_margins_native(b"not a pdf", 0.0, 0.0, 0.0, 0.0).is_err());
    }
}
