//! organize.ts parity contracts — intentionally no new operations.
//!
//! Every page-arrangement function in `src/lib/tools/organize.ts` (other than
//! `nUpPdf`/`bookletPdf`, which are placement/rendering work owned by the
//! n-up/booklet module) reduces to an existing [`crate::pages`] operation plus
//! pure index math that stays on the TypeScript side:
//!
//! | organize.ts function | Rust call                              | TS-side responsibility                          |
//! |----------------------|----------------------------------------|-------------------------------------------------|
//! | `extractPages`       | `select_pages_native(bytes, indices)`  | "No pages selected" pre-check                   |
//! | `removePages`        | `select_pages_native(bytes, keep)`     | complement of indices; "Cannot remove every page" |
//! | `reorderPages`       | `select_pages_native(bytes, order)`    | permutation validation (sort + compare)         |
//! | `splitToPages`       | n × `select_pages_native(bytes, [i])`  | loop + progress callbacks                       |
//! | `splitByRanges`      | per range `select_pages_native`        | range parsing; "No ranges specified"            |
//! | `rotatePdf` (all)    | `rotate_all_native(bytes, angle)`      | `angle % 90 === 0` validation                   |
//! | `rotatePdf` (subset) | `rotate_pages_native(bytes, idx, angle)` | `angle % 90 === 0` validation                 |
//!
//! There is no `reverse`, `duplicate`, or `interleave`/alternate-mix function
//! in organize.ts; if such tools are added later, reverse/interleave are pure
//! TS index math over `select_pages`, but a *duplicate* tool is NOT — see
//! `select_pages_duplicate_indices_share_objects_not_copies` below:
//! `select_pages_native` is a same-document rewrite, so repeated indices
//! re-reference one shared page object (and lopdf's renumber pass collapses
//! repeated kid references). Duplication needs real object cloning in Rust.
//!
//! The `#[cfg(test)]` block below locks these contracts in: it drives the
//! `pages.rs` functions with exactly the index math organize.ts performs and
//! asserts behavior parity — ordering, the JS rotation formula
//! `(((current + angle) % 360) + 360) % 360`, friendly error messages, and
//! content-stream / MediaBox preservation across extraction.
//!
//! This module contributes no code to release or wasm builds.

#[cfg(test)]
mod tests {
    use crate::pages::{
        page_count_native, rotate_all_native, rotate_pages_native, select_pages_native,
    };
    use crate::util::effective_media_box;
    use crate::util::fixtures::{page_content_text, page_width, sample, sample_with_text};
    use lopdf::content::Content;
    use lopdf::{dictionary, Document, Object, Stream};

    // -- helpers ------------------------------------------------------------

    /// The exact rotation formula organize.ts applies (organize.ts:100):
    /// `(((current + angle) % 360) + 360) % 360`. Rust `%` matches JS `%`
    /// (truncated division) for these operands, so this mirrors it verbatim.
    fn js_rotation(current: i64, angle: i64) -> i64 {
        (((current + angle) % 360) + 360) % 360
    }

    /// Effective /Rotate of 0-based page `n` (0 when absent), like
    /// pdf-lib's `page.getRotation().angle`.
    fn rotation_of(data: &[u8], n: usize) -> i64 {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        doc.get_dictionary(pages[n])
            .unwrap()
            .get(b"Rotate")
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0)
    }

    /// Re-save `data` with /Rotate set directly on page `n` — simulates an
    /// upstream document that already carries a rotation (even a non-multiple
    /// of 90, which real-world PDFs do contain).
    fn with_rotation(data: &[u8], n: usize, rotate: i64) -> Vec<u8> {
        let mut doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        doc.get_object_mut(pages[n])
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Rotate", rotate);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    /// An n-page PDF whose MediaBox lives only on the root /Pages node, so
    /// pages must inherit it — exercises attribute materialization.
    fn sample_with_inherited_media_box(pages: usize) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();

        let mut kids: Vec<Object> = Vec::new();
        for _ in 0..pages {
            let content = Content { operations: vec![] };
            let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
            });
            kids.push(page_id.into());
        }

        let count = kids.len() as i64;
        let pages_dict = dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => count,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);

        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    // -- extractPages contract ----------------------------------------------

    #[test]
    fn extract_contract_selects_subset_in_order() {
        let out = select_pages_native(&sample(4), &[2, 0]).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count_native(&out).unwrap(), 2);
        assert_eq!(page_width(&out, 0), 302.0);
        assert_eq!(page_width(&out, 1), 300.0);
    }

    #[test]
    fn extract_contract_rejects_empty_selection() {
        // organize.ts throws 'No pages selected' before calling Rust; the Rust
        // backstop must carry the same friendly message.
        let err = select_pages_native(&sample(3), &[])
            .unwrap_err()
            .to_string();
        assert!(err.contains("No pages selected"), "got: {err}");
    }

    #[test]
    fn extract_contract_rejects_out_of_range_index() {
        let err = select_pages_native(&sample(3), &[5])
            .unwrap_err()
            .to_string();
        assert!(err.contains("could not be found"), "got: {err}");
    }

    #[test]
    fn extract_preserves_page_content_streams() {
        let pdf = sample_with_text(3, Some("Marker"));
        let out = select_pages_native(&pdf, &[2, 0]).unwrap();
        assert!(page_content_text(&out, 0).contains("Marker 2"));
        assert!(page_content_text(&out, 1).contains("Marker 0"));
    }

    #[test]
    fn extract_materializes_inherited_media_box() {
        let pdf = sample_with_inherited_media_box(3);
        let out = select_pages_native(&pdf, &[1]).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        assert_eq!(pages.len(), 1);
        // The page must own the box after extraction (survives re-parenting)…
        assert!(doc.get_dictionary(pages[0]).unwrap().has(b"MediaBox"));
        // …and it must still resolve to the inherited value.
        assert_eq!(
            effective_media_box(&doc, pages[0]).unwrap(),
            [0.0, 0.0, 612.0, 792.0]
        );
    }

    // -- removePages contract -------------------------------------------------

    #[test]
    fn remove_pages_contract_keeps_complement() {
        // removePages([2]) on a 5-page doc => TS complementIndices gives
        // [0, 1, 3, 4], which it feeds to extractPages.
        let keep: Vec<u32> = vec![0, 1, 3, 4];
        let out = select_pages_native(&sample(5), &keep).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 4);
        for (slot, &orig) in keep.iter().enumerate() {
            assert_eq!(page_width(&out, slot), 300.0 + orig as f32);
        }
    }

    #[test]
    fn remove_pages_contract_removing_every_page_hits_rust_backstop() {
        // organize.ts throws 'Cannot remove every page' first, but if the
        // empty complement ever reached Rust, select_pages must refuse too.
        assert!(select_pages_native(&sample(2), &[]).is_err());
    }

    // -- reorderPages contract ------------------------------------------------

    #[test]
    fn reorder_contract_applies_full_permutation() {
        let order: Vec<u32> = vec![4, 2, 0, 3, 1];
        let out = select_pages_native(&sample(5), &order).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 5);
        for (slot, &orig) in order.iter().enumerate() {
            assert_eq!(page_width(&out, slot), 300.0 + orig as f32);
        }
    }

    #[test]
    fn reorder_contract_identity_permutation_preserves_order() {
        let out = select_pages_native(&sample(3), &[0, 1, 2]).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 3);
        for i in 0..3 {
            assert_eq!(page_width(&out, i), 300.0 + i as f32);
        }
    }

    #[test]
    fn select_pages_duplicate_indices_share_objects_not_copies() {
        // select_pages is a same-document page-tree rewrite: a duplicated
        // index re-references the SAME page object instead of cloning it (and
        // lopdf 0.34's renumber pass collapses repeated kid references
        // further). It does not error, so TS must never send duplicates —
        // reorderPages' permutation check and unique range parsing are
        // load-bearing, and a future "duplicate page" tool needs real object
        // cloning in Rust, not select_pages index math.
        let out = select_pages_native(&sample(2), &[0, 1, 0]).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        assert_eq!(pages.len(), 3);
        assert_eq!(
            pages[0], pages[2],
            "duplicated index re-references one object"
        );
    }

    // -- splitToPages / splitByRanges contracts --------------------------------

    #[test]
    fn split_to_pages_contract_yields_one_doc_per_page() {
        let pdf = sample(4);
        let total = page_count_native(&pdf).unwrap();
        assert_eq!(total, 4);
        for i in 0..total as u32 {
            let out = select_pages_native(&pdf, &[i]).unwrap();
            assert!(out.starts_with(b"%PDF-"));
            assert_eq!(page_count_native(&out).unwrap(), 1);
            assert_eq!(page_width(&out, 0), 300.0 + i as f32);
        }
    }

    #[test]
    fn split_by_ranges_contract_yields_one_doc_per_range() {
        let pdf = sample(5);
        let ranges: Vec<Vec<u32>> = vec![vec![0, 1], vec![2, 3, 4]];
        let outs: Vec<Vec<u8>> = ranges
            .iter()
            .map(|range| select_pages_native(&pdf, range).unwrap())
            .collect();
        assert_eq!(page_count_native(&outs[0]).unwrap(), 2);
        assert_eq!(page_count_native(&outs[1]).unwrap(), 3);
        assert_eq!(page_width(&outs[0], 1), 301.0);
        assert_eq!(page_width(&outs[1], 0), 302.0);
        assert_eq!(page_width(&outs[1], 2), 304.0);
    }

    #[test]
    fn split_single_page_document() {
        let out = select_pages_native(&sample(1), &[0]).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 1);
        assert_eq!(page_width(&out, 0), 300.0);
    }

    // -- rotatePdf contract -----------------------------------------------------

    #[test]
    fn rotate_all_contract_sets_rotate_on_every_page() {
        let pdf = sample(3);
        let out = rotate_all_native(&pdf, 90).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count_native(&out).unwrap(), 3);
        for i in 0..3 {
            assert_eq!(rotation_of(&out, i), js_rotation(0, 90));
        }
    }

    #[test]
    fn rotate_pages_contract_rotates_only_targets() {
        // rotatePdf(bytes, 180, [0, 2]) on a 3-page doc.
        let out = rotate_pages_native(&sample(3), &[0, 2], 180).unwrap();
        assert_eq!(rotation_of(&out, 0), 180);
        assert_eq!(rotation_of(&out, 1), 0);
        assert_eq!(rotation_of(&out, 2), 180);
    }

    #[test]
    fn rotation_formula_matches_js_for_negative_angles() {
        // JS: (((0 + -90) % 360) + 360) % 360 === 270. Rust must agree even
        // though bare `-90 % 360` is -90 in both languages.
        let out = rotate_all_native(&sample(1), -90).unwrap();
        assert_eq!(rotation_of(&out, 0), 270);
        assert_eq!(rotation_of(&out, 0), js_rotation(0, -90));
    }

    #[test]
    fn rotation_formula_matches_js_for_wraparound() {
        // current=270, angle=180 -> 90.
        let pdf = with_rotation(&sample(1), 0, 270);
        let out = rotate_all_native(&pdf, 180).unwrap();
        assert_eq!(rotation_of(&out, 0), 90);
        assert_eq!(rotation_of(&out, 0), js_rotation(270, 180));
    }

    #[test]
    fn rotation_formula_matches_js_for_odd_existing_rotation() {
        // Real-world PDFs occasionally carry non-right-angle /Rotate values;
        // current=45, angle=315 -> (((45+315)%360)+360)%360 = 0.
        let pdf = with_rotation(&sample(1), 0, 45);
        let out = rotate_all_native(&pdf, 315).unwrap();
        assert_eq!(rotation_of(&out, 0), 0);
        assert_eq!(rotation_of(&out, 0), js_rotation(45, 315));
    }

    #[test]
    fn rotation_accumulates_like_sequential_rotate_pdf_calls() {
        // rotate 90°, then 180°, then 90° => 360° ≡ 0°.
        let a = rotate_all_native(&sample(1), 90).unwrap();
        let b = rotate_all_native(&a, 180).unwrap();
        let c = rotate_all_native(&b, 90).unwrap();
        assert_eq!(rotation_of(&c, 0), 0);
    }

    #[test]
    fn rotate_pages_with_empty_indices_is_a_noop() {
        // rotatePdf with indices=[] never enters its loop; Rust must likewise
        // succeed and change nothing.
        let pdf = sample(2);
        let out = rotate_pages_native(&pdf, &[], 90).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 2);
        assert_eq!(rotation_of(&out, 0), 0);
        assert_eq!(rotation_of(&out, 1), 0);
    }

    #[test]
    fn rotate_pages_rejects_out_of_range_index() {
        // organize.ts throws 'Page index 5 out of range'; Rust's message is
        // lopdf's 1-based 'Page number 6 could not be found'.
        let err = rotate_pages_native(&sample(2), &[5], 90)
            .unwrap_err()
            .to_string();
        assert!(err.contains("could not be found"), "got: {err}");
    }

    #[test]
    fn rotate_zero_degrees_normalizes_missing_rotate_to_zero() {
        // angle=0 passes the TS `% 90` check; result must stay upright and valid.
        let out = rotate_all_native(&sample(2), 0).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count_native(&out).unwrap(), 2);
        assert_eq!(rotation_of(&out, 0), 0);
        assert_eq!(rotation_of(&out, 1), 0);
    }
}
