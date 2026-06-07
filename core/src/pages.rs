//! Page-tree operations: rotation, counting, selection/reordering.

use lopdf::{Document, Error as LopdfError, Object};

use crate::util::{materialize_inherited_page_attrs, root_pages_id, save_compact};

/// Rotate every page by `degrees` (relative to its current rotation).
pub fn rotate_all_native(data: &[u8], degrees: i64) -> Result<Vec<u8>, LopdfError> {
    let mut doc = Document::load_mem(data)?;
    let page_ids: Vec<_> = doc.get_pages().into_values().collect();
    rotate_ids(&mut doc, &page_ids, degrees)?;
    let mut buf = Vec::new();
    doc.save_to(&mut buf)?;
    Ok(buf)
}

/// Rotate only the given 0-based pages by `degrees` (relative).
pub fn rotate_pages_native(data: &[u8], indices: &[u32], degrees: i64) -> Result<Vec<u8>, LopdfError> {
    let mut doc = Document::load_mem(data)?;
    let pages: Vec<_> = doc.page_iter().collect();
    let mut ids = Vec::with_capacity(indices.len());
    for &index in indices {
        ids.push(
            *pages
                .get(index as usize)
                .ok_or(LopdfError::PageNumberNotFound(index + 1))?,
        );
    }
    rotate_ids(&mut doc, &ids, degrees)?;
    let mut buf = Vec::new();
    doc.save_to(&mut buf)?;
    Ok(buf)
}

fn rotate_ids(doc: &mut Document, ids: &[lopdf::ObjectId], degrees: i64) -> Result<(), LopdfError> {
    for &id in ids {
        if let Ok(obj) = doc.get_object_mut(id) {
            if let Ok(dict) = obj.as_dict_mut() {
                let current = dict.get(b"Rotate").ok().and_then(|o| o.as_i64().ok()).unwrap_or(0);
                let next = (((current + degrees) % 360) + 360) % 360;
                dict.set("Rotate", next);
            }
        }
    }
    Ok(())
}

/// Losslessly optimize: prune unused objects and flate-compress streams.
pub fn optimize_native(data: &[u8]) -> Result<Vec<u8>, LopdfError> {
    let mut doc = Document::load_mem(data)?;
    doc.prune_objects();
    doc.compress();
    let mut buf = Vec::new();
    doc.save_to(&mut buf)?;
    Ok(buf)
}

/// Number of pages in the document.
pub fn page_count_native(data: &[u8]) -> Result<usize, LopdfError> {
    Ok(Document::load_mem(data)?.get_pages().len())
}

/// Build a new PDF containing the selected 0-based page indices, in order.
///
/// This is intentionally a same-document page-tree rewrite rather than a cross-document
/// merger: all selected pages keep their resource/content references, while unreferenced
/// objects are pruned before writing.
pub fn select_pages_native(data: &[u8], indices: &[u32]) -> Result<Vec<u8>, LopdfError> {
    if indices.is_empty() {
        return Err(LopdfError::Syntax("No pages selected".to_string()));
    }

    let mut doc = Document::load_mem(data)?;
    let pages: Vec<_> = doc.page_iter().collect();
    let pages_id = root_pages_id(&doc)?;
    let mut kids = Vec::with_capacity(indices.len());

    for &index in indices {
        let page_id = *pages
            .get(index as usize)
            .ok_or(LopdfError::PageNumberNotFound(index + 1))?;
        materialize_inherited_page_attrs(&mut doc, page_id)?;
        doc.get_object_mut(page_id)?.as_dict_mut()?.set("Parent", pages_id);
        kids.push(Object::Reference(page_id));
    }

    let page_tree = doc.get_object_mut(pages_id)?.as_dict_mut()?;
    page_tree.set("Kids", kids);
    page_tree.set("Count", indices.len() as i64);

    if let Ok(catalog) = doc.catalog_mut() {
        // Page labels/outlines/actions/forms can point at removed pages. pdf-lib's
        // copyPages path also drops these document-level structures, so keep parity.
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
    }

    save_compact(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fixtures::{page_width, sample};
    use lopdf::Document;

    #[test]
    fn counts_pages() {
        assert_eq!(page_count_native(&sample(3)).unwrap(), 3);
    }

    #[test]
    fn rotates_all_pages() {
        let out = rotate_all_native(&sample(2), 90).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        for (_, id) in doc.get_pages() {
            let dict = doc.get_object(id).unwrap().as_dict().unwrap();
            assert_eq!(dict.get(b"Rotate").unwrap().as_i64().unwrap(), 90);
        }
    }

    #[test]
    fn rotation_wraps_modulo_360() {
        let once = rotate_all_native(&sample(1), 270).unwrap();
        let twice = rotate_all_native(&once, 180).unwrap(); // 270 + 180 = 450 -> 90
        let doc = Document::load_mem(&twice).unwrap();
        let id = *doc.get_pages().values().next().unwrap();
        let dict = doc.get_object(id).unwrap().as_dict().unwrap();
        assert_eq!(dict.get(b"Rotate").unwrap().as_i64().unwrap(), 90);
    }

    #[test]
    fn rotates_only_selected_pages() {
        let out = rotate_pages_native(&sample(3), &[1], 180).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let rot = |i: usize| {
            doc.get_dictionary(pages[i])
                .unwrap()
                .get(b"Rotate")
                .ok()
                .and_then(|o| o.as_i64().ok())
                .unwrap_or(0)
        };
        assert_eq!(rot(0), 0);
        assert_eq!(rot(1), 180);
        assert_eq!(rot(2), 0);
    }

    #[test]
    fn rotate_pages_rejects_out_of_range() {
        assert!(rotate_pages_native(&sample(2), &[5], 90).is_err());
    }

    #[test]
    fn optimize_preserves_pages_and_validity() {
        let out = optimize_native(&sample(4)).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 4);
        assert!(out.starts_with(b"%PDF-"));
    }

    #[test]
    fn select_pages_extracts_subset_in_requested_order() {
        let out = select_pages_native(&sample(4), &[2, 0]).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count_native(&out).unwrap(), 2);
        assert_eq!(page_width(&out, 0), 302.0);
        assert_eq!(page_width(&out, 1), 300.0);
    }

    #[test]
    fn select_pages_rejects_empty_and_out_of_range() {
        assert!(select_pages_native(&sample(2), &[]).is_err());
        assert!(select_pages_native(&sample(2), &[9]).is_err());
    }
}
