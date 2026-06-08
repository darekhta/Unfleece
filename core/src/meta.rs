//! Document-information metadata: read / set / strip the trailer `/Info`
//! dictionary (and, for stripping, the catalog's XMP `/Metadata` stream).
//!
//! Behavior mirrors `src/lib/tools/metadata.ts` (pdf-lib based):
//! - `read` returns the Info fields as JSON; `keywords` is split on `[,;]\s*`
//!   and empty segments are dropped (an empty keywords string yields no field,
//!   matching pdf-lib's falsy-string check).
//! - `set` only touches the fields present in the options JSON; an empty
//!   string clears a field. A keywords array is stored as one comma-separated
//!   string (`join(", ")`) so the delimiter survives a read round-trip.
//! - `strip` removes the whole Info dictionary AND the catalog XMP metadata
//!   stream — stronger than the TS version (which only blanked Info values)
//!   for real privacy hygiene.
//!
//! Dates are passed through verbatim as PDF date strings (`D:YYYYMMDD…`);
//! nothing is parsed or timestamped (wasm32 has no clock).

use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};
use serde::{Deserialize, Serialize};

use crate::util::save_compact;

/// JSON shape returned by [`read_metadata_native`]. Absent fields are omitted
/// (matching `undefined` in the TypeScript `PdfMetadata`).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PdfMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    creator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    producer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    creation_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modification_date: Option<String>,
    page_count: usize,
}

/// Options JSON accepted by [`set_metadata_native`]. Every field is optional;
/// only present fields are applied. Unknown fields are ignored.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetadataUpdate {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    keywords: Option<KeywordsInput>,
    #[serde(default)]
    creator: Option<String>,
    #[serde(default)]
    producer: Option<String>,
    #[serde(default)]
    creation_date: Option<String>,
    #[serde(default)]
    modification_date: Option<String>,
}

/// Keywords may arrive as an array (the TS `MetadataUpdate` shape) or as the
/// raw comma-separated text the UI option produces; both normalize to one
/// `", "`-joined string.
#[derive(Deserialize)]
#[serde(untagged)]
enum KeywordsInput {
    List(Vec<String>),
    Text(String),
}

impl KeywordsInput {
    fn joined(&self) -> String {
        match self {
            KeywordsInput::List(items) => items.join(", "),
            // Mirror the worker shim: split(','), trim, drop empties, re-join.
            KeywordsInput::Text(text) => text
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}

/// Read document metadata as a JSON string:
/// `{title?, author?, subject?, keywords?: string[], creator?, producer?,
/// creationDate?, modificationDate?, pageCount}`.
///
/// Fields missing from the Info dictionary are omitted. Dates are returned
/// verbatim as PDF date strings (e.g. `D:20240102030405Z`).
pub fn read_metadata_native(data: &[u8]) -> Result<String, String> {
    let doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    let info = info_dict(&doc);
    let entry = |key: &[u8]| info.and_then(|dict| string_entry(dict, key));

    let meta = PdfMetadata {
        title: entry(b"Title"),
        author: entry(b"Author"),
        subject: entry(b"Subject"),
        // pdf-lib treats an empty keywords string as "no keywords" (falsy),
        // so only a non-empty string produces a (possibly empty) array.
        keywords: entry(b"Keywords")
            .filter(|joined| !joined.is_empty())
            .map(|joined| split_keywords(&joined)),
        creator: entry(b"Creator"),
        producer: entry(b"Producer"),
        creation_date: entry(b"CreationDate"),
        modification_date: entry(b"ModDate"),
        page_count: doc.get_pages().len(),
    };
    serde_json::to_string(&meta).map_err(|e| e.to_string())
}

/// Overwrite the metadata fields present in `json` (see [`MetadataUpdate`]).
///
/// Semantics match the TS `setMetadata`: absent fields are left untouched, an
/// empty string clears a field, and a keywords array is stored as a single
/// `", "`-joined string. The Info dictionary is created if missing.
pub fn set_metadata_native(data: &[u8], json: &str) -> Result<Vec<u8>, String> {
    let update: MetadataUpdate =
        serde_json::from_str(json).map_err(|e| format!("Invalid metadata options: {e}"))?;
    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;

    let info_id = ensure_info_id(&mut doc);
    let joined_keywords = update.keywords.as_ref().map(KeywordsInput::joined);

    let info = doc
        .get_object_mut(info_id)
        .and_then(Object::as_dict_mut)
        .map_err(|e| e.to_string())?;
    apply_field(info, b"Title", update.title.as_deref());
    apply_field(info, b"Author", update.author.as_deref());
    apply_field(info, b"Subject", update.subject.as_deref());
    apply_field(info, b"Keywords", joined_keywords.as_deref());
    apply_field(info, b"Creator", update.creator.as_deref());
    apply_field(info, b"Producer", update.producer.as_deref());
    apply_field(info, b"CreationDate", update.creation_date.as_deref());
    apply_field(info, b"ModDate", update.modification_date.as_deref());
    let info_now_empty = info.is_empty();

    if info_now_empty {
        // Nothing left to say: drop the trailer entry so the (empty) Info
        // object gets pruned by save_compact.
        doc.trailer.remove(b"Info");
    }
    save_compact(doc).map_err(|e| e.to_string())
}

/// Strip all document metadata (privacy hygiene): remove the trailer `/Info`
/// dictionary entirely AND the catalog's XMP `/Metadata` stream.
pub fn strip_metadata_native(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;

    // Document-information dictionary.
    if let Some(info) = doc.trailer.remove(b"Info") {
        if let Ok(id) = info.as_reference() {
            doc.objects.remove(&id);
        }
    }

    // Catalog XMP metadata stream.
    let xmp_id = doc
        .catalog()
        .ok()
        .and_then(|catalog| catalog.get(b"Metadata").ok())
        .and_then(|obj| obj.as_reference().ok());
    if let Ok(catalog) = doc.catalog_mut() {
        catalog.remove(b"Metadata");
    }
    if let Some(id) = xmp_id {
        doc.objects.remove(&id);
    }

    save_compact(doc).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// The trailer's Info dictionary, if present and well-formed.
fn info_dict(doc: &Document) -> Option<&Dictionary> {
    match doc.trailer.get(b"Info").ok()? {
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        Object::Dictionary(dict) => Some(dict),
        _ => None,
    }
}

/// The Info object's id, creating an empty Info dictionary if absent/broken.
fn ensure_info_id(doc: &mut Document) -> ObjectId {
    match doc.trailer.get(b"Info").ok().cloned() {
        Some(Object::Reference(id)) if matches!(doc.get_object(id), Ok(Object::Dictionary(_))) => {
            id
        }
        // Inline trailer dictionaries are legal but awkward; materialize them.
        Some(Object::Dictionary(dict)) => {
            let id = doc.add_object(dict);
            doc.trailer.set("Info", id);
            id
        }
        _ => {
            let id = doc.add_object(Dictionary::new());
            doc.trailer.set("Info", id);
            id
        }
    }
}

/// Apply one optional text field: `None` = leave untouched, `""` = clear,
/// anything else = overwrite.
fn apply_field(info: &mut Dictionary, key: &[u8], value: Option<&str>) {
    match value {
        None => {}
        Some("") => {
            info.remove(key);
        }
        Some(text) => info.set(key.to_vec(), encode_pdf_string(text)),
    }
}

/// Decode a string entry of `dict`, or `None` if absent / not a string.
fn string_entry(dict: &Dictionary, key: &[u8]) -> Option<String> {
    match dict.get(key).ok()? {
        Object::String(bytes, _) => Some(decode_pdf_string(bytes)),
        _ => None,
    }
}

/// Encode text for the Info dictionary: ASCII goes out as a literal string
/// (lopdf escapes `\`, `(`, `)` on save); anything else becomes a UTF-16BE
/// hex string with BOM — the same strategy pdf-lib uses.
fn encode_pdf_string(text: &str) -> Object {
    if text.is_ascii() {
        Object::String(text.as_bytes().to_vec(), StringFormat::Literal)
    } else {
        let mut bytes = vec![0xFE, 0xFF];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }
        Object::String(bytes, StringFormat::Hexadecimal)
    }
}

/// Decode a PDF text string: UTF-16BE with BOM, UTF-8 with BOM (PDF 2.0), or
/// PDFDocEncoding (single-byte) otherwise.
fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&bytes[3..]).into_owned()
    } else {
        bytes.iter().map(|&b| pdf_doc_byte_to_char(b)).collect()
    }
}

/// PDFDocEncoding → Unicode (PDF spec Annex D). Bytes outside the table map
/// like Latin-1, which is identical for the ASCII and 0xA1–0xFF ranges.
fn pdf_doc_byte_to_char(byte: u8) -> char {
    match byte {
        0x18 => '\u{02D8}', // breve
        0x19 => '\u{02C7}', // caron
        0x1A => '\u{02C6}', // circumflex
        0x1B => '\u{02D9}', // dot accent
        0x1C => '\u{02DD}', // double acute
        0x1D => '\u{02DB}', // ogonek
        0x1E => '\u{02DA}', // ring
        0x1F => '\u{02DC}', // small tilde
        0x80 => '\u{2022}', // bullet
        0x81 => '\u{2020}', // dagger
        0x82 => '\u{2021}', // double dagger
        0x83 => '\u{2026}', // ellipsis
        0x84 => '\u{2014}', // em dash
        0x85 => '\u{2013}', // en dash
        0x86 => '\u{0192}', // florin
        0x87 => '\u{2044}', // fraction slash
        0x88 => '\u{2039}', // single guillemet left
        0x89 => '\u{203A}', // single guillemet right
        0x8A => '\u{2212}', // minus
        0x8B => '\u{2030}', // per mille
        0x8C => '\u{201E}', // double low-9 quote
        0x8D => '\u{201C}', // left double quote
        0x8E => '\u{201D}', // right double quote
        0x8F => '\u{2018}', // left single quote
        0x90 => '\u{2019}', // right single quote
        0x91 => '\u{201A}', // single low-9 quote
        0x92 => '\u{2122}', // trademark
        0x93 => '\u{FB01}', // fi ligature
        0x94 => '\u{FB02}', // fl ligature
        0x95 => '\u{0141}', // L with stroke
        0x96 => '\u{0152}', // OE
        0x97 => '\u{0160}', // S caron
        0x98 => '\u{0178}', // Y dieresis
        0x99 => '\u{017D}', // Z caron
        0x9A => '\u{0131}', // dotless i
        0x9B => '\u{0142}', // l with stroke
        0x9C => '\u{0153}', // oe
        0x9D => '\u{0161}', // s caron
        0x9E => '\u{017E}', // z caron
        0xA0 => '\u{20AC}', // euro
        other => other as char,
    }
}

/// Split a stored keywords string exactly like the TS `/[,;]\s*/` regex,
/// then drop empty segments (`filter(Boolean)`).
fn split_keywords(joined: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = joined.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ',' || c == ';' {
            while chars.peek().is_some_and(|next| next.is_whitespace()) {
                chars.next();
            }
            out.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    out.push(current);
    out.retain(|segment| !segment.is_empty());
    out
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fixtures::{page_content_text, sample, sample_with_text};
    use lopdf::{dictionary, Stream};
    use serde_json::{json, Value};

    /// `sample(pages)` plus an Info dictionary with the given raw entries.
    fn with_info(pages: usize, entries: &[(&str, Object)]) -> Vec<u8> {
        let mut doc = Document::load_mem(&sample(pages)).unwrap();
        let mut info = Dictionary::new();
        for (key, value) in entries {
            info.set(key.as_bytes().to_vec(), value.clone());
        }
        let info_id = doc.add_object(info);
        doc.trailer.set("Info", info_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    /// `sample(1)` plus a direct trailer Info dictionary.
    fn with_inline_info(entries: &[(&str, Object)]) -> Vec<u8> {
        let mut doc = Document::load_mem(&sample(1)).unwrap();
        let mut info = Dictionary::new();
        for (key, value) in entries {
            info.set(key.as_bytes().to_vec(), value.clone());
        }
        doc.trailer.set("Info", Object::Dictionary(info));
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    /// `sample(pages)` plus a catalog `/Metadata` XMP stream.
    fn with_xmp(pages: usize) -> Vec<u8> {
        let mut doc = Document::load_mem(&sample(pages)).unwrap();
        let xmp = Stream::new(
            dictionary! { "Type" => "Metadata", "Subtype" => "XML" },
            b"<x:xmpmeta>creator-tool-secret</x:xmpmeta>".to_vec(),
        );
        let xmp_id = doc.add_object(xmp);
        doc.catalog_mut().unwrap().set("Metadata", xmp_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    fn lit(text: &str) -> Object {
        Object::string_literal(text)
    }

    fn read_json(data: &[u8]) -> Value {
        serde_json::from_str(&read_metadata_native(data).unwrap()).unwrap()
    }

    /// Raw stored bytes of one Info entry in a saved PDF.
    fn info_entry_bytes(data: &[u8], key: &[u8]) -> Option<Vec<u8>> {
        let doc = Document::load_mem(data).unwrap();
        let id = doc.trailer.get(b"Info").ok()?.as_reference().ok()?;
        match doc.get_dictionary(id).ok()?.get(key).ok()? {
            Object::String(bytes, _) => Some(bytes.clone()),
            _ => None,
        }
    }

    // -- read ---------------------------------------------------------------

    #[test]
    fn reads_all_info_fields_as_json() {
        let pdf = with_info(
            2,
            &[
                ("Title", lit("My Title")),
                ("Author", lit("Ann Author")),
                ("Subject", lit("Subj")),
                ("Keywords", lit("rust, pdf")),
                ("Creator", lit("Creator App")),
                ("Producer", lit("Producer App")),
                ("CreationDate", lit("D:20240102030405Z")),
                ("ModDate", lit("D:20250607120000Z")),
            ],
        );
        let v = read_json(&pdf);
        assert_eq!(v["title"], "My Title");
        assert_eq!(v["author"], "Ann Author");
        assert_eq!(v["subject"], "Subj");
        assert_eq!(v["keywords"], json!(["rust", "pdf"]));
        assert_eq!(v["creator"], "Creator App");
        assert_eq!(v["producer"], "Producer App");
        assert_eq!(v["creationDate"], "D:20240102030405Z");
        assert_eq!(v["modificationDate"], "D:20250607120000Z");
        assert_eq!(v["pageCount"], 2);
    }

    #[test]
    fn read_without_info_dict_returns_only_page_count() {
        let v = read_json(&sample(3));
        assert_eq!(v["pageCount"], 3);
        for key in [
            "title",
            "author",
            "subject",
            "keywords",
            "creator",
            "producer",
            "creationDate",
            "modificationDate",
        ] {
            assert!(v.get(key).is_none(), "{key} should be absent");
        }
    }

    #[test]
    fn read_ignores_malformed_trailer_info_entry() {
        let mut doc = Document::load_mem(&sample(1)).unwrap();
        doc.trailer.set("Info", Object::Integer(42));
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();

        let v = read_json(&buf);
        assert_eq!(v["pageCount"], 1);
        assert!(v.get("title").is_none());
    }

    #[test]
    fn read_splits_keywords_on_commas_and_semicolons() {
        let pdf = with_info(1, &[("Keywords", lit("rust, pdf;wasm,  web"))]);
        assert_eq!(
            read_json(&pdf)["keywords"],
            json!(["rust", "pdf", "wasm", "web"])
        );
    }

    #[test]
    fn read_keyword_edge_cases_match_js_semantics() {
        // Empty string is falsy in JS -> no keywords field at all.
        let pdf = with_info(1, &[("Keywords", lit(""))]);
        assert!(read_json(&pdf).get("keywords").is_none());

        // Non-empty but only separators -> present-but-empty array.
        let pdf = with_info(1, &[("Keywords", lit(";"))]);
        assert_eq!(read_json(&pdf)["keywords"], json!([]));

        // Empty segments are filtered, whitespace after separators is eaten,
        // but trailing whitespace *before* a separator is preserved (regex parity).
        let pdf = with_info(1, &[("Keywords", lit("a,,b ,c"))]);
        assert_eq!(read_json(&pdf)["keywords"], json!(["a", "b ", "c"]));
    }

    #[test]
    fn read_decodes_utf16be_strings() {
        let title = "Привіт — тест";
        let pdf = with_info(1, &[("Title", encode_pdf_string(title))]);
        assert_eq!(read_json(&pdf)["title"], title);
    }

    #[test]
    fn read_decodes_utf8_bom_strings() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("UTF-8 title".as_bytes());
        let pdf = with_info(
            1,
            &[("Title", Object::String(bytes, StringFormat::Literal))],
        );
        assert_eq!(read_json(&pdf)["title"], "UTF-8 title");
    }

    #[test]
    fn read_decodes_full_pdfdoc_encoding_table() {
        let bytes: Vec<u8> = (0x18..=0x1F).chain(0x80..=0xA0).collect();
        let expected: String = [
            '\u{02D8}', '\u{02C7}', '\u{02C6}', '\u{02D9}', '\u{02DD}', '\u{02DB}', '\u{02DA}',
            '\u{02DC}', '\u{2022}', '\u{2020}', '\u{2021}', '\u{2026}', '\u{2014}', '\u{2013}',
            '\u{0192}', '\u{2044}', '\u{2039}', '\u{203A}', '\u{2212}', '\u{2030}', '\u{201E}',
            '\u{201C}', '\u{201D}', '\u{2018}', '\u{2019}', '\u{201A}', '\u{2122}', '\u{FB01}',
            '\u{FB02}', '\u{0141}', '\u{0152}', '\u{0160}', '\u{0178}', '\u{017D}', '\u{0131}',
            '\u{0142}', '\u{0153}', '\u{0161}', '\u{017E}', '\u{009F}', '\u{20AC}',
        ]
        .into_iter()
        .collect();

        assert_eq!(decode_pdf_string(&bytes), expected);
    }

    #[test]
    fn read_decodes_pdfdoc_encoding_high_bytes() {
        // é (Latin-1 range), en dash (0x85), euro (0xA0) in PDFDocEncoding.
        let pdf = with_info(
            1,
            &[(
                "Title",
                Object::String(vec![b'a', 0xE9, 0x85, 0xA0], StringFormat::Literal),
            )],
        );
        assert_eq!(read_json(&pdf)["title"], "aé\u{2013}\u{20AC}");
    }

    #[test]
    fn read_handles_inline_trailer_info_dictionary() {
        let pdf = with_inline_info(&[("Title", lit("Inline Title"))]);
        assert_eq!(read_json(&pdf)["title"], "Inline Title");
    }

    #[test]
    fn read_passes_dates_through_verbatim() {
        let pdf = with_info(
            1,
            &[
                ("CreationDate", lit("D:20240102030405Z")),
                ("ModDate", lit("D:20250607120000+02'00'")),
            ],
        );
        let v = read_json(&pdf);
        assert_eq!(v["creationDate"], "D:20240102030405Z");
        assert_eq!(v["modificationDate"], "D:20250607120000+02'00'");
    }

    #[test]
    fn read_ignores_non_string_info_values() {
        let pdf = with_info(1, &[("Title", Object::Integer(42)), ("Author", lit("Ann"))]);
        let v = read_json(&pdf);
        assert!(v.get("title").is_none());
        assert_eq!(v["author"], "Ann");
        assert!(info_entry_bytes(&pdf, b"Title").is_none());
    }

    #[test]
    fn read_rejects_invalid_input() {
        assert!(read_metadata_native(b"").is_err());
        assert!(read_metadata_native(b"definitely not a pdf").is_err());
    }

    // -- set ----------------------------------------------------------------

    #[test]
    fn set_round_trips_every_field() {
        let opts = json!({
            "title": "T", "author": "A", "subject": "S",
            "keywords": ["k1", "k2"], "creator": "C", "producer": "P",
            "creationDate": "D:20240101000000Z", "modificationDate": "D:20250101000000Z",
        });
        let out = set_metadata_native(&sample(2), &opts.to_string()).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        let v = read_json(&out);
        assert_eq!(v["title"], "T");
        assert_eq!(v["author"], "A");
        assert_eq!(v["subject"], "S");
        assert_eq!(v["keywords"], json!(["k1", "k2"]));
        assert_eq!(v["creator"], "C");
        assert_eq!(v["producer"], "P");
        assert_eq!(v["creationDate"], "D:20240101000000Z");
        assert_eq!(v["modificationDate"], "D:20250101000000Z");
        assert_eq!(v["pageCount"], 2);
    }

    #[test]
    fn set_creates_info_dict_when_missing() {
        let out = set_metadata_native(&sample(1), r#"{"title":"New"}"#).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        assert!(doc.trailer.get(b"Info").is_ok());
        assert_eq!(read_json(&out)["title"], "New");
    }

    #[test]
    fn set_materializes_inline_trailer_info_dictionary() {
        let pdf = with_inline_info(&[("Title", lit("Keep"))]);
        let out = set_metadata_native(&pdf, r#"{"author":"Add"}"#).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        assert!(doc.trailer.get(b"Info").unwrap().as_reference().is_ok());

        let v = read_json(&out);
        assert_eq!(v["title"], "Keep");
        assert_eq!(v["author"], "Add");
    }

    #[test]
    fn set_only_touches_provided_fields() {
        let pdf = with_info(
            1,
            &[("Author", lit("Keep Me")), ("Subject", lit("Old Subject"))],
        );
        let out = set_metadata_native(&pdf, r#"{"title":"New Title"}"#).unwrap();
        let v = read_json(&out);
        assert_eq!(v["title"], "New Title");
        assert_eq!(v["author"], "Keep Me");
        assert_eq!(v["subject"], "Old Subject");
    }

    #[test]
    fn set_empty_string_clears_field() {
        let pdf = with_info(1, &[("Title", lit("Old")), ("Author", lit("Ann"))]);
        let out = set_metadata_native(&pdf, r#"{"title":""}"#).unwrap();
        let v = read_json(&out);
        assert!(v.get("title").is_none());
        assert_eq!(v["author"], "Ann");
        assert!(info_entry_bytes(&out, b"Title").is_none());
    }

    #[test]
    fn set_keywords_joined_with_comma_space() {
        let out =
            set_metadata_native(&sample(1), r#"{"keywords":["alpha","beta","gamma"]}"#).unwrap();
        // Stored as ONE comma-separated string so the delimiter round-trips.
        assert_eq!(
            info_entry_bytes(&out, b"Keywords").unwrap(),
            b"alpha, beta, gamma"
        );
        assert_eq!(
            read_json(&out)["keywords"],
            json!(["alpha", "beta", "gamma"])
        );
    }

    #[test]
    fn set_keywords_accepts_comma_separated_string() {
        let out = set_metadata_native(&sample(1), r#"{"keywords":" a, b ,, c "}"#).unwrap();
        assert_eq!(info_entry_bytes(&out, b"Keywords").unwrap(), b"a, b, c");
        assert_eq!(read_json(&out)["keywords"], json!(["a", "b", "c"]));
    }

    #[test]
    fn set_empty_keywords_clears_entry() {
        let pdf = with_info(1, &[("Keywords", lit("old, stuff"))]);
        for opts in [r#"{"keywords":[]}"#, r#"{"keywords":""}"#] {
            let out = set_metadata_native(&pdf, opts).unwrap();
            assert!(
                info_entry_bytes(&out, b"Keywords").is_none(),
                "opts: {opts}"
            );
            assert!(read_json(&out).get("keywords").is_none(), "opts: {opts}");
        }
    }

    #[test]
    fn set_unicode_writes_utf16be_with_bom() {
        let out =
            set_metadata_native(&sample(1), r#"{"title":"Привіт","author":"ASCII Only"}"#).unwrap();
        let title_bytes = info_entry_bytes(&out, b"Title").unwrap();
        assert_eq!(&title_bytes[..2], &[0xFE, 0xFF], "UTF-16BE BOM expected");
        let author_bytes = info_entry_bytes(&out, b"Author").unwrap();
        assert_eq!(author_bytes, b"ASCII Only", "ASCII stays single-byte");
        let v = read_json(&out);
        assert_eq!(v["title"], "Привіт");
        assert_eq!(v["author"], "ASCII Only");
    }

    #[test]
    fn set_escapes_parens_and_backslashes() {
        let tricky = r"a (weird) \ title)(";
        let opts = json!({ "title": tricky }).to_string();
        let out = set_metadata_native(&sample(1), &opts).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(read_json(&out)["title"], tricky);
    }

    #[test]
    fn set_dates_stored_verbatim_and_clearable() {
        let out =
            set_metadata_native(&sample(1), r#"{"creationDate":"D:20231231235959Z"}"#).unwrap();
        assert_eq!(
            info_entry_bytes(&out, b"CreationDate").unwrap(),
            b"D:20231231235959Z"
        );
        let cleared = set_metadata_native(&out, r#"{"creationDate":""}"#).unwrap();
        assert!(read_json(&cleared).get("creationDate").is_none());
    }

    #[test]
    fn set_rejects_invalid_json() {
        assert!(set_metadata_native(&sample(1), "not json").is_err());
        assert!(set_metadata_native(&sample(1), r#"{"keywords":42}"#).is_err());
        assert!(set_metadata_native(b"not a pdf", r#"{"title":"x"}"#).is_err());
    }

    #[test]
    fn set_preserves_pages_and_content() {
        let pdf = sample_with_text(2, Some("Hello"));
        let out = set_metadata_native(&pdf, r#"{"title":"Annotated"}"#).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        let doc = Document::load_mem(&out).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
        assert!(page_content_text(&out, 0).contains("Hello 0"));
        assert!(page_content_text(&out, 1).contains("Hello 1"));
    }

    // -- strip --------------------------------------------------------------

    #[test]
    fn strip_removes_info_dictionary() {
        let opts = json!({
            "title": "T", "author": "A", "subject": "S", "keywords": ["k"],
            "creator": "C", "producer": "P", "creationDate": "D:20240101000000Z",
        });
        let set = set_metadata_native(&sample(1), &opts.to_string()).unwrap();
        let out = strip_metadata_native(&set).unwrap();
        let v = read_json(&out);
        assert_eq!(v["pageCount"], 1);
        assert_eq!(
            v.as_object().unwrap().len(),
            1,
            "only pageCount should remain"
        );
        let doc = Document::load_mem(&out).unwrap();
        assert!(
            doc.trailer.get(b"Info").is_err(),
            "trailer /Info should be gone"
        );
    }

    #[test]
    fn strip_removes_xmp_metadata_stream() {
        let pdf = with_xmp(1);
        // Sanity: fixture really has the stream before stripping.
        let before = Document::load_mem(&pdf).unwrap();
        assert!(before.catalog().unwrap().get(b"Metadata").is_ok());

        let out = strip_metadata_native(&pdf).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        assert!(doc.catalog().unwrap().get(b"Metadata").is_err());
        let has_xmp_stream = doc.objects.values().any(|obj| {
            obj.as_stream()
                .map(|s| {
                    s.dict
                        .get(b"Type")
                        .and_then(Object::as_name)
                        .map(|n| n == b"Metadata")
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        });
        assert!(!has_xmp_stream, "XMP stream object should be removed");
    }

    #[test]
    fn strip_is_idempotent_and_preserves_content() {
        let pdf = sample_with_text(2, Some("Body"));
        let once = strip_metadata_native(&pdf).unwrap(); // no metadata at all: still fine
        let twice = strip_metadata_native(&once).unwrap();
        assert!(twice.starts_with(b"%PDF-"));
        let doc = Document::load_mem(&twice).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
        assert!(page_content_text(&twice, 0).contains("Body 0"));
        let v = read_json(&twice);
        assert_eq!(v.as_object().unwrap().len(), 1);
        assert_eq!(v["pageCount"], 2);
    }

    #[test]
    fn strip_rejects_invalid_input() {
        assert!(strip_metadata_native(b"").is_err());
        assert!(strip_metadata_native(b"nope").is_err());
    }
}
