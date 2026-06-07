//! Sanitize: strip metadata, scripts, embedded files and other risky
//! structures from a PDF before sharing.
//!
//! Rust port of `sanitizePdf` in `src/lib/tools/security.ts` (pdf-lib), with
//! behavior parity. The pass removes, in order:
//! 1. Info dictionary — the eight standard metadata keys, the Info object
//!    itself and its trailer entry.
//! 2. Catalog keys — `AA`, `OpenAction`, `Metadata`, `PieceInfo`,
//!    `StructTreeRoot`, `SpiderInfo`, `Collection`, `Requirements`.
//! 3. Name trees — `JavaScript`, `EmbeddedFiles`, `Renditions`,
//!    `AlternatePresentations` (plus the `/Names` catalog entry once empty).
//! 4. `/AcroForm` — option `removeForms`, default on.
//! 5. Per page — `AA`/`Metadata`/`PieceInfo`/`PresSteps`, plus either the
//!    whole `/Annots` array (option `removeAnnotations`, default on) or, when
//!    annotations are kept, each annotation's active keys (`A`, `AA`, `JS`,
//!    `FS`, rich-media/sound/movie/rendition payloads).
//! 6. Every indirect object whose `/Type`, `/Subtype` or `/S` marks it as a
//!    dangerous payload (JavaScript, Launch, embedded files, rich media…).
//!
//! Dangerous indirect objects are replaced with `null` rather than dropped, so
//! surviving references can never be re-bound to a different object when the
//! document is renumbered on save; unreferenced nulls are pruned by the
//! standard [`save_compact`] pass.

use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::{Deserialize, Serialize};

use crate::util::save_compact;

/// Standard Info-dictionary metadata keys stripped by sanitize.
const INFO_KEYS: [&[u8]; 8] = [
    b"Title",
    b"Author",
    b"Subject",
    b"Keywords",
    b"Creator",
    b"Producer",
    b"CreationDate",
    b"ModDate",
];

/// Catalog-level keys that carry actions, metadata or hidden structure.
const CATALOG_KEYS: [&[u8]; 8] = [
    b"AA",
    b"OpenAction",
    b"Metadata",
    b"PieceInfo",
    b"StructTreeRoot",
    b"SpiderInfo",
    b"Collection",
    b"Requirements",
];

/// Page-level keys that carry actions or hidden metadata.
const PAGE_KEYS: [&[u8]; 4] = [b"AA", b"Metadata", b"PieceInfo", b"PresSteps"];

/// Keys inside a kept annotation that can trigger actions or embed payloads.
const ANNOTATION_ACTIVE_KEYS: [&[u8]; 9] = [
    b"A",
    b"AA",
    b"JS",
    b"FS",
    b"RichMediaContent",
    b"RichMediaSettings",
    b"Sound",
    b"Movie",
    b"Rendition",
];

/// Document-level name trees that index scripts and attachments.
const NAME_TREE_KEYS: [&[u8]; 4] = [
    b"JavaScript",
    b"EmbeddedFiles",
    b"Renditions",
    b"AlternatePresentations",
];

/// `/Type` or `/Subtype` values that mark an indirect object as dangerous.
const DANGEROUS_TYPES: [&[u8]; 7] = [
    b"EmbeddedFile",
    b"Filespec",
    b"JavaScript",
    b"Rendition",
    b"RichMedia",
    b"Sound",
    b"Movie",
];

/// `/S` action types that execute code, exfiltrate data or open files.
const DANGEROUS_ACTIONS: [&[u8]; 8] = [
    b"JavaScript",
    b"Launch",
    b"SubmitForm",
    b"ImportData",
    b"GoToE",
    b"GoToR",
    b"Rendition",
    b"RichMediaExecute",
];

fn default_true() -> bool {
    true
}

/// Options for [`sanitize_native`], mirroring the `sanitize-pdf` tool options.
///
/// The JS side sends camelCase keys (`removeAnnotations`, `removeForms`);
/// snake_case aliases are also accepted. Both default to `true` when omitted,
/// matching the TypeScript `options.x !== false` checks.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SanitizeOptions {
    /// Delete every page's `/Annots` array outright (default). When `false`,
    /// annotations are kept but their active keys are stripped.
    #[serde(default = "default_true", alias = "remove_annotations")]
    pub remove_annotations: bool,
    /// Delete the document `/AcroForm` (default).
    #[serde(default = "default_true", alias = "remove_forms")]
    pub remove_forms: bool,
}

/// Per-category counts of entries a sanitize pass removes.
#[derive(Debug, Serialize)]
struct SanitizeReport {
    /// Info-dictionary metadata keys plus the Info object itself.
    info_fields: usize,
    /// Catalog keys (`OpenAction`, `AA`, `Metadata`…).
    catalog_keys: usize,
    /// Name-tree entries (`JavaScript`, `EmbeddedFiles`…), plus the emptied
    /// `/Names` entry.
    name_tree_keys: usize,
    /// 1 if an `/AcroForm` was removed.
    acro_form: usize,
    /// Page-level keys (`AA`, `Metadata`, `PieceInfo`, `PresSteps`).
    page_keys: usize,
    /// `/Annots` arrays deleted (or, when keeping annotations, active keys
    /// stripped from individual annotations).
    annotations: usize,
    /// Indirect objects neutralized for a dangerous `/Type`/`/Subtype`/`/S`.
    dangerous_indirect_objects: usize,
    /// Sum of all categories.
    total: usize,
}

/// Strip metadata, document scripts, embedded files, page actions and
/// (optionally) annotations/forms from a PDF.
///
/// `opts_json` is a JSON object matching [`SanitizeOptions`]; pass `"{}"` for
/// the defaults (remove both annotations and forms). Returns the rebuilt PDF.
pub fn sanitize_native(data: &[u8], opts_json: &str) -> Result<Vec<u8>, String> {
    let opts: SanitizeOptions =
        serde_json::from_str(opts_json).map_err(|e| format!("Invalid sanitize options: {e}"))?;
    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    apply_sanitize(&mut doc, opts.remove_annotations, opts.remove_forms)?;
    save_compact(doc).map_err(|e| e.to_string())
}

/// JSON report of what a default sanitize pass (annotations and forms both
/// removed) WOULD strip from this PDF, without producing output bytes.
///
/// Shape: `{"info_fields", "catalog_keys", "name_tree_keys", "acro_form",
/// "page_keys", "annotations", "dangerous_indirect_objects", "total"}`.
pub fn sanitize_report_native(data: &[u8]) -> Result<String, String> {
    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    let report = apply_sanitize(&mut doc, true, true)?;
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

/// Run the full sanitize pass on a loaded document, tallying removals.
fn apply_sanitize(
    doc: &mut Document,
    remove_annotations: bool,
    remove_forms: bool,
) -> Result<SanitizeReport, String> {
    let info_fields = delete_info_dict(doc);

    let catalog = doc.catalog_mut().map_err(|e| e.to_string())?;
    let catalog_keys = delete_keys(catalog, &CATALOG_KEYS);

    let name_tree_keys = clean_name_tree(doc)?;

    let acro_form = if remove_forms {
        let catalog = doc.catalog_mut().map_err(|e| e.to_string())?;
        usize::from(catalog.remove(b"AcroForm").is_some())
    } else {
        0
    };

    let mut page_keys = 0;
    let mut annotations = 0;
    let page_ids: Vec<ObjectId> = doc.page_iter().collect();
    for page_id in page_ids {
        if let Ok(dict) = doc.get_object_mut(page_id).and_then(Object::as_dict_mut) {
            page_keys += delete_keys(dict, &PAGE_KEYS);
            if remove_annotations {
                annotations += usize::from(dict.remove(b"Annots").is_some());
            }
        }
        if !remove_annotations {
            annotations += clean_page_annotations(doc, page_id);
        }
    }

    let dangerous_indirect_objects = null_dangerous_objects(doc);

    let total = info_fields
        + catalog_keys
        + name_tree_keys
        + acro_form
        + page_keys
        + annotations
        + dangerous_indirect_objects;
    Ok(SanitizeReport {
        info_fields,
        catalog_keys,
        name_tree_keys,
        acro_form,
        page_keys,
        annotations,
        dangerous_indirect_objects,
        total,
    })
}

/// Remove each key present in `dict`, returning how many existed.
fn delete_keys(dict: &mut Dictionary, keys: &[&[u8]]) -> usize {
    keys.iter().filter(|key| dict.remove(key).is_some()).count()
}

/// Follow a reference chain to the id of a dictionary object, if any.
fn resolve_dict_id(doc: &Document, mut id: ObjectId) -> Option<ObjectId> {
    for _ in 0..16 {
        match doc.objects.get(&id) {
            Some(Object::Reference(next)) => id = *next,
            Some(Object::Dictionary(_)) => return Some(id),
            _ => return None,
        }
    }
    None
}

/// Strip the standard metadata keys from the Info dictionary, drop the Info
/// object and clear the trailer entry. Counts each removed key plus the
/// removed indirect Info object itself.
fn delete_info_dict(doc: &mut Document) -> usize {
    let mut removed = 0;
    match doc.trailer.get(b"Info").ok().cloned() {
        Some(Object::Reference(id)) => {
            if let Some(dict_id) = resolve_dict_id(doc, id) {
                if let Ok(dict) = doc.get_object_mut(dict_id).and_then(Object::as_dict_mut) {
                    removed += delete_keys(dict, &INFO_KEYS);
                }
            }
            if doc.objects.remove(&id).is_some() {
                removed += 1;
            }
            doc.trailer.remove(b"Info");
        }
        Some(Object::Dictionary(_)) => {
            if let Ok(Object::Dictionary(dict)) = doc.trailer.get_mut(b"Info") {
                removed += delete_keys(dict, &INFO_KEYS);
            }
            doc.trailer.remove(b"Info");
        }
        Some(_) => {
            doc.trailer.remove(b"Info");
        }
        None => {}
    }
    removed
}

/// Remove the script/attachment name trees from the catalog `/Names`
/// dictionary; if that empties it, remove the `/Names` entry too.
fn clean_name_tree(doc: &mut Document) -> Result<usize, String> {
    let names_value = match doc
        .catalog()
        .ok()
        .and_then(|c| c.get(b"Names").ok())
        .cloned()
    {
        Some(value) => value,
        None => return Ok(0),
    };

    let mut removed = 0;
    let now_empty = match names_value {
        Object::Dictionary(_) => {
            let catalog = doc.catalog_mut().map_err(|e| e.to_string())?;
            match catalog.get_mut(b"Names") {
                Ok(Object::Dictionary(dict)) => {
                    removed += delete_keys(dict, &NAME_TREE_KEYS);
                    dict.is_empty()
                }
                _ => return Ok(0),
            }
        }
        Object::Reference(id) => match resolve_dict_id(doc, id) {
            Some(dict_id) => {
                let dict = doc
                    .get_object_mut(dict_id)
                    .and_then(Object::as_dict_mut)
                    .map_err(|e| e.to_string())?;
                removed += delete_keys(dict, &NAME_TREE_KEYS);
                dict.is_empty()
            }
            None => return Ok(0),
        },
        _ => return Ok(0),
    };

    if now_empty {
        let catalog = doc.catalog_mut().map_err(|e| e.to_string())?;
        if catalog.remove(b"Names").is_some() {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Strip active keys from every annotation on a page that is being kept.
/// Handles `/Annots` as a direct array or behind references, and annotation
/// entries as direct dictionaries or references.
fn clean_page_annotations(doc: &mut Document, page_id: ObjectId) -> usize {
    enum ArrayLocation {
        InPageDict,
        Indirect(ObjectId),
    }

    let annots_value = match doc
        .get_dictionary(page_id)
        .ok()
        .and_then(|d| d.get(b"Annots").ok())
        .cloned()
    {
        Some(value) => value,
        None => return 0,
    };

    let location = match annots_value {
        Object::Array(_) => ArrayLocation::InPageDict,
        Object::Reference(mut id) => {
            let mut found = None;
            for _ in 0..16 {
                match doc.objects.get(&id) {
                    Some(Object::Reference(next)) => id = *next,
                    Some(Object::Array(_)) => {
                        found = Some(id);
                        break;
                    }
                    _ => break,
                }
            }
            match found {
                Some(id) => ArrayLocation::Indirect(id),
                None => return 0,
            }
        }
        _ => return 0,
    };

    let mut removed = 0;
    let mut referenced = Vec::new();
    let array = match location {
        ArrayLocation::InPageDict => doc
            .get_object_mut(page_id)
            .ok()
            .and_then(|o| o.as_dict_mut().ok())
            .and_then(|d| d.get_mut(b"Annots").ok())
            .and_then(|o| o.as_array_mut().ok()),
        ArrayLocation::Indirect(id) => doc
            .get_object_mut(id)
            .ok()
            .and_then(|o| o.as_array_mut().ok()),
    };
    if let Some(array) = array {
        for entry in array.iter_mut() {
            match entry {
                Object::Dictionary(dict) => removed += delete_keys(dict, &ANNOTATION_ACTIVE_KEYS),
                Object::Reference(id) => referenced.push(*id),
                _ => {}
            }
        }
    }
    for id in referenced {
        if let Some(dict_id) = resolve_dict_id(doc, id) {
            if let Ok(dict) = doc.get_object_mut(dict_id).and_then(Object::as_dict_mut) {
                removed += delete_keys(dict, &ANNOTATION_ACTIVE_KEYS);
            }
        }
    }
    removed
}

/// Replace every indirect object with a dangerous `/Type`, `/Subtype` or `/S`
/// with `null`. Returns the number of neutralized objects.
fn null_dangerous_objects(doc: &mut Document) -> usize {
    let mut doomed = Vec::new();
    for (&id, object) in &doc.objects {
        let dict = match object {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            _ => continue,
        };
        let name_in = |key: &[u8], list: &[&[u8]]| {
            dict.get(key)
                .ok()
                .and_then(|o| o.as_name().ok())
                .is_some_and(|name| list.contains(&name))
        };
        if name_in(b"Type", &DANGEROUS_TYPES)
            || name_in(b"Subtype", &DANGEROUS_TYPES)
            || name_in(b"S", &DANGEROUS_ACTIONS)
        {
            doomed.push(id);
        }
    }
    let count = doomed.len();
    for id in doomed {
        if let Some(slot) = doc.objects.get_mut(&id) {
            *slot = Object::Null;
        }
    }
    count
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fixtures::{page_content_text, page_width, sample, sample_with_text};
    use lopdf::{dictionary, Document, Object, Stream};

    fn load(data: &[u8]) -> Document {
        Document::load_mem(data).unwrap()
    }

    fn bytes_of(mut doc: Document) -> Vec<u8> {
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    fn report(data: &[u8]) -> serde_json::Value {
        serde_json::from_str(&sanitize_report_native(data).unwrap()).unwrap()
    }

    fn catalog_of(data: &[u8]) -> Dictionary {
        load(data).catalog().unwrap().clone()
    }

    fn first_page_dict(data: &[u8]) -> Dictionary {
        let doc = load(data);
        let id = doc.page_iter().next().unwrap();
        doc.get_dictionary(id).unwrap().clone()
    }

    /// Does any indirect object (dict or stream) in `data` satisfy `pred`?
    fn any_dict_matches(data: &[u8], pred: impl Fn(&Dictionary) -> bool) -> bool {
        let doc = load(data);
        doc.objects.values().any(|object| match object {
            Object::Dictionary(dict) => pred(dict),
            Object::Stream(stream) => pred(&stream.dict),
            _ => false,
        })
    }

    fn has_name(dict: &Dictionary, key: &[u8], value: &[u8]) -> bool {
        dict.get(key).ok().and_then(|o| o.as_name().ok()) == Some(value)
    }

    /// A 2-page PDF salted with every category of risky structure:
    /// Info metadata, OpenAction + catalog AA, JavaScript/EmbeddedFiles name
    /// trees, an AcroForm, a Launch-action link annotation and a page AA.
    fn risky_pdf() -> Vec<u8> {
        let mut doc = load(&sample_with_text(2, Some("Body")));

        let info_id = doc.add_object(dictionary! {
            "Title" => Object::string_literal("secret title"),
            "Author" => Object::string_literal("secret author"),
            "Producer" => Object::string_literal("test producer"),
        });
        doc.trailer.set("Info", info_id);

        let js_action = doc.add_object(dictionary! {
            "S" => "JavaScript",
            "JS" => Object::string_literal("app.alert(1)"),
        });
        let embedded = doc.add_object(Stream::new(
            dictionary! { "Type" => "EmbeddedFile" },
            b"payload".to_vec(),
        ));
        let filespec = doc.add_object(dictionary! {
            "Type" => "Filespec",
            "F" => Object::string_literal("evil.txt"),
            "EF" => dictionary! { "F" => embedded },
        });
        let js_tree = doc.add_object(dictionary! {
            "Names" => vec![Object::string_literal("boot"), js_action.into()],
        });
        let ef_tree = doc.add_object(dictionary! {
            "Names" => vec![Object::string_literal("evil.txt"), filespec.into()],
        });
        let names_id = doc.add_object(dictionary! {
            "JavaScript" => js_tree,
            "EmbeddedFiles" => ef_tree,
        });
        let launch = doc.add_object(dictionary! {
            "S" => "Launch",
            "F" => Object::string_literal("cmd.exe"),
        });
        let annot = doc.add_object(dictionary! {
            "Type" => "Annot",
            "Subtype" => "Link",
            "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()],
            "A" => launch,
        });
        let acro_form = doc.add_object(dictionary! { "Fields" => Object::Array(vec![]) });

        let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let catalog = doc
            .get_object_mut(catalog_id)
            .unwrap()
            .as_dict_mut()
            .unwrap();
        catalog.set("Names", names_id);
        catalog.set("OpenAction", js_action);
        catalog.set("AA", dictionary! { "WC" => js_action });
        catalog.set("AcroForm", acro_form);

        let pages: Vec<_> = doc.page_iter().collect();
        let page0 = doc.get_object_mut(pages[0]).unwrap().as_dict_mut().unwrap();
        page0.set("Annots", vec![Object::Reference(annot)]);
        page0.set("AA", dictionary! { "O" => js_action });

        bytes_of(doc)
    }

    // ----- happy path -------------------------------------------------------

    #[test]
    fn defaults_keep_document_valid_and_pages() {
        let out = sanitize_native(&sample(3), "{}").unwrap();
        assert!(out.starts_with(b"%PDF-"));
        let doc = load(&out);
        assert_eq!(doc.get_pages().len(), 3);
    }

    #[test]
    fn preserves_page_content_size_and_resources() {
        let out = sanitize_native(&risky_pdf(), "{}").unwrap();
        assert_eq!(load(&out).get_pages().len(), 2);
        assert!(page_content_text(&out, 0).contains("Body 0"));
        assert!(page_content_text(&out, 1).contains("Body 1"));
        assert_eq!(page_width(&out, 0), 300.0);
        assert_eq!(page_width(&out, 1), 301.0);
        assert!(first_page_dict(&out).has(b"Resources"));
    }

    // ----- info dictionary --------------------------------------------------

    #[test]
    fn removes_info_dict_and_trailer_entry() {
        let out = sanitize_native(&risky_pdf(), "{}").unwrap();
        let doc = load(&out);
        assert!(doc.trailer.get(b"Info").is_err());
        assert!(!any_dict_matches(&out, |d| d.has(b"Title") || d.has(b"Author")));
    }

    #[test]
    fn strips_info_keys_from_direct_trailer_dict() {
        let mut doc = load(&sample(1));
        doc.trailer.set(
            "Info",
            dictionary! { "Title" => Object::string_literal("inline secret") },
        );
        let out = sanitize_native(&bytes_of(doc), "{}").unwrap();
        assert!(load(&out).trailer.get(b"Info").is_err());
    }

    #[test]
    fn removes_malformed_info_trailer_entry() {
        let mut doc = load(&sample(1));
        doc.trailer.set("Info", Object::Integer(42));

        let out = sanitize_native(&bytes_of(doc), "{}").unwrap();
        assert!(load(&out).trailer.get(b"Info").is_err());
    }

    // ----- catalog ------------------------------------------------------------

    #[test]
    fn removes_all_eight_catalog_keys() {
        let mut doc = load(&sample(1));
        let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let catalog = doc
            .get_object_mut(catalog_id)
            .unwrap()
            .as_dict_mut()
            .unwrap();
        for key in CATALOG_KEYS {
            catalog.set(key.to_vec(), dictionary! {});
        }
        let out = sanitize_native(&bytes_of(doc), "{}").unwrap();
        let catalog = catalog_of(&out);
        for key in CATALOG_KEYS {
            assert!(
                !catalog.has(key),
                "catalog still has {}",
                String::from_utf8_lossy(key)
            );
        }
        assert!(catalog.has(b"Pages")); // untouched structure survives
    }

    // ----- name trees ---------------------------------------------------------

    #[test]
    fn removes_name_tree_entirely_when_emptied() {
        let out = sanitize_native(&risky_pdf(), "{}").unwrap();
        assert!(!catalog_of(&out).has(b"Names"));
    }

    #[test]
    fn keeps_name_tree_with_other_entries() {
        let mut doc = load(&sample(1));
        let js_action = doc.add_object(dictionary! {
            "S" => "JavaScript",
            "JS" => Object::string_literal("app.alert(2)"),
        });
        let js_tree = doc.add_object(dictionary! {
            "Names" => vec![Object::string_literal("boot"), js_action.into()],
        });
        let names_id = doc.add_object(dictionary! {
            "JavaScript" => js_tree,
            "Dests" => dictionary! {},
        });
        let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let catalog = doc
            .get_object_mut(catalog_id)
            .unwrap()
            .as_dict_mut()
            .unwrap();
        catalog.set("Names", names_id);

        let out = sanitize_native(&bytes_of(doc), "{}").unwrap();
        let doc = load(&out);
        let names_id = doc
            .catalog()
            .unwrap()
            .get(b"Names")
            .unwrap()
            .as_reference()
            .unwrap();
        let names = doc.get_dictionary(names_id).unwrap();
        assert!(names.has(b"Dests"));
        assert!(!names.has(b"JavaScript"));
    }

    #[test]
    fn cleans_direct_name_tree_dictionary() {
        let mut doc = load(&sample(1));
        let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let catalog = doc
            .get_object_mut(catalog_id)
            .unwrap()
            .as_dict_mut()
            .unwrap();
        catalog.set(
            "Names",
            dictionary! {
                "JavaScript" => dictionary! {},
                "Dests" => dictionary! {},
            },
        );

        let out = sanitize_native(&bytes_of(doc), "{}").unwrap();
        let names = catalog_of(&out)
            .get(b"Names")
            .unwrap()
            .as_dict()
            .unwrap()
            .clone();
        assert!(names.has(b"Dests"));
        assert!(!names.has(b"JavaScript"));
    }

    // ----- forms ----------------------------------------------------------------

    #[test]
    fn removes_acroform_by_default() {
        let out = sanitize_native(&risky_pdf(), "{}").unwrap();
        assert!(!catalog_of(&out).has(b"AcroForm"));
    }

    #[test]
    fn keeps_acroform_when_remove_forms_false() {
        let out = sanitize_native(&risky_pdf(), r#"{"removeForms": false}"#).unwrap();
        let doc = load(&out);
        let form_id = doc
            .catalog()
            .unwrap()
            .get(b"AcroForm")
            .unwrap()
            .as_reference()
            .unwrap();
        assert!(doc.get_dictionary(form_id).unwrap().has(b"Fields"));
    }

    // ----- annotations ----------------------------------------------------------

    #[test]
    fn removes_annots_array_by_default() {
        let out = sanitize_native(&risky_pdf(), "{}").unwrap();
        assert!(!first_page_dict(&out).has(b"Annots"));
    }

    #[test]
    fn cleans_referenced_annotations_when_kept() {
        let out = sanitize_native(&risky_pdf(), r#"{"removeAnnotations": false}"#).unwrap();
        let doc = load(&out);
        let page = doc.page_iter().next().unwrap();
        let annots = doc
            .get_dictionary(page)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(annots.len(), 1);
        let annot = doc
            .get_dictionary(annots[0].as_reference().unwrap())
            .unwrap();
        assert!(has_name(annot, b"Subtype", b"Link")); // annotation survives
        assert!(!annot.has(b"A")); // its action does not
                                   // ...and the Launch action object is gone from the document.
        assert!(!any_dict_matches(&out, |d| has_name(d, b"S", b"Launch")));
    }

    #[test]
    fn cleans_direct_annotation_dicts_when_kept() {
        let mut doc = load(&sample(1));
        let page = doc.page_iter().next().unwrap();
        let page_dict = doc.get_object_mut(page).unwrap().as_dict_mut().unwrap();
        page_dict.set(
            "Annots",
            vec![Object::Dictionary(dictionary! {
                "Subtype" => "Text",
                "JS" => Object::string_literal("app.alert(3)"),
                "AA" => dictionary! {},
            })],
        );
        let out = sanitize_native(&bytes_of(doc), r#"{"removeAnnotations": false}"#).unwrap();
        let annots = first_page_dict(&out)
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(annots.len(), 1);
        let annot = annots[0].as_dict().unwrap();
        assert!(has_name(annot, b"Subtype", b"Text"));
        assert!(!annot.has(b"JS"));
        assert!(!annot.has(b"AA"));
    }

    #[test]
    fn handles_annots_behind_indirect_array_reference() {
        let build = || {
            let mut doc = load(&sample(1));
            let annot = doc.add_object(dictionary! {
                "Subtype" => "Link",
                "A" => dictionary! { "S" => "URI", "URI" => Object::string_literal("https://x") },
            });
            let array_id = doc.add_object(vec![Object::Reference(annot)]);
            let page = doc.page_iter().next().unwrap();
            let page_dict = doc.get_object_mut(page).unwrap().as_dict_mut().unwrap();
            page_dict.set("Annots", array_id);
            bytes_of(doc)
        };

        // Kept: cleaned through the reference.
        let out = sanitize_native(&build(), r#"{"removeAnnotations": false}"#).unwrap();
        let doc = load(&out);
        let page = doc.page_iter().next().unwrap();
        let annots_obj = doc
            .get_dictionary(page)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .clone();
        let array = match annots_obj {
            Object::Reference(id) => doc.get_object(id).unwrap().as_array().unwrap().clone(),
            Object::Array(a) => a,
            other => panic!("unexpected Annots value: {other:?}"),
        };
        let annot = doc
            .get_dictionary(array[0].as_reference().unwrap())
            .unwrap();
        assert!(!annot.has(b"A"));

        // Removed: the key disappears from the page.
        let out = sanitize_native(&build(), "{}").unwrap();
        assert!(!first_page_dict(&out).has(b"Annots"));
    }

    #[test]
    fn cleans_annots_behind_reference_chain_when_kept() {
        let mut doc = load(&sample(1));
        let annot = doc.add_object(dictionary! {
            "Subtype" => "Link",
            "A" => dictionary! { "S" => "URI", "URI" => Object::string_literal("https://x") },
        });
        let array_id = doc.add_object(vec![Object::Reference(annot)]);
        let alias_id = doc.add_object(Object::Reference(array_id));
        let page = doc.page_iter().next().unwrap();
        doc.get_object_mut(page)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Annots", alias_id);

        let out = sanitize_native(&bytes_of(doc), r#"{"removeAnnotations": false}"#).unwrap();
        let doc = load(&out);
        let page = doc.page_iter().next().unwrap();
        let annots_id = doc
            .get_dictionary(page)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_reference()
            .unwrap();
        let annots = doc.get_object(annots_id).unwrap().as_array().unwrap();
        let annot = doc
            .get_dictionary(annots[0].as_reference().unwrap())
            .unwrap();
        assert!(!annot.has(b"A"));
    }

    // ----- page keys ------------------------------------------------------------

    #[test]
    fn removes_all_four_page_keys() {
        let mut doc = load(&sample(1));
        let page = doc.page_iter().next().unwrap();
        let page_dict = doc.get_object_mut(page).unwrap().as_dict_mut().unwrap();
        for key in PAGE_KEYS {
            page_dict.set(key.to_vec(), dictionary! {});
        }
        let out = sanitize_native(&bytes_of(doc), "{}").unwrap();
        let page = first_page_dict(&out);
        for key in PAGE_KEYS {
            assert!(
                !page.has(key),
                "page still has {}",
                String::from_utf8_lossy(key)
            );
        }
        assert!(page.has(b"MediaBox"));
    }

    // ----- dangerous indirect objects --------------------------------------------

    #[test]
    fn purges_dangerous_objects_from_output() {
        let out = sanitize_native(&risky_pdf(), "{}").unwrap();
        assert!(!any_dict_matches(&out, |d| has_name(
            d,
            b"S",
            b"JavaScript"
        )));
        assert!(!any_dict_matches(&out, |d| has_name(d, b"S", b"Launch")));
        assert!(!any_dict_matches(&out, |d| has_name(
            d,
            b"Type",
            b"EmbeddedFile"
        )));
        assert!(!any_dict_matches(&out, |d| has_name(
            d,
            b"Type",
            b"Filespec"
        )));
    }

    #[test]
    fn nulls_dangerous_objects_that_stay_referenced() {
        let mut doc = load(&sample(1));
        let filespec = doc.add_object(dictionary! {
            "Type" => "Filespec",
            "F" => Object::string_literal("evil.txt"),
        });
        let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let catalog = doc
            .get_object_mut(catalog_id)
            .unwrap()
            .as_dict_mut()
            .unwrap();
        catalog.set("Unfleece", filespec); // custom key: not stripped by sanitize

        let out = sanitize_native(&bytes_of(doc), "{}").unwrap();
        let doc = load(&out);
        let id = doc
            .catalog()
            .unwrap()
            .get(b"Unfleece")
            .unwrap()
            .as_reference()
            .unwrap();
        assert!(matches!(doc.get_object(id).unwrap(), Object::Null));
    }

    #[test]
    fn nulls_dangerous_annotation_subtype_even_when_annotations_kept() {
        let mut doc = load(&sample(1));
        let movie = doc.add_object(dictionary! {
            "Subtype" => "Movie",
            "Rect" => vec![0.into(), 0.into(), 5.into(), 5.into()],
        });
        let page = doc.page_iter().next().unwrap();
        let page_dict = doc.get_object_mut(page).unwrap().as_dict_mut().unwrap();
        page_dict.set("Annots", vec![Object::Reference(movie)]);

        let out = sanitize_native(&bytes_of(doc), r#"{"removeAnnotations": false}"#).unwrap();
        let doc = load(&out);
        let page = doc.page_iter().next().unwrap();
        let annots = doc
            .get_dictionary(page)
            .unwrap()
            .get(b"Annots")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(annots.len(), 1);
        let annot = doc.get_object(annots[0].as_reference().unwrap()).unwrap();
        assert!(matches!(annot, Object::Null));
    }

    #[test]
    fn purges_dangerous_stream_subtypes() {
        let mut doc = load(&sample(1));
        let js_stream = doc.add_object(Stream::new(
            dictionary! { "Subtype" => "JavaScript" },
            b"app.alert(4)".to_vec(),
        ));
        let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let catalog = doc
            .get_object_mut(catalog_id)
            .unwrap()
            .as_dict_mut()
            .unwrap();
        catalog.set("Unfleece", js_stream);

        let out = sanitize_native(&bytes_of(doc), "{}").unwrap();
        assert!(!any_dict_matches(&out, |d| has_name(
            d,
            b"Subtype",
            b"JavaScript"
        )));
        assert!(!out
            .windows(b"app.alert(4)".len())
            .any(|w| w == b"app.alert(4)"));
    }

    // ----- options parsing --------------------------------------------------------

    #[test]
    fn accepts_snake_case_aliases_and_unknown_fields() {
        let out = sanitize_native(
            &risky_pdf(),
            r#"{"remove_forms": false, "future_option": 1}"#,
        )
        .unwrap();
        assert!(catalog_of(&out).has(b"AcroForm")); // snake_case alias honored
        assert!(!first_page_dict(&out).has(b"Annots")); // unset option defaulted to true
    }

    #[test]
    fn rejects_invalid_options_json_with_friendly_error() {
        let err = sanitize_native(&sample(1), "not json").unwrap_err();
        assert!(err.contains("Invalid sanitize options"), "got: {err}");
    }

    // ----- bad input -----------------------------------------------------------------

    #[test]
    fn errors_on_empty_and_garbage_input() {
        assert!(sanitize_native(&[], "{}").is_err());
        assert!(sanitize_native(b"not a pdf at all", "{}").is_err());
        assert!(sanitize_report_native(&[]).is_err());
    }

    // ----- report ----------------------------------------------------------------------

    #[test]
    fn report_is_all_zero_for_clean_pdf() {
        let report = report(&sample(2));
        for field in [
            "info_fields",
            "catalog_keys",
            "name_tree_keys",
            "acro_form",
            "page_keys",
            "annotations",
            "dangerous_indirect_objects",
            "total",
        ] {
            assert_eq!(report[field], 0, "expected {field} == 0, got {report}");
        }
    }

    #[test]
    fn report_counts_risky_entries_exactly() {
        let report = report(&risky_pdf());
        assert_eq!(report["info_fields"], 4); // Title + Author + Producer + Info object
        assert_eq!(report["catalog_keys"], 2); // OpenAction + AA
        assert_eq!(report["name_tree_keys"], 3); // JavaScript + EmbeddedFiles + emptied Names
        assert_eq!(report["acro_form"], 1);
        assert_eq!(report["page_keys"], 1); // page 0 AA
        assert_eq!(report["annotations"], 1); // page 0 Annots array
        assert_eq!(report["dangerous_indirect_objects"], 4); // js action, embedded file, filespec, launch
        assert_eq!(report["total"], 16);
    }

    #[test]
    fn sanitize_is_idempotent_per_report() {
        // A default sanitize pass leaves nothing for a second pass to find.
        let out = sanitize_native(&risky_pdf(), "{}").unwrap();
        assert_eq!(report(&out)["total"], 0);
    }

    #[test]
    fn removes_annots_from_every_page() {
        let mut doc = load(&sample(3));
        let pages: Vec<_> = doc.page_iter().collect();
        for &page in &pages {
            let annot = doc.add_object(dictionary! {
                "Subtype" => "Link",
                "A" => dictionary! { "S" => "URI", "URI" => Object::string_literal("https://x") },
            });
            let page_dict = doc.get_object_mut(page).unwrap().as_dict_mut().unwrap();
            page_dict.set("Annots", vec![Object::Reference(annot)]);
        }
        let data = bytes_of(doc);

        assert_eq!(report(&data)["annotations"], 3);
        let out = sanitize_native(&data, "{}").unwrap();
        let doc = load(&out);
        for page in doc.page_iter() {
            assert!(!doc.get_dictionary(page).unwrap().has(b"Annots"));
        }
    }
}
