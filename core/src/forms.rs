//! AcroForm field operations: list, fill, flatten.
//!
//! Rust port of `src/lib/tools/forms.ts` (pdf-lib `listFormFields`, `fillForm`,
//! `flattenForm`) with behaviour parity:
//!
//! - **list** walks the catalog `/AcroForm /Fields` tree (recursing `/Kids` that
//!   are themselves fields), classifies each terminal field from `/FT` + the
//!   `/Ff` flag bits, and reports its current value and (for choice/radio fields)
//!   its options — exactly the shape pdf-lib derives from its typed field classes.
//! - **fill** applies a `{fieldName: string|bool}` map: text fields get a new
//!   `/V` plus a regenerated `/AP /N` appearance stream, checkboxes/radios switch
//!   `/V`+`/AS` between their on-state name and `/Off`, choice fields get a new
//!   `/V`. Unknown field names are silently ignored (pdf-lib's `getFieldMaybe`
//!   contract). `/NeedAppearances` is set false, mirroring pdf-lib's `doc.save()`.
//! - **flatten** bakes each widget's selected `/AP /N` appearance into the host
//!   page's content as a `q cm /X Do Q` XObject draw, drops the widget
//!   annotations, and removes the catalog `/AcroForm`.
//!
//! Text `/V` values are stored UTF-16BE (`FE FF` prefix) like pdf-lib's
//! `PDFHexString.fromText`, so any Unicode round-trips. Appearance-stream glyphs
//! are WinAnsi (via [`crate::stamp_text::encode_winansi`]); characters outside
//! WinAnsi render as `?` — pdf-lib's documented degraded-appearance behaviour.
//!
//! Encrypted input is rejected with a friendly error: decryption is the crypto
//! module's job, and the form value strings are unreadable until that runs.

use std::collections::HashSet;

use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream, StringFormat};
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::stamp_text::{encode_winansi, text_width_pt, HELVETICA_WIDTHS};
use crate::util::{number_as_f32, save_compact};

// /Ff field-flag bits (PDF 32000-1 Table 226/227/228), 1-based in the spec.
const FF_MULTILINE: i64 = 1 << 12; // text, bit 13
const FF_RADIO: i64 = 1 << 15; // button, bit 16
const FF_PUSHBUTTON: i64 = 1 << 16; // button, bit 17
const FF_COMBO: i64 = 1 << 17; // choice, bit 18

/// Bound on `/Parent` chain walks — guards against malformed cyclic trees.
const MAX_PARENT_DEPTH: usize = 64;

// ---------------------------------------------------------------------------
// JSON shapes
// ---------------------------------------------------------------------------

/// One entry of the `listFormFields` result. `type` is one of
/// `text|checkbox|radio|dropdown|optionlist|button|signature|unknown`.
#[derive(Serialize)]
struct FormField {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<JsonValue>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// JSON array of every terminal AcroForm field: `{name, type, options?, value?}`.
/// Returns `[]` for a PDF with no AcroForm.
pub fn list_form_fields_native(data: &[u8]) -> Result<String, String> {
    let doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    if doc.is_encrypted() {
        return Err("PDF is encrypted; decrypt it before reading form fields".to_string());
    }

    let mut out: Vec<FormField> = Vec::new();
    for (name, fid) in terminal_fields(&doc) {
        out.push(build_field(&doc, fid, name));
    }
    serde_json::to_string(&out).map_err(|e| e.to_string())
}

/// Apply `values_json` (`{fieldName: string|bool}`) to the form, regenerating
/// text/choice appearance streams and toggling checkbox/radio states. Unknown
/// field names are ignored. Sets `/NeedAppearances` false. Returns new PDF bytes.
pub fn fill_form_native(data: &[u8], values_json: &str) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    if doc.is_encrypted() {
        return Err("PDF is encrypted; decrypt it before filling the form".to_string());
    }

    let values = serde_json::from_str::<JsonValue>(values_json).map_err(|e| e.to_string())?;
    let map = match values {
        JsonValue::Object(m) => m,
        _ => return Err("Form values must be a JSON object".to_string()),
    };

    let acro_id = match acroform_id(&doc) {
        Some(id) => id,
        // No form: nothing to fill — return the (re-saved) document unchanged.
        None => return save_compact(doc).map_err(|e| e.to_string()),
    };
    let acro_da = doc
        .get_dictionary(acro_id)
        .ok()
        .and_then(|d| d.get(b"DA").ok())
        .and_then(|o| o.as_str().ok())
        .map(decode_bytes);

    let fields = terminal_fields(&doc);
    // One shared Standard-14 Helvetica font for all regenerated appearances.
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });

    for (name, fid) in &fields {
        if let Some(val) = map.get(name) {
            apply_value(&mut doc, *fid, val, acro_da.as_deref(), font_id)?;
        }
    }

    if let Ok(d) = doc.get_object_mut(acro_id).and_then(Object::as_dict_mut) {
        d.set("NeedAppearances", Object::Boolean(false));
    }
    save_compact(doc).map_err(|e| e.to_string())
}

/// Bake current widget appearances into page content, drop the widget
/// annotations, and remove the catalog `/AcroForm`. Returns new PDF bytes.
pub fn flatten_form_native(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    if doc.is_encrypted() {
        return Err("PDF is encrypted; decrypt it before flattening the form".to_string());
    }

    let page_ids: Vec<ObjectId> = doc.page_iter().collect();
    for page_id in page_ids {
        flatten_page(&mut doc, page_id)?;
    }
    if let Ok(cat) = doc.catalog_mut() {
        cat.remove(b"AcroForm");
    }
    save_compact(doc).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Field-tree traversal
// ---------------------------------------------------------------------------

fn acroform_id(doc: &Document) -> Option<ObjectId> {
    doc.catalog()
        .ok()?
        .get(b"AcroForm")
        .ok()?
        .as_reference()
        .ok()
}

fn acroform_field_ids(doc: &Document) -> Vec<ObjectId> {
    acroform_id(doc)
        .and_then(|id| doc.get_dictionary(id).ok())
        .and_then(|d| d.get(b"Fields").ok())
        .and_then(|o| o.as_array().ok())
        .map(|arr| arr.iter().filter_map(|o| o.as_reference().ok()).collect())
        .unwrap_or_default()
}

/// All terminal (leaf) fields as `(fully-qualified-name, object-id)`, in tree order.
fn terminal_fields(doc: &Document) -> Vec<(String, ObjectId)> {
    let mut out = Vec::new();
    let mut visited = HashSet::new();
    for fid in acroform_field_ids(doc) {
        walk_terminal(doc, fid, "", &mut visited, &mut out);
    }
    out
}

fn walk_terminal(
    doc: &Document,
    fid: ObjectId,
    prefix: &str,
    visited: &mut HashSet<ObjectId>,
    out: &mut Vec<(String, ObjectId)>,
) {
    if !visited.insert(fid) {
        return;
    }
    let dict = match doc.get_dictionary(fid) {
        Ok(d) => d,
        Err(_) => return,
    };

    let partial = dict
        .get(b"T")
        .ok()
        .and_then(|o| o.as_str().ok())
        .map(decode_pdf_text);
    let full = match partial {
        Some(p) if prefix.is_empty() => p,
        Some(p) => format!("{prefix}.{p}"),
        None => prefix.to_string(),
    };

    // Kids that are themselves fields (have a partial name /T) are sub-fields;
    // kids without /T are widget annotations belonging to this terminal field.
    let kid_fields: Vec<ObjectId> = dict
        .get(b"Kids")
        .ok()
        .and_then(|o| o.as_array().ok())
        .map(|arr| {
            arr.iter()
                .filter_map(|k| k.as_reference().ok())
                .filter(|&kid| {
                    doc.get_dictionary(kid)
                        .map(|d| d.has(b"T"))
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();

    if kid_fields.is_empty() {
        out.push((full, fid));
    } else {
        for kf in kid_fields {
            walk_terminal(doc, kf, &full, visited, out);
        }
    }
}

/// Follow `/Parent` looking for an attribute (`/FT`, `/Ff`) that may be inherited.
fn get_inherited<'a>(doc: &'a Document, start: ObjectId, key: &[u8]) -> Option<&'a Object> {
    let mut id = start;
    for _ in 0..MAX_PARENT_DEPTH {
        let d = doc.get_dictionary(id).ok()?;
        if let Ok(v) = d.get(key) {
            return Some(v);
        }
        id = d.get(b"Parent").ok()?.as_reference().ok()?;
    }
    None
}

fn field_type(ft: Option<&[u8]>, ff: i64) -> &'static str {
    match ft {
        Some(b"Tx") => "text",
        Some(b"Btn") => {
            if ff & FF_PUSHBUTTON != 0 {
                "button"
            } else if ff & FF_RADIO != 0 {
                "radio"
            } else {
                "checkbox"
            }
        }
        Some(b"Ch") => {
            if ff & FF_COMBO != 0 {
                "dropdown"
            } else {
                "optionlist"
            }
        }
        Some(b"Sig") => "signature",
        _ => "unknown",
    }
}

// ---------------------------------------------------------------------------
// list: build one FormField
// ---------------------------------------------------------------------------

fn build_field(doc: &Document, fid: ObjectId, name: String) -> FormField {
    let ft = get_inherited(doc, fid, b"FT")
        .and_then(|o| o.as_name().ok())
        .map(|b| b.to_vec());
    let ff = get_inherited(doc, fid, b"Ff")
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(0);
    let ty = field_type(ft.as_deref(), ff);

    let dict = doc.get_dictionary(fid).ok();
    let (options, value) = match (ty, dict) {
        ("text", Some(d)) => (None, Some(JsonValue::String(text_value(d)))),
        ("checkbox", Some(d)) => (None, Some(JsonValue::Bool(checkbox_value(d)))),
        ("radio", Some(d)) => (
            Some(radio_options(doc, d)),
            Some(JsonValue::String(radio_value(doc, d))),
        ),
        ("dropdown" | "optionlist", Some(d)) => (
            Some(opt_display_values(d)),
            Some(JsonValue::String(choice_value(d))),
        ),
        _ => (None, None),
    };

    FormField {
        name,
        ty: ty.to_string(),
        options,
        value,
    }
}

fn text_value(dict: &Dictionary) -> String {
    dict.get(b"V")
        .ok()
        .and_then(|o| o.as_str().ok())
        .map(decode_pdf_text)
        .unwrap_or_default()
}

fn checkbox_value(dict: &Dictionary) -> bool {
    dict.get(b"V")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(|n| n != b"Off")
        .unwrap_or(false)
}

/// Export/display labels from `/Opt`: a plain string entry is used as-is; a
/// `[export, display]` pair contributes its display element (pdf-lib parity).
fn opt_display_values(dict: &Dictionary) -> Vec<String> {
    dict.get(b"Opt")
        .ok()
        .and_then(|o| o.as_array().ok())
        .map(|arr| {
            arr.iter()
                .map(|e| match e {
                    Object::Array(pair) => pair
                        .get(1)
                        .or_else(|| pair.first())
                        .and_then(|o| o.as_str().ok())
                        .map(decode_pdf_text)
                        .unwrap_or_default(),
                    Object::String(b, _) => decode_pdf_text(b),
                    Object::Name(b) => decode_bytes(b),
                    _ => String::new(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn radio_options(doc: &Document, dict: &Dictionary) -> Vec<String> {
    let opts = opt_display_values(dict);
    if !opts.is_empty() {
        return opts;
    }
    // No /Opt: fall back to the widgets' on-state names.
    widget_ids(doc, dict)
        .iter()
        .filter_map(|&w| widget_on_state(doc, w))
        .map(|b| decode_bytes(&b))
        .collect()
}

/// The selected radio value: `/V` is the chosen widget on-state name; map it
/// through the widget order onto `/Opt` to recover the export label.
fn radio_value(doc: &Document, dict: &Dictionary) -> String {
    let v = match dict.get(b"V").ok().and_then(|o| o.as_name().ok()) {
        Some(v) => v.to_vec(),
        None => return String::new(),
    };
    let opts = opt_display_values(dict);
    let on_states: Vec<Vec<u8>> = widget_ids(doc, dict)
        .iter()
        .map(|&w| widget_on_state(doc, w).unwrap_or_default())
        .collect();
    if let Some(idx) = on_states.iter().position(|s| *s == v) {
        if let Some(label) = opts.get(idx) {
            return label.clone();
        }
    }
    decode_bytes(&v)
}

fn choice_value(dict: &Dictionary) -> String {
    match dict.get(b"V") {
        Ok(Object::String(b, _)) => decode_pdf_text(b),
        Ok(Object::Array(a)) => a
            .first()
            .and_then(|o| o.as_str().ok())
            .map(decode_pdf_text)
            .unwrap_or_default(),
        Ok(Object::Name(b)) => decode_bytes(b),
        _ => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Widget helpers (shared by list / fill / flatten)
// ---------------------------------------------------------------------------

/// Widget annotation object-ids of a field: its `/Kids` without a `/T`, or the
/// field dict itself when the field and its single widget are merged.
fn widget_ids(doc: &Document, dict: &Dictionary) -> Vec<ObjectId> {
    if let Ok(kids) = dict.get(b"Kids").and_then(|o| o.as_array()) {
        let ws: Vec<ObjectId> = kids
            .iter()
            .filter_map(|k| k.as_reference().ok())
            .filter(|&k| doc.get_dictionary(k).map(|d| !d.has(b"T")).unwrap_or(false))
            .collect();
        if !ws.is_empty() {
            return ws;
        }
    }
    Vec::new()
}

/// Like [`widget_ids`] but falls back to `[fid]` for a merged field/widget.
fn widget_ids_or_self(doc: &Document, fid: ObjectId, dict: &Dictionary) -> Vec<ObjectId> {
    let ws = widget_ids(doc, dict);
    if ws.is_empty() {
        vec![fid]
    } else {
        ws
    }
}

fn follow_dict<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Dictionary> {
    match obj {
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        Object::Dictionary(d) => Some(d),
        _ => None,
    }
}

/// The widget's appearance on-state name (the first `/AP /N` key that is not
/// `/Off`), or `None` when the appearance is a single stream (text/choice).
fn widget_on_state(doc: &Document, wid: ObjectId) -> Option<Vec<u8>> {
    let wd = doc.get_dictionary(wid).ok()?;
    let ap = wd.get(b"AP").ok()?;
    let apd = follow_dict(doc, ap)?;
    let n = apd.get(b"N").ok()?;
    let states = match n {
        Object::Dictionary(d) => d,
        Object::Reference(id) => doc.get_object(*id).ok()?.as_dict().ok()?,
        _ => return None,
    };
    states
        .iter()
        .map(|(k, _)| k)
        .find(|k| k.as_slice() != b"Off")
        .map(|k| k.to_vec())
}

fn read_rect(doc: &Document, wid: ObjectId) -> Option<[f32; 4]> {
    let arr = doc
        .get_dictionary(wid)
        .ok()?
        .get(b"Rect")
        .ok()?
        .as_array()
        .ok()?;
    if arr.len() != 4 {
        return None;
    }
    let mut r = [0.0f32; 4];
    for (i, slot) in r.iter_mut().enumerate() {
        *slot = number_as_f32(&arr[i])?;
    }
    Some(r)
}

// ---------------------------------------------------------------------------
// fill
// ---------------------------------------------------------------------------

/// Everything `apply_value` needs, read out of the field before mutation.
struct FieldInfo {
    ty: String,
    multiline: bool,
    da: Option<String>,
    widgets: Vec<ObjectId>,
    on_states: Vec<Vec<u8>>,
    opt_values: Vec<String>,
}

fn read_field_info(doc: &Document, fid: ObjectId, acro_da: Option<&str>) -> FieldInfo {
    let ft = get_inherited(doc, fid, b"FT")
        .and_then(|o| o.as_name().ok())
        .map(|b| b.to_vec());
    let ff = get_inherited(doc, fid, b"Ff")
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(0);
    let ty = field_type(ft.as_deref(), ff).to_string();

    let dict = doc.get_dictionary(fid).ok();
    let da = dict
        .and_then(|d| d.get(b"DA").ok())
        .and_then(|o| o.as_str().ok())
        .map(decode_bytes)
        .or_else(|| acro_da.map(|s| s.to_string()));

    let widgets = dict
        .map(|d| widget_ids_or_self(doc, fid, d))
        .unwrap_or_else(|| vec![fid]);
    let on_states = widgets
        .iter()
        .map(|&w| widget_on_state(doc, w).unwrap_or_default())
        .collect();
    let opt_values = dict.map(opt_display_values).unwrap_or_default();

    FieldInfo {
        ty,
        multiline: ff & FF_MULTILINE != 0,
        da,
        widgets,
        on_states,
        opt_values,
    }
}

fn apply_value(
    doc: &mut Document,
    fid: ObjectId,
    val: &JsonValue,
    acro_da: Option<&str>,
    font_id: ObjectId,
) -> Result<(), String> {
    let info = read_field_info(doc, fid, acro_da);

    match info.ty.as_str() {
        "text" => {
            let text = json_to_string(val);
            set_object(doc, fid, "V", encode_v_text(&text))?;
            for &w in &info.widgets {
                set_text_appearance(doc, w, &text, info.multiline, info.da.as_deref(), font_id)?;
            }
        }
        "checkbox" => {
            let on = info
                .on_states
                .iter()
                .find(|s| !s.is_empty())
                .cloned()
                .unwrap_or_else(|| b"Yes".to_vec());
            let state = if is_truthy(val) { on } else { b"Off".to_vec() };
            set_object(doc, fid, "V", Object::Name(state.clone()))?;
            for &w in &info.widgets {
                set_object(doc, w, "AS", Object::Name(state.clone()))?;
            }
        }
        "radio" => {
            let target = json_to_string(val);
            let sel = info.widgets.iter().enumerate().position(|(i, _)| {
                info.opt_values
                    .get(i)
                    .map(|o| *o == target)
                    .unwrap_or(false)
                    || info
                        .on_states
                        .get(i)
                        .map(|s| decode_bytes(s) == target)
                        .unwrap_or(false)
            });
            if let Some(idx) = sel {
                let on = info
                    .on_states
                    .get(idx)
                    .cloned()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| target.clone().into_bytes());
                set_object(doc, fid, "V", Object::Name(on.clone()))?;
                for (j, &w) in info.widgets.iter().enumerate() {
                    let st = if j == idx {
                        on.clone()
                    } else {
                        b"Off".to_vec()
                    };
                    set_object(doc, w, "AS", Object::Name(st))?;
                }
            }
            // No matching option: ignore (pdf-lib would throw; we stay lenient).
        }
        "dropdown" | "optionlist" => {
            let text = json_to_string(val);
            set_object(doc, fid, "V", encode_v_text(&text))?;
            for &w in &info.widgets {
                set_text_appearance(doc, w, &text, false, info.da.as_deref(), font_id)?;
            }
        }
        // button / signature / unknown: not fillable.
        _ => {}
    }
    Ok(())
}

fn set_object(doc: &mut Document, id: ObjectId, key: &str, val: Object) -> Result<(), String> {
    doc.get_object_mut(id)
        .map_err(|e| e.to_string())?
        .as_dict_mut()
        .map_err(|e| e.to_string())?
        .set(key, val);
    Ok(())
}

/// Build (or replace) a widget's `/AP /N` text appearance showing `text`.
fn set_text_appearance(
    doc: &mut Document,
    wid: ObjectId,
    text: &str,
    multiline: bool,
    da: Option<&str>,
    font_id: ObjectId,
) -> Result<(), String> {
    let rect = read_rect(doc, wid).unwrap_or([0.0, 0.0, 100.0, 18.0]);
    let w = (rect[2] - rect[0]).abs();
    let h = (rect[3] - rect[1]).abs();
    let (font_key, size, color) = parse_da(da, w, h, multiline, text);
    let content = build_appearance_content(&font_key, size, color, w, h, multiline, text)?;

    let mut font_sub = Dictionary::new();
    font_sub.set(font_key.clone().into_bytes(), Object::Reference(font_id));
    let mut resources = Dictionary::new();
    resources.set("Font", Object::Dictionary(font_sub));

    let xobj = Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "FormType" => 1,
            "BBox" => vec![0.0.into(), 0.0.into(), Object::Real(w), Object::Real(h)],
            "Matrix" => vec![1.into(), 0.into(), 0.into(), 1.into(), 0.into(), 0.into()],
            "Resources" => Object::Dictionary(resources),
        },
        content,
    );
    let sid = doc.add_object(xobj);

    let mut ap = Dictionary::new();
    ap.set("N", Object::Reference(sid));
    set_object(doc, wid, "AP", Object::Dictionary(ap))
}

/// Parse `/DA` for the font resource key, size, and fill colour. A `0`/missing
/// size auto-fits the rectangle. Defaults: Helvetica, black.
fn parse_da(
    da: Option<&str>,
    w: f32,
    h: f32,
    multiline: bool,
    text: &str,
) -> (String, f32, [f32; 3]) {
    let mut key = "Helvetica".to_string();
    let mut size = 0.0f32;
    let mut color = [0.0f32; 3];

    if let Some(s) = da {
        let t: Vec<&str> = s.split_whitespace().collect();
        for i in 0..t.len() {
            match t[i] {
                "Tf" if i >= 2 => {
                    key = t[i - 2].trim_start_matches('/').to_string();
                    if let Ok(v) = t[i - 1].parse::<f32>() {
                        size = v;
                    }
                }
                "rg" if i >= 3 => {
                    for (j, c) in color.iter_mut().enumerate() {
                        if let Ok(v) = t[i - 3 + j].parse::<f32>() {
                            *c = v;
                        }
                    }
                }
                "g" if i >= 1 => {
                    if let Ok(v) = t[i - 1].parse::<f32>() {
                        color = [v, v, v];
                    }
                }
                _ => {}
            }
        }
    }
    if key.is_empty() {
        key = "Helvetica".to_string();
    }
    if size <= 0.0 {
        size = auto_size(w, h, multiline, text);
    }
    (key, size, color)
}

fn auto_size(w: f32, h: f32, multiline: bool, text: &str) -> f32 {
    if multiline {
        return 12.0;
    }
    let by_height = (h - 4.0).clamp(4.0, 12.0);
    let unit_width = text_width_pt(&encode_winansi(text), &HELVETICA_WIDTHS, 1.0);
    if unit_width > 0.0 {
        let by_width = (w - 4.0) / unit_width;
        by_height.min(by_width).max(4.0)
    } else {
        by_height
    }
}

/// `/Tx BMC q BT rg Tf (Tm Tj)+ ET Q EMC` — pdf-lib's text field appearance shape.
fn build_appearance_content(
    font_key: &str,
    size: f32,
    color: [f32; 3],
    _w: f32,
    h: f32,
    multiline: bool,
    text: &str,
) -> Result<Vec<u8>, String> {
    let lines: Vec<&str> = if multiline {
        text.split('\n').collect()
    } else {
        vec![text]
    };

    let mut ops = vec![
        Operation::new("BMC", vec![Object::Name(b"Tx".to_vec())]),
        Operation::new("q", vec![]),
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
                Object::Real(size),
            ],
        ),
    ];

    let x = 2.0f32;
    let leading = size * 1.15;
    let single_y = ((h - size) / 2.0 + size * 0.2).max(2.0);
    for (i, line) in lines.iter().enumerate() {
        let y = if multiline {
            (h - 2.0 - size) - (i as f32) * leading
        } else {
            single_y
        };
        ops.push(Operation::new(
            "Tm",
            vec![
                Object::Real(1.0),
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(1.0),
                Object::Real(x),
                Object::Real(y),
            ],
        ));
        ops.push(Operation::new(
            "Tj",
            vec![Object::String(encode_winansi(line), StringFormat::Literal)],
        ));
    }

    ops.push(Operation::new("ET", vec![]));
    ops.push(Operation::new("Q", vec![]));
    ops.push(Operation::new("EMC", vec![]));
    Content { operations: ops }
        .encode()
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// flatten
// ---------------------------------------------------------------------------

/// One widget to bake: `(appearance stream id, its BBox, its Matrix, widget Rect)`.
type WidgetDraw = (ObjectId, [f32; 4], [f32; 6], [f32; 4]);

fn flatten_page(doc: &mut Document, page_id: ObjectId) -> Result<(), String> {
    let annot_objs: Vec<Object> = doc
        .get_dictionary(page_id)
        .ok()
        .and_then(|d| d.get(b"Annots").ok().cloned())
        .map(|o| match o {
            Object::Array(a) => a,
            Object::Reference(id) => doc
                .get_object(id)
                .ok()
                .and_then(|o| o.as_array().ok())
                .cloned()
                .unwrap_or_default(),
            _ => Vec::new(),
        })
        .unwrap_or_default();

    if annot_objs.is_empty() {
        return Ok(());
    }

    let mut keep: Vec<Object> = Vec::new();
    let mut draws: Vec<WidgetDraw> = Vec::new();

    for obj in &annot_objs {
        let aid = match obj.as_reference() {
            Ok(id) => id,
            Err(_) => {
                keep.push(obj.clone());
                continue;
            }
        };
        let is_widget = doc
            .get_dictionary(aid)
            .ok()
            .and_then(|d| d.get(b"Subtype").ok())
            .and_then(|o| o.as_name().ok())
            .map(|n| n == b"Widget")
            .unwrap_or(false);
        if !is_widget {
            keep.push(Object::Reference(aid));
            continue;
        }
        // It's a widget: bake it and drop it from /Annots.
        let rect = read_rect(doc, aid).unwrap_or([0.0, 0.0, 0.0, 0.0]);
        let field_id = doc
            .get_dictionary(aid)
            .ok()
            .and_then(|d| d.get(b"Parent").ok())
            .and_then(|o| o.as_reference().ok());
        if let Some(sid) = select_ap_stream(doc, aid, field_id) {
            let (bbox, matrix) = read_bbox_matrix(doc, sid);
            draws.push((sid, bbox, matrix, rect));
        }
    }

    if !draws.is_empty() {
        let mut ops: Vec<Operation> = Vec::new();
        for (sid, bbox, matrix, rect) in &draws {
            let key = add_page_xobject(doc, page_id, Object::Reference(*sid))?;
            let cm = appearance_cm(*bbox, *matrix, *rect);
            ops.push(Operation::new("q", vec![]));
            ops.push(Operation::new(
                "cm",
                cm.iter().map(|&v| Object::Real(v)).collect(),
            ));
            ops.push(Operation::new("Do", vec![Object::Name(key.into_bytes())]));
            ops.push(Operation::new("Q", vec![]));
        }
        let body = Content { operations: ops }
            .encode()
            .map_err(|e| e.to_string())?;
        let mut padded = Vec::with_capacity(body.len() + 2);
        padded.push(b'\n');
        padded.extend(body);
        padded.push(b'\n');
        let cid = doc.add_object(Stream::new(Dictionary::new(), padded));
        append_page_content(doc, page_id, cid)?;
    }

    // Replace /Annots with the kept (non-widget) annotations.
    let page = doc
        .get_object_mut(page_id)
        .map_err(|e| e.to_string())?
        .as_dict_mut()
        .map_err(|e| e.to_string())?;
    if keep.is_empty() {
        page.remove(b"Annots");
    } else {
        page.set("Annots", keep);
    }
    Ok(())
}

/// Pick the appearance stream to bake for a widget: a direct `/AP /N` stream, or
/// — for state appearances — the sub-stream named by the field `/V`, else `/AS`,
/// else `/Off`, else the first state.
fn select_ap_stream(doc: &Document, wid: ObjectId, field_id: Option<ObjectId>) -> Option<ObjectId> {
    let wd = doc.get_dictionary(wid).ok()?;
    let ap = wd.get(b"AP").ok()?;
    let apd = follow_dict(doc, ap)?;
    let n = apd.get(b"N").ok()?;

    match n {
        Object::Reference(id) => match doc.get_object(*id).ok()? {
            Object::Stream(_) => Some(*id),
            Object::Dictionary(states) => pick_state(doc, states, wid, field_id),
            _ => None,
        },
        Object::Dictionary(states) => pick_state(doc, states, wid, field_id),
        _ => None,
    }
}

fn pick_state(
    doc: &Document,
    states: &Dictionary,
    wid: ObjectId,
    field_id: Option<ObjectId>,
) -> Option<ObjectId> {
    let v_name = field_id
        .and_then(|fid| doc.get_dictionary(fid).ok())
        .and_then(|fd| fd.get(b"V").ok())
        .and_then(|o| o.as_name().ok())
        .map(|b| b.to_vec());
    let as_name = doc
        .get_dictionary(wid)
        .ok()
        .and_then(|wd| wd.get(b"AS").ok())
        .and_then(|o| o.as_name().ok())
        .map(|b| b.to_vec());

    for cand in [v_name, as_name].into_iter().flatten() {
        if let Ok(r) = states.get(&cand) {
            if let Ok(id) = r.as_reference() {
                return Some(id);
            }
        }
    }
    if let Ok(r) = states.get(b"Off") {
        if let Ok(id) = r.as_reference() {
            return Some(id);
        }
    }
    states
        .iter()
        .next()
        .and_then(|(_, v)| v.as_reference().ok())
}

fn read_bbox_matrix(doc: &Document, sid: ObjectId) -> ([f32; 4], [f32; 6]) {
    let mut bbox = [0.0, 0.0, 1.0, 1.0];
    let mut matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    if let Ok(Object::Stream(s)) = doc.get_object(sid) {
        if let Ok(arr) = s.dict.get(b"BBox").and_then(|o| o.as_array()) {
            for (i, slot) in bbox.iter_mut().enumerate() {
                if let Some(v) = arr.get(i).and_then(number_as_f32) {
                    *slot = v;
                }
            }
        }
        if let Ok(arr) = s.dict.get(b"Matrix").and_then(|o| o.as_array()) {
            for (i, slot) in matrix.iter_mut().enumerate() {
                if let Some(v) = arr.get(i).and_then(number_as_f32) {
                    *slot = v;
                }
            }
        }
    }
    (bbox, matrix)
}

/// Map a form's `BBox`/`Matrix` onto the annotation `Rect` (PDF 12.5.5 algorithm).
fn appearance_cm(bbox: [f32; 4], m: [f32; 6], rect: [f32; 4]) -> [f32; 6] {
    let corners = [
        (bbox[0], bbox[1]),
        (bbox[2], bbox[1]),
        (bbox[2], bbox[3]),
        (bbox[0], bbox[3]),
    ];
    let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (x, y) in corners {
        let tx = m[0] * x + m[2] * y + m[4];
        let ty = m[1] * x + m[3] * y + m[5];
        minx = minx.min(tx);
        maxx = maxx.max(tx);
        miny = miny.min(ty);
        maxy = maxy.max(ty);
    }
    let rx0 = rect[0].min(rect[2]);
    let rx1 = rect[0].max(rect[2]);
    let ry0 = rect[1].min(rect[3]);
    let ry1 = rect[1].max(rect[3]);
    let tw = maxx - minx;
    let th = maxy - miny;
    let sx = if tw.abs() > f32::EPSILON {
        (rx1 - rx0) / tw
    } else {
        1.0
    };
    let sy = if th.abs() > f32::EPSILON {
        (ry1 - ry0) / th
    } else {
        1.0
    };
    [sx, 0.0, 0.0, sy, rx0 - sx * minx, ry0 - sy * miny]
}

/// Insert `value` under a fresh key in the page's `/Resources /XObject`,
/// resolving inline/indirect Resources and XObject sub-dictionaries.
fn add_page_xobject(
    doc: &mut Document,
    page_id: ObjectId,
    value: Object,
) -> Result<String, String> {
    let xobj_indirect: Option<ObjectId> = {
        let res = resources_mut(doc, page_id)?;
        match res.get(b"XObject") {
            Ok(Object::Reference(id)) => Some(*id),
            Ok(Object::Dictionary(_)) => None,
            Ok(_) => return Err("Page /XObject is not a dictionary".to_string()),
            Err(_) => {
                res.set("XObject", Dictionary::new());
                None
            }
        }
    };

    match xobj_indirect {
        Some(id) => {
            let cat = doc
                .get_object_mut(id)
                .map_err(|e| e.to_string())?
                .as_dict_mut()
                .map_err(|_| "Page /XObject reference is not a dictionary".to_string())?;
            let key = unique_key(cat);
            cat.set(key.clone(), value);
            Ok(key)
        }
        None => {
            let cat = resources_mut(doc, page_id)?
                .get_mut(b"XObject")
                .map_err(|e| e.to_string())?
                .as_dict_mut()
                .map_err(|e| e.to_string())?;
            let key = unique_key(cat);
            cat.set(key.clone(), value);
            Ok(key)
        }
    }
}

fn resources_mut(doc: &mut Document, page_id: ObjectId) -> Result<&mut Dictionary, String> {
    {
        let d = doc
            .get_object_mut(page_id)
            .map_err(|e| e.to_string())?
            .as_dict_mut()
            .map_err(|e| e.to_string())?;
        if d.get(b"Resources").is_err() {
            d.set("Resources", Dictionary::new());
        }
    }
    let indirect = match doc
        .get_dictionary(page_id)
        .map_err(|e| e.to_string())?
        .get(b"Resources")
    {
        Ok(Object::Reference(id)) => Some(*id),
        _ => None,
    };
    match indirect {
        Some(id) => doc
            .get_object_mut(id)
            .map_err(|e| e.to_string())?
            .as_dict_mut()
            .map_err(|_| "Page /Resources reference is not a dictionary".to_string()),
        None => doc
            .get_object_mut(page_id)
            .map_err(|e| e.to_string())?
            .as_dict_mut()
            .map_err(|e| e.to_string())?
            .get_mut(b"Resources")
            .map_err(|e| e.to_string())?
            .as_dict_mut()
            .map_err(|e| e.to_string()),
    }
}

fn unique_key(dict: &Dictionary) -> String {
    let mut n = 0u32;
    loop {
        let key = format!("UFFlat{n}");
        if !dict.has(key.as_bytes()) {
            return key;
        }
        n += 1;
    }
}

fn append_page_content(doc: &mut Document, page_id: ObjectId, cid: ObjectId) -> Result<(), String> {
    enum Existing {
        None,
        Items(Vec<Object>),
        Inline(Box<Stream>),
    }
    let existing = {
        let d = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;
        match d.get(b"Contents") {
            Ok(Object::Reference(id)) => Existing::Items(vec![Object::Reference(*id)]),
            Ok(Object::Array(a)) => Existing::Items(a.clone()),
            Ok(Object::Stream(s)) => Existing::Inline(Box::new(s.clone())),
            _ => Existing::None,
        }
    };
    let mut items = match existing {
        Existing::None => Vec::new(),
        Existing::Items(v) => v,
        Existing::Inline(s) => vec![Object::Reference(doc.add_object(*s))],
    };
    items.push(Object::Reference(cid));
    set_object(doc, page_id, "Contents", Object::Array(items))
}

// ---------------------------------------------------------------------------
// String coding
// ---------------------------------------------------------------------------

/// Decode a PDF text string: UTF-16BE when `FE FF`-prefixed, else PDFDocEncoding
/// (approximated by Latin-1, which is exact for the ASCII range these forms use).
fn decode_pdf_text(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let units: Vec<u16> = bytes[2..]
            .chunks(2)
            .map(|c| {
                if c.len() == 2 {
                    u16::from_be_bytes([c[0], c[1]])
                } else {
                    c[0] as u16
                }
            })
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        decode_bytes(bytes)
    }
}

/// Latin-1 decode (each byte is a code point). Used for names and ASCII strings.
fn decode_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// Encode a text value as a UTF-16BE `/V` string (`FE FF` prefix), matching
/// pdf-lib's `PDFHexString.fromText`.
fn encode_v_text(s: &str) -> Object {
    let mut out = vec![0xFE, 0xFF];
    for u in s.encode_utf16() {
        out.extend_from_slice(&u.to_be_bytes());
    }
    Object::String(out, StringFormat::Hexadecimal)
}

fn json_to_string(val: &JsonValue) -> String {
    match val {
        JsonValue::String(s) => s.clone(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

/// JavaScript truthiness for the fill contract (`value ? check() : uncheck()`):
/// `true`, a non-empty string, or a non-zero number is truthy.
fn is_truthy(val: &JsonValue) -> bool {
    match val {
        JsonValue::Bool(b) => *b,
        JsonValue::String(s) => !s.is_empty(),
        JsonValue::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    // -- fixtures -----------------------------------------------------------

    fn fixture(name: &str) -> Vec<u8> {
        let path = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
        std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
    }

    fn acroform() -> Vec<u8> {
        fixture("acroform.pdf")
    }

    /// Parse the list JSON into a vec of serde Values.
    fn list(data: &[u8]) -> Vec<JsonValue> {
        let s = list_form_fields_native(data).unwrap();
        serde_json::from_str::<Vec<JsonValue>>(&s).unwrap()
    }

    fn field<'a>(fields: &'a [JsonValue], name: &str) -> &'a JsonValue {
        fields
            .iter()
            .find(|f| f["name"] == json!(name))
            .unwrap_or_else(|| panic!("field {name} not found"))
    }

    /// Concatenate every stream's decompressed bytes (page content + XObjects).
    fn all_stream_bytes(data: &[u8]) -> Vec<u8> {
        let doc = Document::load_mem(data).unwrap();
        let mut out = Vec::new();
        for (_, obj) in doc.objects.iter() {
            if let Object::Stream(s) = obj {
                out.extend(
                    s.decompressed_content()
                        .unwrap_or_else(|_| s.content.clone()),
                );
            }
        }
        out
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    /// A minimal AcroForm with a pushbutton, a signature, and an FT-less field —
    /// to exercise the button/signature/unknown classification branches.
    fn synth_form() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let f_btn = doc.add_object(dictionary! {
            "FT" => "Btn", "Ff" => FF_PUSHBUTTON, "T" => Object::string_literal("submit"),
        });
        let f_sig = doc.add_object(dictionary! {
            "FT" => "Sig", "T" => Object::string_literal("sig1"),
        });
        let f_unk = doc.add_object(dictionary! { "T" => Object::string_literal("weird") });
        let page = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
        });
        let pages = dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let acro = doc.add_object(dictionary! {
            "Fields" => vec![f_btn.into(), f_sig.into(), f_unk.into()],
        });
        let cat = doc.add_object(dictionary! {
            "Type" => "Catalog", "Pages" => pages_id, "AcroForm" => acro,
        });
        doc.trailer.set("Root", cat);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    /// Raw `/V` bytes of a named terminal field (for encoding assertions).
    fn raw_v(data: &[u8], name: &str) -> Option<Vec<u8>> {
        let doc = Document::load_mem(data).unwrap();
        for (n, fid) in terminal_fields(&doc) {
            if n == name {
                return doc
                    .get_dictionary(fid)
                    .ok()?
                    .get(b"V")
                    .ok()
                    .and_then(|o| o.as_str().ok())
                    .map(|b| b.to_vec());
            }
        }
        None
    }

    // -- list ---------------------------------------------------------------

    #[test]
    fn list_returns_seven_fields() {
        assert_eq!(list(&acroform()).len(), 7);
    }

    #[test]
    fn list_field_names() {
        let f = list(&acroform());
        let mut names: Vec<&str> = f.iter().map(|x| x["name"].as_str().unwrap()).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            vec![
                "color",
                "country",
                "email",
                "name",
                "notes",
                "subscribe",
                "terms"
            ]
        );
    }

    #[test]
    fn list_field_types() {
        let f = list(&acroform());
        assert_eq!(field(&f, "name")["type"], "text");
        assert_eq!(field(&f, "email")["type"], "text");
        assert_eq!(field(&f, "notes")["type"], "text");
        assert_eq!(field(&f, "subscribe")["type"], "checkbox");
        assert_eq!(field(&f, "terms")["type"], "checkbox");
        assert_eq!(field(&f, "color")["type"], "radio");
        assert_eq!(field(&f, "country")["type"], "dropdown");
    }

    #[test]
    fn list_radio_options() {
        let f = list(&acroform());
        assert_eq!(field(&f, "color")["options"], json!(["red", "blue"]));
    }

    #[test]
    fn list_dropdown_options() {
        let f = list(&acroform());
        assert_eq!(
            field(&f, "country")["options"],
            json!(["Ukraine", "Italy", "Other"])
        );
    }

    #[test]
    fn list_initial_values() {
        let f = list(&acroform());
        assert_eq!(field(&f, "name")["value"], json!("Alice"));
        assert_eq!(field(&f, "email")["value"], json!(""));
        assert_eq!(field(&f, "notes")["value"], json!("line one\nline two"));
        assert_eq!(field(&f, "subscribe")["value"], json!(true));
        assert_eq!(field(&f, "terms")["value"], json!(false));
        assert_eq!(field(&f, "color")["value"], json!("red"));
        assert_eq!(field(&f, "country")["value"], json!("Ukraine"));
    }

    #[test]
    fn list_text_fields_have_no_options_key() {
        let f = list(&acroform());
        assert!(field(&f, "name").get("options").is_none());
        assert!(field(&f, "subscribe").get("options").is_none());
    }

    #[test]
    fn list_empty_when_no_acroform() {
        let pdf = crate::util::fixtures::sample(1);
        assert_eq!(list(&pdf).len(), 0);
        assert_eq!(list_form_fields_native(&pdf).unwrap(), "[]");
    }

    #[test]
    fn list_classifies_button_signature_unknown() {
        let f = list(&synth_form());
        assert_eq!(field(&f, "submit")["type"], "button");
        assert_eq!(field(&f, "sig1")["type"], "signature");
        assert_eq!(field(&f, "weird")["type"], "unknown");
        // No value / options for these.
        assert!(field(&f, "submit").get("value").is_none());
        assert!(field(&f, "sig1").get("options").is_none());
    }

    #[test]
    fn list_distinguishes_optionlist_from_dropdown() {
        // Clear the Combo bit on `country` -> it becomes an option list (listbox).
        let mut doc = Document::load_mem(&acroform()).unwrap();
        let country = terminal_fields(&doc)
            .into_iter()
            .find(|(n, _)| n == "country")
            .unwrap()
            .1;
        let ff = doc
            .get_dictionary(country)
            .unwrap()
            .get(b"Ff")
            .unwrap()
            .as_i64()
            .unwrap();
        doc.get_object_mut(country)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Ff", ff & !FF_COMBO);
        let bytes = crate::util::save_compact(doc).unwrap();
        let f = list(&bytes);
        assert_eq!(field(&f, "country")["type"], "optionlist");
        // options preserved
        assert_eq!(
            field(&f, "country")["options"],
            json!(["Ukraine", "Italy", "Other"])
        );
    }

    // -- fill: text ---------------------------------------------------------

    #[test]
    fn fill_text_field() {
        let out = fill_form_native(&acroform(), r#"{"name":"Bob"}"#).unwrap();
        assert_eq!(field(&list(&out), "name")["value"], json!("Bob"));
    }

    #[test]
    fn fill_text_empty_string() {
        let out = fill_form_native(&acroform(), r#"{"name":""}"#).unwrap();
        assert_eq!(field(&list(&out), "name")["value"], json!(""));
    }

    #[test]
    fn fill_text_unicode_round_trips() {
        let out = fill_form_native(&acroform(), r#"{"notes":"тест"}"#).unwrap();
        assert_eq!(field(&list(&out), "notes")["value"], json!("тест"));
        // /V is stored UTF-16BE (FE FF prefix).
        let v = raw_v(&out, "notes").unwrap();
        assert_eq!(&v[..2], &[0xFE, 0xFF]);
    }

    #[test]
    fn fill_does_not_disturb_other_fields() {
        let out = fill_form_native(&acroform(), r#"{"name":"Bob"}"#).unwrap();
        let f = list(&out);
        assert_eq!(field(&f, "country")["value"], json!("Ukraine"));
        assert_eq!(field(&f, "color")["value"], json!("red"));
    }

    #[test]
    fn fill_creates_appearance_stream_with_text() {
        let out = fill_form_native(&acroform(), r#"{"name":"Zaphod"}"#).unwrap();
        assert!(contains(&all_stream_bytes(&out), b"Zaphod"));
    }

    #[test]
    fn fill_sets_needappearances_false() {
        let out = fill_form_native(&acroform(), r#"{"name":"Bob"}"#).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        let acro = doc.get_dictionary(acroform_id(&doc).unwrap()).unwrap();
        assert!(!acro.get(b"NeedAppearances").unwrap().as_bool().unwrap());
    }

    // -- fill: checkbox / radio / choice ------------------------------------

    #[test]
    fn fill_checkbox_true() {
        let out = fill_form_native(&acroform(), r#"{"terms":true}"#).unwrap();
        assert_eq!(field(&list(&out), "terms")["value"], json!(true));
    }

    #[test]
    fn fill_checkbox_false() {
        let out = fill_form_native(&acroform(), r#"{"subscribe":false}"#).unwrap();
        assert_eq!(field(&list(&out), "subscribe")["value"], json!(false));
    }

    #[test]
    fn fill_checkbox_sets_widget_as() {
        let out = fill_form_native(&acroform(), r#"{"terms":true}"#).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        let (_, fid) = terminal_fields(&doc)
            .into_iter()
            .find(|(n, _)| n == "terms")
            .unwrap();
        let w = widget_ids(&doc, doc.get_dictionary(fid).unwrap())[0];
        let as_name = doc
            .get_dictionary(w)
            .unwrap()
            .get(b"AS")
            .unwrap()
            .as_name()
            .unwrap()
            .to_vec();
        assert_ne!(as_name, b"Off");
    }

    #[test]
    fn fill_radio() {
        let out = fill_form_native(&acroform(), r#"{"color":"blue"}"#).unwrap();
        assert_eq!(field(&list(&out), "color")["value"], json!("blue"));
    }

    #[test]
    fn fill_radio_invalid_option_ignored() {
        let out = fill_form_native(&acroform(), r#"{"color":"green"}"#).unwrap();
        // Unchanged: still the original selection.
        assert_eq!(field(&list(&out), "color")["value"], json!("red"));
    }

    #[test]
    fn fill_dropdown() {
        let out = fill_form_native(&acroform(), r#"{"country":"Italy"}"#).unwrap();
        assert_eq!(field(&list(&out), "country")["value"], json!("Italy"));
    }

    #[test]
    fn fill_multiple_fields() {
        let json = r#"{"name":"Carol","email":"c@example.com","terms":true,"country":"Other"}"#;
        let out = fill_form_native(&acroform(), json).unwrap();
        let f = list(&out);
        assert_eq!(field(&f, "name")["value"], json!("Carol"));
        assert_eq!(field(&f, "email")["value"], json!("c@example.com"));
        assert_eq!(field(&f, "terms")["value"], json!(true));
        assert_eq!(field(&f, "country")["value"], json!("Other"));
    }

    #[test]
    fn fill_unknown_field_ignored() {
        let out = fill_form_native(&acroform(), r#"{"nonexistent":"x"}"#).unwrap();
        // No error, field set unchanged.
        assert_eq!(list(&out).len(), 7);
        assert_eq!(field(&list(&out), "name")["value"], json!("Alice"));
    }

    #[test]
    fn fill_bool_into_text_field_coerces() {
        let out = fill_form_native(&acroform(), r#"{"name":true}"#).unwrap();
        assert_eq!(field(&list(&out), "name")["value"], json!("true"));
    }

    #[test]
    fn fill_no_acroform_is_noop_ok() {
        let pdf = crate::util::fixtures::sample(1);
        let out = fill_form_native(&pdf, r#"{"whatever":"x"}"#).unwrap();
        assert!(out.starts_with(b"%PDF-"));
    }

    #[test]
    fn fill_rejects_non_object_json() {
        assert!(fill_form_native(&acroform(), "[1,2,3]").is_err());
        assert!(fill_form_native(&acroform(), "not json").is_err());
    }

    #[test]
    fn fill_output_is_valid_and_reloadable() {
        let out = fill_form_native(&acroform(), r#"{"name":"Bob"}"#).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(Document::load_mem(&out).unwrap().get_pages().len(), 1);
    }

    // -- flatten ------------------------------------------------------------

    #[test]
    fn flatten_removes_acroform() {
        let out = flatten_form_native(&acroform()).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        assert!(doc.catalog().unwrap().get(b"AcroForm").is_err());
    }

    #[test]
    fn flatten_removes_widget_annots() {
        let out = flatten_form_native(&acroform()).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        let any_widget = doc.objects.values().any(|o| {
            o.as_dict()
                .ok()
                .and_then(|d| d.get(b"Subtype").ok())
                .and_then(|s| s.as_name().ok())
                .map(|n| n == b"Widget")
                .unwrap_or(false)
        });
        assert!(!any_widget);
    }

    #[test]
    fn flatten_preserves_page_count() {
        let out = flatten_form_native(&acroform()).unwrap();
        assert_eq!(Document::load_mem(&out).unwrap().get_pages().len(), 1);
    }

    #[test]
    fn flatten_bakes_text_value_into_content() {
        // Base fixture has name = "Alice"; flattening keeps its appearance
        // reachable. pdf-lib wrote that glyph run as the hex string <416C696365>.
        let out = flatten_form_native(&acroform()).unwrap();
        let bytes = all_stream_bytes(&out);
        assert!(contains(&bytes, b"Alice") || contains(&bytes, b"416C696365"));
    }

    #[test]
    fn flatten_emits_do_operator() {
        let out = flatten_form_native(&acroform()).unwrap();
        assert!(contains(&all_stream_bytes(&out), b" Do"));
    }

    #[test]
    fn flatten_filled_form() {
        let filled = fill_form_native(&acroform(), r#"{"name":"Charlie"}"#).unwrap();
        let flat = flatten_form_native(&filled).unwrap();
        let doc = Document::load_mem(&flat).unwrap();
        assert!(doc.catalog().unwrap().get(b"AcroForm").is_err());
        assert!(contains(&all_stream_bytes(&flat), b"Charlie"));
    }

    #[test]
    fn flatten_no_acroform_is_ok() {
        let pdf = crate::util::fixtures::sample(2);
        let out = flatten_form_native(&pdf).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(Document::load_mem(&out).unwrap().get_pages().len(), 2);
    }

    #[test]
    fn flatten_is_idempotent() {
        let once = flatten_form_native(&acroform()).unwrap();
        let twice = flatten_form_native(&once).unwrap();
        let doc = Document::load_mem(&twice).unwrap();
        assert!(doc.catalog().unwrap().get(b"AcroForm").is_err());
        assert_eq!(doc.get_pages().len(), 1);
    }

    // -- encryption guard ---------------------------------------------------

    #[test]
    fn rejects_encrypted_input() {
        let enc = fixture("encrypted_user.pdf");
        assert!(list_form_fields_native(&enc).is_err());
        assert!(fill_form_native(&enc, "{}").is_err());
        assert!(flatten_form_native(&enc).is_err());
    }

    #[test]
    fn works_on_resaved_input() {
        // Mirror the real pipeline tail: once the crypto module has decrypted and
        // re-saved a document, forms must operate on the normalized bytes. We
        // simulate "decrypted then re-saved" with save_compact (lopdf 0.35's own
        // `decrypt` panics on these pdf-lib AES fixtures, so the crypto module —
        // not forms — owns decryption). All three ops survive the round-trip.
        let doc = Document::load_mem(&acroform()).unwrap();
        let resaved = crate::util::save_compact(doc).unwrap();

        assert_eq!(list(&resaved).len(), 7);
        let filled = fill_form_native(&resaved, r#"{"name":"Eve"}"#).unwrap();
        assert_eq!(field(&list(&filled), "name")["value"], json!("Eve"));
        let flat = flatten_form_native(&resaved).unwrap();
        let doc = Document::load_mem(&flat).unwrap();
        assert!(doc.catalog().unwrap().get(b"AcroForm").is_err());
    }

    // -- round-trips --------------------------------------------------------

    #[test]
    fn round_trip_fill_then_list() {
        let json = r#"{"name":"Dana","email":"d@x.io","subscribe":false,"color":"blue"}"#;
        let out = fill_form_native(&acroform(), json).unwrap();
        let f = list(&out);
        assert_eq!(field(&f, "name")["value"], json!("Dana"));
        assert_eq!(field(&f, "email")["value"], json!("d@x.io"));
        assert_eq!(field(&f, "subscribe")["value"], json!(false));
        assert_eq!(field(&f, "color")["value"], json!("blue"));
    }

    proptest! {
        #[test]
        fn prop_text_and_bool_round_trip(
            name in "[A-Za-z0-9 ]{0,20}",
            email in "[A-Za-z0-9 ]{0,20}",
            subscribe in any::<bool>(),
            terms in any::<bool>(),
        ) {
            let values = json!({
                "name": name,
                "email": email,
                "subscribe": subscribe,
                "terms": terms,
            });
            let out = fill_form_native(&acroform(), &values.to_string()).unwrap();
            let f = list(&out);
            prop_assert_eq!(field(&f, "name")["value"].clone(), json!(name));
            prop_assert_eq!(field(&f, "email")["value"].clone(), json!(email));
            prop_assert_eq!(field(&f, "subscribe")["value"].clone(), json!(subscribe));
            prop_assert_eq!(field(&f, "terms")["value"].clone(), json!(terms));
        }

        #[test]
        fn prop_radio_and_dropdown_round_trip(
            color in prop::sample::select(vec!["red", "blue"]),
            country in prop::sample::select(vec!["Ukraine", "Italy", "Other"]),
        ) {
            let values = json!({ "color": color, "country": country });
            let out = fill_form_native(&acroform(), &values.to_string()).unwrap();
            let f = list(&out);
            prop_assert_eq!(field(&f, "color")["value"].clone(), json!(color));
            prop_assert_eq!(field(&f, "country")["value"].clone(), json!(country));
        }

        #[test]
        fn prop_flatten_after_fill_idempotent(
            name in "[A-Za-z0-9 ]{0,20}",
            subscribe in any::<bool>(),
        ) {
            let values = json!({ "name": name, "subscribe": subscribe });
            let filled = fill_form_native(&acroform(), &values.to_string()).unwrap();
            let flat = flatten_form_native(&filled).unwrap();
            let doc = Document::load_mem(&flat).unwrap();
            prop_assert!(doc.catalog().unwrap().get(b"AcroForm").is_err());
            prop_assert_eq!(doc.get_pages().len(), 1);
        }
    }
}
