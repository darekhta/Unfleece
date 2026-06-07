//! Cross-document PDF merging.
//!
//! Mirrors the TypeScript `mergePdfs` (pdf-lib `copyPages` + `addPage`): each
//! source document's pages are appended, in order, to a single fresh `/Pages`
//! tree under a fresh catalog. Pages are made self-contained first (inherited
//! Resources/MediaBox/CropBox/Rotate are materialized onto the page dict), and
//! document-level structures — outlines, forms, name trees, dests — are
//! intentionally dropped, matching pdf-lib's `copyPages` semantics.

use lopdf::{dictionary, Document, Object, ObjectId};

use crate::pack::PackReader;
use crate::util::{materialize_inherited_page_attrs, save_compact};

const MERGE_PACK_MAGIC: &[u8; 4] = b"UFMG";

/// Merge multiple PDF documents into one, in pack order.
///
/// Pack format (`UFMG`, little-endian):
/// - `UFMG` magic
/// - u32 document count
/// - repeated documents: u32 byte length, PDF bytes
///
/// Returns the merged PDF bytes. Errors with a friendly message on an empty
/// pack, malformed pack framing, or invalid PDF bytes.
pub fn merge_pdfs_native(pack: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = PackReader::new(pack);
    reader.expect_magic(MERGE_PACK_MAGIC)?;

    let doc_count = reader.read_u32()?;
    if doc_count == 0 {
        return Err("No files to merge".to_string());
    }

    let mut out = Document::with_version("1.5");
    let mut page_ids: Vec<ObjectId> = Vec::new();
    let mut max_id: u32 = 0;

    for _ in 0..doc_count {
        let bytes = reader.read_bytes()?;
        let mut src =
            Document::load_mem(bytes).map_err(|e| format!("Invalid PDF document: {e}"))?;

        // Make every page self-contained before it leaves its original tree.
        let src_pages: Vec<ObjectId> = src.page_iter().collect();
        for &id in &src_pages {
            materialize_inherited_page_attrs(&mut src, id).map_err(|e| e.to_string())?;
        }

        // Shift the source ids past everything merged so far, then move the
        // whole object graph across. `renumber_objects_with` rewrites every
        // reference (trailer included), so the renumbered page ids can be read
        // back off the source page tree, still in page order.
        src.renumber_objects_with(max_id + 1);
        max_id = src.max_id;
        page_ids.extend(src.page_iter());
        out.objects.extend(src.objects);
    }
    reader.expect_done()?;

    if page_ids.is_empty() {
        return Err("Merge produced zero pages".to_string());
    }

    // Adopt every page into a fresh /Pages tree under a fresh catalog. The
    // source catalogs, page-tree nodes, outlines, forms and name trees become
    // unreachable and are pruned by `save_compact` (pdf-lib copyPages parity).
    out.max_id = max_id;
    let pages_id = out.new_object_id();
    for &id in &page_ids {
        out.get_object_mut(id)
            .map_err(|e| e.to_string())?
            .as_dict_mut()
            .map_err(|e| e.to_string())?
            .set("Parent", pages_id);
    }

    let kids: Vec<Object> = page_ids.iter().map(|&id| Object::Reference(id)).collect();
    let count = kids.len() as i64;
    out.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => count,
        }),
    );
    let catalog_id = out.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    out.trailer.set("Root", catalog_id);

    save_compact(out).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackWriter;
    use crate::pages::page_count_native;
    use crate::util::effective_media_box;
    use crate::util::fixtures::{page_content_text, page_width, sample, sample_with_text};
    use lopdf::content::Content;
    use lopdf::{dictionary, Document, Object, Stream};

    fn pack(docs: &[&[u8]]) -> Vec<u8> {
        let mut w = PackWriter::new(MERGE_PACK_MAGIC).u32(docs.len() as u32);
        for doc in docs {
            w = w.bytes(doc);
        }
        w.finish()
    }

    /// A 1-page PDF whose MediaBox lives only on the /Pages node (inherited).
    fn sample_inherited_media_box(width: i64) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let content_id =
            doc.add_object(Stream::new(dictionary! {}, Content { operations: vec![] }.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
                "MediaBox" => vec![0.into(), 0.into(), width.into(), 800.into()],
            }),
        );
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    /// A 1-page PDF with a non-integer (Real) MediaBox.
    fn sample_real_media_box(width: f32, height: f32) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let content_id =
            doc.add_object(Stream::new(dictionary! {}, Content { operations: vec![] }.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), Object::Real(width), Object::Real(height)],
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    /// [`sample`] with document-level structures (Outlines/AcroForm/Names) added.
    fn sample_with_doc_structures(pages: usize) -> Vec<u8> {
        let mut doc = Document::load_mem(&sample(pages)).unwrap();
        let outlines_id = doc.add_object(dictionary! { "Type" => "Outlines", "Count" => 0 });
        let root_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let catalog = doc.get_object_mut(root_id).unwrap().as_dict_mut().unwrap();
        catalog.set("Outlines", outlines_id);
        catalog.set("AcroForm", dictionary! { "Fields" => Vec::<Object>::new() });
        catalog.set("Names", dictionary! {});
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    #[test]
    fn merges_two_docs_in_order() {
        let out = merge_pdfs_native(&pack(&[&sample(3), &sample(2)])).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 5);
        // sample(n) page i has width 300+i, so order is fully observable.
        for (i, want) in [300.0, 301.0, 302.0, 300.0, 301.0].iter().enumerate() {
            assert_eq!(page_width(&out, i), *want, "page {i}");
        }
    }

    #[test]
    fn merges_three_docs_in_order() {
        let out = merge_pdfs_native(&pack(&[&sample(2), &sample(1), &sample(2)])).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 5);
        for (i, want) in [300.0, 301.0, 300.0, 300.0, 301.0].iter().enumerate() {
            assert_eq!(page_width(&out, i), *want, "page {i}");
        }
    }

    #[test]
    fn single_doc_round_trips() {
        let out = merge_pdfs_native(&pack(&[&sample(3)])).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 3);
        for i in 0..3 {
            assert_eq!(page_width(&out, i), 300.0 + i as f32);
        }
    }

    #[test]
    fn respects_input_order() {
        let ab = merge_pdfs_native(&pack(&[&sample(2), &sample(3)])).unwrap();
        let ba = merge_pdfs_native(&pack(&[&sample(3), &sample(2)])).unwrap();
        assert_eq!(page_width(&ab, 2), 300.0); // first page of second doc
        assert_eq!(page_width(&ba, 2), 302.0); // third page of first doc
    }

    #[test]
    fn output_is_valid_pdf() {
        let out = merge_pdfs_native(&pack(&[&sample(2), &sample(2)])).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        let doc = Document::load_mem(&out).unwrap();
        assert_eq!(doc.page_iter().count(), 4);
    }

    #[test]
    fn preserves_content_streams_across_docs() {
        let a = sample_with_text(2, Some("Doc A"));
        let b = sample_with_text(2, Some("Doc B"));
        let out = merge_pdfs_native(&pack(&[&a, &b])).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 4);
        assert!(page_content_text(&out, 0).contains("Doc A 0"));
        assert!(page_content_text(&out, 1).contains("Doc A 1"));
        assert!(page_content_text(&out, 2).contains("Doc B 0"));
        assert!(page_content_text(&out, 3).contains("Doc B 1"));
    }

    #[test]
    fn preserves_mixed_page_sizes() {
        let a = sample_real_media_box(612.5, 792.25);
        let b = sample_real_media_box(1224.0, 144.0);
        let out = merge_pdfs_native(&pack(&[&a, &b])).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        assert_eq!(effective_media_box(&doc, pages[0]).unwrap(), [0.0, 0.0, 612.5, 792.25]);
        assert_eq!(effective_media_box(&doc, pages[1]).unwrap(), [0.0, 0.0, 1224.0, 144.0]);
    }

    #[test]
    fn materializes_inherited_media_box_onto_pages() {
        let out = merge_pdfs_native(&pack(&[&sample(1), &sample_inherited_media_box(720)])).unwrap();
        // page_width reads MediaBox directly off the page dict, so this also
        // proves the inherited attribute was copied down before re-parenting.
        assert_eq!(page_width(&out, 0), 300.0);
        assert_eq!(page_width(&out, 1), 720.0);
    }

    #[test]
    fn rebuilds_page_tree_with_accurate_count_and_parents() {
        let out = merge_pdfs_native(&pack(&[&sample(3), &sample(2)])).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        let root_id = doc.catalog().unwrap().get(b"Pages").unwrap().as_reference().unwrap();
        let pages_dict = doc.get_dictionary(root_id).unwrap();
        assert_eq!(pages_dict.get(b"Count").unwrap().as_i64().unwrap(), 5);
        assert_eq!(pages_dict.get(b"Kids").unwrap().as_array().unwrap().len(), 5);
        for id in doc.page_iter().collect::<Vec<_>>() {
            let parent = doc.get_dictionary(id).unwrap().get(b"Parent").unwrap().as_reference().unwrap();
            assert_eq!(parent, root_id);
        }
    }

    #[test]
    fn drops_document_level_structures() {
        let out = merge_pdfs_native(&pack(&[&sample_with_doc_structures(2), &sample(1)])).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        let catalog = doc.catalog().unwrap();
        for key in [b"Outlines".as_slice(), b"AcroForm".as_slice(), b"Names".as_slice()] {
            assert!(!catalog.has(key), "catalog still has {}", String::from_utf8_lossy(key));
        }
        // The orphaned outline object itself must have been pruned too.
        let has_outlines_obj = doc.objects.values().any(|o| {
            o.as_dict()
                .ok()
                .and_then(|d| d.get(b"Type").ok())
                .and_then(|t| t.as_name().ok())
                .is_some_and(|n| n == b"Outlines")
        });
        assert!(!has_outlines_obj);
    }

    #[test]
    fn skips_zero_page_documents() {
        let out = merge_pdfs_native(&pack(&[&sample(2), &sample(0), &sample(1)])).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 3);
        assert_eq!(page_width(&out, 2), 300.0);
    }

    #[test]
    fn rejects_merge_of_only_zero_page_documents() {
        let err = merge_pdfs_native(&pack(&[&sample(0)])).unwrap_err();
        assert_eq!(err, "Merge produced zero pages");
    }

    #[test]
    fn merges_many_single_page_docs() {
        let docs: Vec<Vec<u8>> = (0..10).map(|_| sample(1)).collect();
        let refs: Vec<&[u8]> = docs.iter().map(Vec::as_slice).collect();
        let out = merge_pdfs_native(&pack(&refs)).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 10);
        for i in 0..10 {
            assert_eq!(page_width(&out, i), 300.0);
        }
    }

    #[test]
    fn merges_large_docs() {
        let out = merge_pdfs_native(&pack(&[&sample(100), &sample(50)])).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 150);
        assert_eq!(page_width(&out, 99), 399.0); // last page of first doc
        assert_eq!(page_width(&out, 100), 300.0); // first page of second doc
        assert_eq!(page_width(&out, 149), 349.0); // last page overall
    }

    #[test]
    fn merged_output_can_be_merged_again() {
        let once = merge_pdfs_native(&pack(&[&sample(2), &sample(1)])).unwrap();
        let twice = merge_pdfs_native(&pack(&[&once, &sample(3)])).unwrap();
        assert_eq!(page_count_native(&twice).unwrap(), 6);
        for (i, want) in [300.0, 301.0, 300.0, 300.0, 301.0, 302.0].iter().enumerate() {
            assert_eq!(page_width(&twice, i), *want, "page {i}");
        }
    }

    #[test]
    fn rejects_empty_doc_list() {
        let err = merge_pdfs_native(&pack(&[])).unwrap_err();
        assert_eq!(err, "No files to merge");
    }

    #[test]
    fn rejects_wrong_magic() {
        let bad = PackWriter::new(b"NOPE").u32(1).bytes(&sample(1)).finish();
        assert_eq!(merge_pdfs_native(&bad).unwrap_err(), "Invalid input pack");
    }

    #[test]
    fn rejects_empty_input() {
        assert!(merge_pdfs_native(&[]).is_err());
    }

    #[test]
    fn rejects_truncated_pack() {
        // Declares two documents but provides only one.
        let truncated = PackWriter::new(MERGE_PACK_MAGIC).u32(2).bytes(&sample(1)).finish();
        assert_eq!(merge_pdfs_native(&truncated).unwrap_err(), "Input pack ended early");

        // Magic only — count itself is missing.
        assert_eq!(
            merge_pdfs_native(MERGE_PACK_MAGIC).unwrap_err(),
            "Input pack ended early"
        );
    }

    #[test]
    fn rejects_trailing_data() {
        let mut p = pack(&[&sample(1)]);
        p.extend_from_slice(b"junk");
        assert_eq!(merge_pdfs_native(&p).unwrap_err(), "Input pack has trailing data");
    }

    #[test]
    fn rejects_non_pdf_bytes() {
        let err = merge_pdfs_native(&pack(&[&sample(1), b"definitely not a pdf"])).unwrap_err();
        assert!(err.contains("Invalid PDF document"), "got: {err}");
    }
}
