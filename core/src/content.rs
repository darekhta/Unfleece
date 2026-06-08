//! Content-stream + page-resource plumbing shared by the modules that append a
//! new content stream onto existing pages (`stamp_text.rs`, `assemble.rs`).
//!
//! These helpers were copy-pasted between those two modules; they now live here
//! as `pub(crate)` functions so the behavior (and its byte-level output) has a
//! single source of truth:
//!
//! - [`content_bytes`] / [`add_content_stream`] — encode operations to a
//!   newline-padded stream so adjacent streams in a `/Contents` array cannot
//!   token-fuse.
//! - [`wrap_stream_ids`] — the shared `q` / `Q` wrapper streams used to bracket
//!   pre-existing page content.
//! - [`add_resource_entry`] — insert a value under a collision-free key in a
//!   page's `/Resources` sub-dictionary, following indirect references and
//!   creating any missing dictionaries.
//! - [`append_to_page_contents`] — append a stream to a page's `/Contents`,
//!   bracketing any pre-existing content and normalizing `/Contents` to an array.
//!
//! `stamp_image.rs` reuses [`wrap_stream_ids`]; it keeps its own batched
//! resource-merge / content-append routines because they follow a different
//! design (BTreeMap accumulation across many stamps per page).

use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};

/// Encode a list of content operations to stream bytes.
///
/// The bytes are padded with leading/trailing newlines: streams in a
/// `/Contents` array are logically concatenated, and without the padding a
/// stream ending in `Q` followed by one starting with `q` would fuse into a
/// single bogus `Qq` token (lopdf concatenates them byte-for-byte on read).
pub(crate) fn content_bytes(operations: Vec<Operation>) -> Result<Vec<u8>, String> {
    let body = Content { operations }.encode().map_err(|e| e.to_string())?;
    let mut bytes = Vec::with_capacity(body.len() + 2);
    bytes.push(b'\n');
    bytes.extend(body);
    bytes.push(b'\n');
    Ok(bytes)
}

/// Add a content stream object holding `operations`; returns its id.
pub(crate) fn add_content_stream(
    doc: &mut Document,
    operations: Vec<Operation>,
) -> Result<ObjectId, String> {
    Ok(doc.add_object(Stream::new(dictionary! {}, content_bytes(operations)?)))
}

/// Shared `q` / `Q` wrapper streams used to bracket pre-existing page content.
pub(crate) fn wrap_stream_ids(doc: &mut Document) -> Result<(ObjectId, ObjectId), String> {
    let push = content_bytes(vec![Operation::new("q", vec![])])?;
    let pop = content_bytes(vec![Operation::new("Q", vec![])])?;
    Ok((
        doc.add_object(Stream::new(dictionary! {}, push)),
        doc.add_object(Stream::new(dictionary! {}, pop)),
    ))
}

/// Where a page's `/Resources` dictionary lives.
enum ResourcesLoc {
    /// Inline dictionary in the page dict (created when missing).
    Inline,
    /// Indirect object referenced from the page dict.
    Indirect(ObjectId),
}

fn locate_resources(doc: &mut Document, page_id: ObjectId) -> Result<ResourcesLoc, String> {
    enum Found {
        Inline,
        Indirect(ObjectId),
        Missing,
    }
    let found = {
        let dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;
        match dict.get(b"Resources") {
            Ok(Object::Reference(id)) => Found::Indirect(*id),
            Ok(Object::Dictionary(_)) => Found::Inline,
            Ok(_) => return Err("Page /Resources is not a dictionary".to_string()),
            Err(_) => Found::Missing,
        }
    };
    match found {
        Found::Inline => Ok(ResourcesLoc::Inline),
        Found::Indirect(id) => {
            doc.get_dictionary(id)
                .map_err(|_| "Page /Resources reference is not a dictionary".to_string())?;
            Ok(ResourcesLoc::Indirect(id))
        }
        Found::Missing => {
            doc.get_object_mut(page_id)
                .map_err(|e| e.to_string())?
                .as_dict_mut()
                .map_err(|e| e.to_string())?
                .set("Resources", Dictionary::new());
            Ok(ResourcesLoc::Inline)
        }
    }
}

fn resources_dict<'a>(
    doc: &'a Document,
    page_id: ObjectId,
    loc: &ResourcesLoc,
) -> Result<&'a Dictionary, String> {
    let obj = match loc {
        ResourcesLoc::Inline => doc
            .get_dictionary(page_id)
            .map_err(|e| e.to_string())?
            .get(b"Resources")
            .map_err(|e| e.to_string())?,
        ResourcesLoc::Indirect(id) => doc.get_object(*id).map_err(|e| e.to_string())?,
    };
    obj.as_dict().map_err(|e| e.to_string())
}

fn resources_dict_mut<'a>(
    doc: &'a mut Document,
    page_id: ObjectId,
    loc: &ResourcesLoc,
) -> Result<&'a mut Dictionary, String> {
    let obj = match loc {
        ResourcesLoc::Inline => doc
            .get_object_mut(page_id)
            .map_err(|e| e.to_string())?
            .as_dict_mut()
            .map_err(|e| e.to_string())?
            .get_mut(b"Resources")
            .map_err(|e| e.to_string())?,
        ResourcesLoc::Indirect(id) => doc.get_object_mut(*id).map_err(|e| e.to_string())?,
    };
    obj.as_dict_mut().map_err(|e| e.to_string())
}

/// Insert `value` under a fresh key in the `category` (e.g. `Font` / `ExtGState`)
/// sub-dictionary of the page's Resources, following indirect references and
/// creating missing dictionaries. Returns the chosen resource key.
pub(crate) fn add_resource_entry(
    doc: &mut Document,
    page_id: ObjectId,
    category: &str,
    base_key: &str,
    value: Object,
) -> Result<String, String> {
    let res_loc = locate_resources(doc, page_id)?;

    /// Where the category sub-dictionary lives.
    enum CatLoc {
        Inline,
        Indirect(ObjectId),
    }
    let cat_loc = {
        let found = match resources_dict(doc, page_id, &res_loc)?.get(category.as_bytes()) {
            Ok(Object::Reference(id)) => Some(CatLoc::Indirect(*id)),
            Ok(Object::Dictionary(_)) => Some(CatLoc::Inline),
            Ok(_) => {
                return Err(format!(
                    "Page /{category} resource entry is not a dictionary"
                ))
            }
            Err(_) => None,
        };
        match found {
            Some(CatLoc::Indirect(id)) => {
                doc.get_dictionary(id).map_err(|_| {
                    format!("Page /{category} resource reference is not a dictionary")
                })?;
                CatLoc::Indirect(id)
            }
            Some(CatLoc::Inline) => CatLoc::Inline,
            None => {
                resources_dict_mut(doc, page_id, &res_loc)?.set(category, Dictionary::new());
                CatLoc::Inline
            }
        }
    };

    // Pick a key that does not collide with existing entries.
    let existing: Vec<Vec<u8>> = {
        let cat = match &cat_loc {
            CatLoc::Inline => resources_dict(doc, page_id, &res_loc)?
                .get(category.as_bytes())
                .and_then(Object::as_dict)
                .map_err(|e| e.to_string())?,
            CatLoc::Indirect(id) => doc.get_dictionary(*id).map_err(|e| e.to_string())?,
        };
        cat.iter().map(|(k, _)| k.to_vec()).collect()
    };
    let mut key = base_key.to_string();
    let mut suffix = 0u32;
    while existing.iter().any(|k| k.as_slice() == key.as_bytes()) {
        suffix += 1;
        key = format!("{base_key}{suffix}");
    }

    let cat = match &cat_loc {
        CatLoc::Inline => resources_dict_mut(doc, page_id, &res_loc)?
            .get_mut(category.as_bytes())
            .map_err(|e| e.to_string())?
            .as_dict_mut()
            .map_err(|e| e.to_string())?,
        CatLoc::Indirect(id) => doc
            .get_object_mut(*id)
            .map_err(|e| e.to_string())?
            .as_dict_mut()
            .map_err(|e| e.to_string())?,
    };
    cat.set(key.clone(), value);
    Ok(key)
}

/// Append `appended_id` to the page's `/Contents`, first bracketing any
/// pre-existing content with the shared `q`/`Q` wrapper streams and normalizing
/// `/Contents` into an array.
pub(crate) fn append_to_page_contents(
    doc: &mut Document,
    page_id: ObjectId,
    appended_id: ObjectId,
    push_id: ObjectId,
    pop_id: ObjectId,
) -> Result<(), String> {
    enum Existing {
        None,
        Items(Vec<Object>),
        Inline(Box<Stream>),
    }
    let existing = {
        let dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;
        match dict.get(b"Contents") {
            Ok(Object::Reference(id)) => Existing::Items(vec![Object::Reference(*id)]),
            Ok(Object::Array(items)) => Existing::Items(items.clone()),
            Ok(Object::Stream(s)) => Existing::Inline(Box::new(s.clone())),
            Ok(_) => return Err("Page /Contents is not a stream or array".to_string()),
            Err(_) => Existing::None,
        }
    };
    let items = match existing {
        Existing::None => Vec::new(),
        Existing::Items(items) => items,
        Existing::Inline(s) => vec![Object::Reference(doc.add_object(*s))],
    };
    let contents: Vec<Object> = if items.is_empty() {
        vec![Object::Reference(appended_id)]
    } else {
        let mut v = Vec::with_capacity(items.len() + 3);
        v.push(Object::Reference(push_id));
        v.extend(items);
        v.push(Object::Reference(pop_id));
        v.push(Object::Reference(appended_id));
        v
    };
    doc.get_object_mut(page_id)
        .map_err(|e| e.to_string())?
        .as_dict_mut()
        .map_err(|e| e.to_string())?
        .set("Contents", contents);
    Ok(())
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn one_page_doc() -> (Document, ObjectId) {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 400.into()],
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
        (doc, page_id)
    }

    #[test]
    fn content_bytes_are_newline_padded() {
        let bytes = content_bytes(vec![Operation::new("q", vec![])]).unwrap();
        assert_eq!(bytes.first(), Some(&b'\n'));
        assert_eq!(bytes.last(), Some(&b'\n'));
        assert!(bytes.windows(1).any(|w| w == b"q"));
    }

    #[test]
    fn add_resource_entry_creates_missing_dicts_and_returns_key() {
        let (mut doc, page_id) = one_page_doc();
        let font_id = doc.add_object(dictionary! { "Type" => "Font" });
        let key =
            add_resource_entry(&mut doc, page_id, "Font", "F", Object::Reference(font_id)).unwrap();
        assert_eq!(key, "F");
        let page = doc.get_dictionary(page_id).unwrap();
        let res = page.get(b"Resources").unwrap().as_dict().unwrap();
        let font = res.get(b"Font").unwrap().as_dict().unwrap();
        assert!(font.get(b"F").is_ok());
    }

    #[test]
    fn add_resource_entry_avoids_key_collisions() {
        let (mut doc, page_id) = one_page_doc();
        let a = doc.add_object(dictionary! { "Type" => "Font" });
        let b = doc.add_object(dictionary! { "Type" => "Font" });
        let k1 = add_resource_entry(&mut doc, page_id, "Font", "F", Object::Reference(a)).unwrap();
        let k2 = add_resource_entry(&mut doc, page_id, "Font", "F", Object::Reference(b)).unwrap();
        assert_eq!(k1, "F");
        assert_eq!(k2, "F1");
    }

    #[test]
    fn append_to_page_contents_brackets_existing_inline_stream() {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => Object::Stream(Stream::new(dictionary! {}, b"q\n".to_vec())),
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 400.into()],
        });
        let appended_id = doc.add_object(Stream::new(dictionary! {}, b"BT (1) Tj ET\n".to_vec()));
        let (push_id, pop_id) = wrap_stream_ids(&mut doc).unwrap();

        append_to_page_contents(&mut doc, page_id, appended_id, push_id, pop_id).unwrap();

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
        assert_eq!(contents[3].as_reference().unwrap(), appended_id);
    }

    #[test]
    fn append_to_page_contents_handles_missing_contents() {
        let (mut doc, page_id) = one_page_doc();
        let appended_id = doc.add_object(Stream::new(dictionary! {}, b"BT (1) Tj ET\n".to_vec()));
        let (push_id, pop_id) = wrap_stream_ids(&mut doc).unwrap();
        append_to_page_contents(&mut doc, page_id, appended_id, push_id, pop_id).unwrap();
        let contents = doc
            .get_dictionary(page_id)
            .unwrap()
            .get(b"Contents")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].as_reference().unwrap(), appended_id);
    }
}
