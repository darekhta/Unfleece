//! Shared lopdf helpers used across operation modules.

use lopdf::{Document, Error as LopdfError, Object, ObjectId};

/// The root /Pages node of the document catalog.
pub fn root_pages_id(doc: &Document) -> Result<ObjectId, LopdfError> {
    doc.catalog()?.get(b"Pages")?.as_reference()
}

/// Walk up the page tree looking for an inherited attribute value.
pub fn inherited_page_value(
    doc: &Document,
    page_id: ObjectId,
    key: &[u8],
) -> Result<Option<Object>, LopdfError> {
    let mut parent = doc
        .get_dictionary(page_id)?
        .get(b"Parent")
        .and_then(Object::as_reference)
        .ok();

    while let Some(parent_id) = parent {
        let dict = doc.get_dictionary(parent_id)?;
        if let Ok(value) = dict.get(key) {
            return Ok(Some(value.clone()));
        }
        parent = dict.get(b"Parent").and_then(Object::as_reference).ok();
    }

    Ok(None)
}

/// Copy inheritable attributes (Resources/MediaBox/CropBox/Rotate) down onto the
/// page dictionary so the page survives being re-parented or extracted.
pub fn materialize_inherited_page_attrs(
    doc: &mut Document,
    page_id: ObjectId,
) -> Result<(), LopdfError> {
    for key in [
        b"Resources".as_slice(),
        b"MediaBox".as_slice(),
        b"CropBox".as_slice(),
        b"Rotate".as_slice(),
    ] {
        if !doc.get_dictionary(page_id)?.has(key) {
            if let Some(value) = inherited_page_value(doc, page_id, key)? {
                doc.get_object_mut(page_id)?
                    .as_dict_mut()?
                    .set(key.to_vec(), value);
            }
        }
    }
    Ok(())
}

/// The effective MediaBox of a page (own or inherited), as `[x0, y0, x1, y1]`.
pub fn effective_media_box(doc: &Document, page_id: ObjectId) -> Result<[f32; 4], LopdfError> {
    let raw = match doc.get_dictionary(page_id)?.get(b"MediaBox") {
        Ok(v) => Some(v.clone()),
        Err(_) => inherited_page_value(doc, page_id, b"MediaBox")?,
    }
    .ok_or_else(|| LopdfError::Syntax("Page has no MediaBox".to_string()))?;

    let arr = raw
        .as_array()
        .map_err(|_| LopdfError::Syntax("MediaBox is not an array".to_string()))?;
    if arr.len() != 4 {
        return Err(LopdfError::Syntax(
            "MediaBox must have 4 numbers".to_string(),
        ));
    }
    let mut out = [0f32; 4];
    for (i, obj) in arr.iter().enumerate() {
        out[i] = number_as_f32(obj)
            .ok_or_else(|| LopdfError::Syntax("MediaBox entry is not a number".to_string()))?;
    }
    Ok(out)
}

/// Treat both Integer and Real PDF numbers as f32.
pub fn number_as_f32(obj: &Object) -> Option<f32> {
    match obj {
        Object::Integer(i) => Some(*i as f32),
        Object::Real(r) => Some(*r),
        _ => None,
    }
}

/// Save a document to bytes after the standard prune/compress pass.
pub fn save_compact(mut doc: Document) -> Result<Vec<u8>, LopdfError> {
    doc.prune_objects();
    doc.compress();
    doc.renumber_objects();
    let mut buf = Vec::new();
    doc.save_to(&mut buf)?;
    Ok(buf)
}

// ---------------------------------------------------------------------------
// Test fixtures shared by every module's #[cfg(test)] block.
// ---------------------------------------------------------------------------
#[cfg(test)]
pub mod fixtures {
    use lopdf::content::{Content, Operation};
    use lopdf::{dictionary, Document, Object, Stream};

    /// A valid 1×1 PNG (RGBA).
    pub const PNG_1X1: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4,
        0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 96, 0, 0, 0, 6, 0,
        2, 48, 129, 208, 47, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    /// Build a minimal valid n-page PDF. Page i has MediaBox width 300+i so
    /// tests can tell pages apart, and a real (empty) content stream.
    pub fn sample(pages: usize) -> Vec<u8> {
        sample_with_text(pages, None)
    }

    /// Like [`sample`], but each page shows `text` via the Standard-14 Helvetica
    /// font — pages then have non-trivial Resources and Contents.
    pub fn sample_with_text(pages: usize, text: Option<&str>) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();

        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });

        let mut kids: Vec<Object> = Vec::new();
        for i in 0..pages {
            let operations = match text {
                Some(t) => vec![
                    Operation::new("BT", vec![]),
                    Operation::new("Tf", vec!["F1".into(), 24.into()]),
                    Operation::new("Td", vec![50.into(), 350.into()]),
                    Operation::new("Tj", vec![Object::string_literal(format!("{t} {i}"))]),
                    Operation::new("ET", vec![]),
                ],
                None => vec![],
            };
            let content = Content { operations };
            let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
                "MediaBox" => vec![0.into(), 0.into(), (300 + i as i64).into(), 400.into()],
            });
            kids.push(page_id.into());
        }

        let count = kids.len() as i64;
        let pages_dict = dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => count,
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);

        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    /// MediaBox width of page `n` (0-based) — pairs with [`sample`]'s 300+i widths.
    pub fn page_width(data: &[u8], n: usize) -> f32 {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let media_box = doc
            .get_dictionary(pages[n])
            .unwrap()
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        crate::util::number_as_f32(&media_box[2]).unwrap()
    }

    /// Decode every content stream of page `n` to a string (for "does the page
    /// draw X" assertions).
    pub fn page_content_text(data: &[u8], n: usize) -> String {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let content = doc.get_page_content(pages[n]).unwrap();
        String::from_utf8_lossy(&content).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fixtures::sample;

    #[test]
    fn effective_media_box_rejects_wrong_length_array() {
        let mut doc = Document::load_mem(&sample(1)).unwrap();
        let page_id = doc.page_iter().next().unwrap();
        doc.get_object_mut(page_id)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("MediaBox", vec![0.into(), 0.into(), 300.into()]);

        let err = effective_media_box(&doc, page_id).unwrap_err();
        assert!(matches!(
            err,
            LopdfError::Syntax(message) if message == "MediaBox must have 4 numbers"
        ));
    }
}
