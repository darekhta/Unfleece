//! Browser-assembler primitives — let the JS tools drop pdf-lib entirely.
//!
//! These three functions cover the remaining "assemble a PDF in the browser"
//! jobs that `imagesToPdf` / `stamp` / `crop` did not already own:
//!
//! - [`image_pages_to_pdf_native`] builds a *fresh* image-only PDF where each
//!   page is exactly `width_pt × height_pt` and a single image fills it. Used by
//!   the redact and raster-compress tools, which render pages to bitmaps and then
//!   need them back as a PDF.
//! - [`add_text_layer_native`] appends an *invisible* (text render mode 3)
//!   Helvetica text layer onto the existing pages of a PDF — the searchable layer
//!   produced by OCR. Existing pages are kept intact; only a content stream and a
//!   font resource are added per touched page.
//! - [`set_crop_boxes_native`] stamps an absolute per-page `CropBox` onto listed
//!   pages — used by auto-crop once the crop rectangle is known.
//!
//! Image embedding mirrors `images_to_pdf.rs` exactly (PNG → FlateDecode
//! DeviceRGB/DeviceGray with a DeviceGray `/SMask` for alpha; JPEG → DCTDecode
//! passthrough for 1/3-component scans, decoded-RGB fallback otherwise). The
//! invisible-text plumbing mirrors `stamp_text.rs` (newline-padded streams, a
//! `q`/`Q` bracket around pre-existing content, collision-free resource keys).
//! Everything is `wasm32`-safe: no time, threads, filesystem or RNG.

use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};

use crate::pack::PackReader;
use crate::stamp_text::encode_winansi;
use crate::util::{materialize_inherited_page_attrs, save_compact};

const IMAGE_PAGES_MAGIC: &[u8; 4] = b"UFAP";
const TEXT_LAYER_MAGIC: &[u8; 4] = b"UFTL";
const CROP_BOXES_MAGIC: &[u8; 4] = b"UFCB";

// ===========================================================================
// 1) image_pages_to_pdf_native — fresh image-only PDF, page == image size
// ===========================================================================

/// Build a new image-only PDF: one page per pack entry, each page exactly
/// `width_pt × height_pt`, with the image scaled to fill the whole page
/// (`q {w} 0 0 {h} 0 0 cm /Im0 Do Q`).
///
/// Pack format (`UFAP`), little-endian:
/// - `UFAP` magic
/// - u32 page count
/// - per page: f32 `width_pt`, f32 `height_pt`, u8 kind (0 = PNG, 1 = JPEG),
///   u32 image byte length, image bytes
///
/// PNG/JPEG embedding matches `images_to_pdf.rs` byte-for-byte (DCT passthrough
/// for JPEG, flate for PNG, DeviceGray `/SMask` for PNG alpha). Page dimensions
/// must be positive and finite.
pub fn image_pages_to_pdf_native(pack: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = PackReader::new(pack);
    reader.expect_magic(IMAGE_PAGES_MAGIC)?;
    let count = reader.read_u32()? as usize;
    if count == 0 {
        return Err("No pages provided".to_string());
    }

    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut kids: Vec<Object> = Vec::with_capacity(count);

    for _ in 0..count {
        let width = reader.read_f32()?;
        let height = reader.read_f32()?;
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err("Page dimensions must be positive".to_string());
        }
        let kind = reader.read_u8()?;
        let bytes = reader.read_bytes()?;
        let image = match kind {
            0 => embed_png(&mut doc, bytes)?,
            1 => embed_jpeg(&mut doc, bytes)?,
            _ => return Err("Unknown image kind in input pack".to_string()),
        };

        let content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![
                        width.into(),
                        0f32.into(),
                        0f32.into(),
                        height.into(),
                        0f32.into(),
                        0f32.into(),
                    ],
                ),
                Operation::new("Do", vec!["Im0".into()]),
                Operation::new("Q", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            content.encode().map_err(|e| e.to_string())?,
        ));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image } },
            "MediaBox" => vec![0f32.into(), 0f32.into(), width.into(), height.into()],
        });
        kids.push(page_id.into());
    }
    reader.expect_done()?;

    let pages_dict = dictionary! {
        "Type" => "Pages",
        "Kids" => kids,
        "Count" => count as i64,
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);

    save_compact(doc).map_err(|e| e.to_string())
}

// ===========================================================================
// 2) add_text_layer_native — invisible Helvetica OCR layer on existing pages
// ===========================================================================

struct Span {
    x: f32,
    y: f32,
    font_size: f32,
    text: String,
}

struct PageSpans {
    page_index: usize,
    spans: Vec<Span>,
}

/// Append an invisible (text render mode 3) Helvetica text layer onto the
/// existing pages of `data`. Coordinates are already in PDF points (bottom-left
/// origin). Pages not listed in the pack are left untouched.
///
/// Pack format (`UFTL`), little-endian:
/// - `UFTL` magic
/// - u32 page count
/// - per page: u32 `page_index`, u32 span count, then per span: f32 `x`, f32
///   `y`, f32 `font_size`, str `text` (u32 length + UTF-8 bytes)
///
/// Each span emits `BT 3 Tr /HelvOCR {size} Tf {x} {y} Td (text) Tj ET`; the
/// whole layer is wrapped in `q`/`Q`. A Helvetica Type1 / WinAnsiEncoding font is
/// registered under a collision-free key in every touched page's
/// `/Resources /Font`. Text is WinAnsi-encoded; characters outside WinAnsi become
/// `?` (via [`encode_winansi`]). Existing `/Contents` are turned into an array
/// with the new layer stream last.
pub fn add_text_layer_native(data: &[u8], pack: &[u8]) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    let pages: Vec<ObjectId> = doc.page_iter().collect();

    let mut reader = PackReader::new(pack);
    reader.expect_magic(TEXT_LAYER_MAGIC)?;
    let page_count = reader.read_u32()? as usize;

    let mut entries: Vec<PageSpans> = Vec::with_capacity(page_count);
    for _ in 0..page_count {
        let page_index = reader.read_u32()? as usize;
        if page_index >= pages.len() {
            return Err(format!(
                "Page index {page_index} is out of range; document has {} pages",
                pages.len()
            ));
        }
        let span_count = reader.read_u32()? as usize;
        let mut spans = Vec::with_capacity(span_count);
        for _ in 0..span_count {
            let x = reader.read_f32()?;
            let y = reader.read_f32()?;
            let font_size = reader.read_f32()?;
            let text = reader.read_str()?.to_string();
            spans.push(Span {
                x,
                y,
                font_size,
                text,
            });
        }
        entries.push(PageSpans { page_index, spans });
    }
    reader.expect_done()?;

    // One shared Helvetica font object, referenced from each touched page.
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });
    let (push_id, pop_id) = wrap_stream_ids(&mut doc)?;

    for entry in &entries {
        if entry.spans.is_empty() {
            continue;
        }
        let page_id = pages[entry.page_index];
        materialize_inherited_page_attrs(&mut doc, page_id).map_err(|e| e.to_string())?;
        let font_key = add_resource_entry(
            &mut doc,
            page_id,
            "Font",
            "HelvOCR",
            Object::Reference(font_id),
        )?;

        let mut ops = vec![Operation::new("q", vec![])];
        for span in &entry.spans {
            let encoded = encode_winansi(&span.text);
            ops.extend([
                Operation::new("BT", vec![]),
                Operation::new("Tr", vec![Object::Integer(3)]),
                Operation::new(
                    "Tf",
                    vec![
                        Object::Name(font_key.as_bytes().to_vec()),
                        Object::Real(span.font_size),
                    ],
                ),
                Operation::new("Td", vec![Object::Real(span.x), Object::Real(span.y)]),
                Operation::new("Tj", vec![Object::string_literal(encoded)]),
                Operation::new("ET", vec![]),
            ]);
        }
        ops.push(Operation::new("Q", vec![]));

        let layer_id = add_content_stream(&mut doc, ops)?;
        append_to_page_contents(&mut doc, page_id, layer_id, push_id, pop_id)?;
    }

    save_compact(doc).map_err(|e| e.to_string())
}

// ===========================================================================
// 3) set_crop_boxes_native — absolute per-page CropBox on listed pages
// ===========================================================================

/// Set a per-page `CropBox` (absolute PDF coords `[x, y, x + w, y + h]`) on the
/// listed pages; unlisted pages are unchanged.
///
/// Pack format (`UFCB`), little-endian:
/// - `UFCB` magic
/// - u32 entry count
/// - per entry: u32 `page_index`, f32 `x`, f32 `y`, f32 `w`, f32 `h`
///
/// `w`/`h` must be positive and all four numbers finite; the MediaBox is left
/// untouched.
pub fn set_crop_boxes_native(data: &[u8], pack: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = PackReader::new(pack);
    reader.expect_magic(CROP_BOXES_MAGIC)?;
    let entry_count = reader.read_u32()? as usize;

    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    let pages: Vec<ObjectId> = doc.page_iter().collect();

    let mut entries: Vec<(usize, f32, f32, f32, f32)> = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let page_index = reader.read_u32()? as usize;
        let x = reader.read_f32()?;
        let y = reader.read_f32()?;
        let w = reader.read_f32()?;
        let h = reader.read_f32()?;
        if page_index >= pages.len() {
            return Err(format!(
                "Page index {page_index} is out of range; document has {} pages",
                pages.len()
            ));
        }
        if !(x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite()) {
            return Err("Crop box values must be finite numbers".to_string());
        }
        if w <= 0.0 || h <= 0.0 {
            return Err("Crop box width and height must be positive".to_string());
        }
        entries.push((page_index, x, y, w, h));
    }
    reader.expect_done()?;

    for (page_index, x, y, w, h) in entries {
        let page_id = pages[page_index];
        let crop_box = vec![
            Object::Real(x),
            Object::Real(y),
            Object::Real(x + w),
            Object::Real(y + h),
        ];
        doc.get_object_mut(page_id)
            .and_then(|obj| obj.as_dict_mut())
            .map_err(|e| e.to_string())?
            .set("CropBox", crop_box);
    }

    save_compact(doc).map_err(|e| e.to_string())
}

// ===========================================================================
// Image embedding — local copy of images_to_pdf.rs's private helpers
// ===========================================================================

/// Embed a PNG as a flate-compressed image XObject; returns its id.
///
/// Alpha becomes a DeviceGray `/SMask`; opaque grayscale stays DeviceGray;
/// everything else is DeviceRGB. Identical to `images_to_pdf::embed_png`.
fn embed_png(doc: &mut Document, bytes: &[u8]) -> Result<ObjectId, String> {
    use image::GenericImageView;

    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|e| format!("Invalid image data: {e}"))?;
    let (width, height) = img.dimensions();

    let id = if img.color().has_alpha() {
        let rgba = img.into_rgba8();
        let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
        let mut alpha = Vec::with_capacity(width as usize * height as usize);
        for px in rgba.pixels() {
            rgb.extend_from_slice(&px.0[..3]);
            alpha.push(px.0[3]);
        }
        let smask_id = doc.add_object(flate_image_stream(
            width,
            height,
            "DeviceGray",
            &alpha,
            None,
        )?);
        doc.add_object(flate_image_stream(
            width,
            height,
            "DeviceRGB",
            &rgb,
            Some(smask_id),
        )?)
    } else if matches!(img.color(), image::ColorType::L8 | image::ColorType::L16) {
        doc.add_object(flate_image_stream(
            width,
            height,
            "DeviceGray",
            img.into_luma8().as_raw(),
            None,
        )?)
    } else {
        doc.add_object(flate_image_stream(
            width,
            height,
            "DeviceRGB",
            img.into_rgb8().as_raw(),
            None,
        )?)
    };
    Ok(id)
}

/// Embed a JPEG, preferring DCT passthrough; returns its id. Identical to
/// `images_to_pdf::embed_jpeg`.
fn embed_jpeg(doc: &mut Document, bytes: &[u8]) -> Result<ObjectId, String> {
    use image::GenericImageView;

    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)
        .map_err(|e| format!("Invalid image data: {e}"))?;
    let (width, height) = img.dimensions();

    let id = match jpeg_component_count(bytes) {
        Some(1) => doc.add_object(dct_image_stream(width, height, "DeviceGray", bytes)),
        Some(3) => doc.add_object(dct_image_stream(width, height, "DeviceRGB", bytes)),
        _ => doc.add_object(flate_image_stream(
            width,
            height,
            "DeviceRGB",
            img.into_rgb8().as_raw(),
            None,
        )?),
    };
    Ok(id)
}

/// Image XObject holding raw samples, zlib-compressed (`/FlateDecode`).
fn flate_image_stream(
    width: u32,
    height: u32,
    color_space: &str,
    samples: &[u8],
    smask: Option<ObjectId>,
) -> Result<Stream, String> {
    let mut dict = dictionary! {
        "Type" => "XObject",
        "Subtype" => "Image",
        "Width" => width as i64,
        "Height" => height as i64,
        "ColorSpace" => color_space,
        "BitsPerComponent" => 8,
        "Filter" => "FlateDecode",
    };
    if let Some(id) = smask {
        dict.set("SMask", id);
    }
    Ok(Stream::new(dict, flate_compress(samples)?))
}

/// Image XObject carrying the original JPEG bytes (`/DCTDecode` passthrough).
fn dct_image_stream(width: u32, height: u32, color_space: &str, jpeg: &[u8]) -> Stream {
    Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => width as i64,
            "Height" => height as i64,
            "ColorSpace" => color_space,
            "BitsPerComponent" => 8,
            "Filter" => "DCTDecode",
        },
        jpeg.to_vec(),
    )
}

fn flate_compress(data: &[u8]) -> Result<Vec<u8>, String> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())
}

/// Number of color components from the JPEG's SOF header, if parseable.
/// Identical to `images_to_pdf::jpeg_component_count`.
fn jpeg_component_count(bytes: &[u8]) -> Option<u8> {
    let mut i = 2; // skip SOI (FFD8)
    while i + 3 < bytes.len() {
        if bytes[i] != 0xFF {
            return None;
        }
        let marker = bytes[i + 1];
        if marker == 0xFF {
            i += 1; // fill byte
            continue;
        }
        if marker == 0x01 || (0xD0..=0xD9).contains(&marker) {
            i += 2; // standalone marker (TEM/RSTn/SOI/EOI), no length field
            continue;
        }
        if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            // SOFn: length(2) precision(1) height(2) width(2) components(1)
            return bytes.get(i + 9).copied();
        }
        if marker == 0xDA {
            return None; // start of scan — no SOF seen, give up
        }
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        i += 2 + len;
    }
    None
}

// ===========================================================================
// Content-stream + resource plumbing — local copy of stamp_text.rs's helpers
// ===========================================================================

/// Encode content operations to stream bytes, padded with leading/trailing
/// newlines so adjacent streams in a `/Contents` array cannot token-fuse.
fn content_bytes(operations: Vec<Operation>) -> Result<Vec<u8>, String> {
    let body = Content { operations }.encode().map_err(|e| e.to_string())?;
    let mut bytes = Vec::with_capacity(body.len() + 2);
    bytes.push(b'\n');
    bytes.extend(body);
    bytes.push(b'\n');
    Ok(bytes)
}

/// Add a content stream object holding `operations`; returns its id.
fn add_content_stream(doc: &mut Document, operations: Vec<Operation>) -> Result<ObjectId, String> {
    Ok(doc.add_object(Stream::new(dictionary! {}, content_bytes(operations)?)))
}

/// Shared `q` / `Q` wrapper streams used to bracket pre-existing page content.
fn wrap_stream_ids(doc: &mut Document) -> Result<(ObjectId, ObjectId), String> {
    let push = content_bytes(vec![Operation::new("q", vec![])])?;
    let pop = content_bytes(vec![Operation::new("Q", vec![])])?;
    Ok((
        doc.add_object(Stream::new(dictionary! {}, push)),
        doc.add_object(Stream::new(dictionary! {}, pop)),
    ))
}

/// Where a page's `/Resources` dictionary lives.
enum ResourcesLoc {
    Inline,
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

/// Insert `value` under a fresh key in the `category` (e.g. `Font`)
/// sub-dictionary of the page's Resources, following indirect references and
/// creating missing dictionaries. Returns the chosen resource key.
fn add_resource_entry(
    doc: &mut Document,
    page_id: ObjectId,
    category: &str,
    base_key: &str,
    value: Object,
) -> Result<String, String> {
    let res_loc = locate_resources(doc, page_id)?;

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

/// Append `layer_id` to the page's `/Contents`, first bracketing any pre-existing
/// content with the shared `q`/`Q` wrapper streams and normalizing `/Contents`
/// into an array.
fn append_to_page_contents(
    doc: &mut Document,
    page_id: ObjectId,
    layer_id: ObjectId,
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
        vec![Object::Reference(layer_id)]
    } else {
        let mut v = Vec::with_capacity(items.len() + 3);
        v.push(Object::Reference(push_id));
        v.extend(items);
        v.push(Object::Reference(pop_id));
        v.push(Object::Reference(layer_id));
        v
    };
    doc.get_object_mut(page_id)
        .map_err(|e| e.to_string())?
        .as_dict_mut()
        .map_err(|e| e.to_string())?
        .set("Contents", contents);
    Ok(())
}

// ===========================================================================
// tests
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackWriter;
    use crate::util::fixtures::{sample, PNG_1X1};
    use std::io::{Cursor, Read};

    // -- shared helpers -----------------------------------------------------

    fn n_pages(data: &[u8]) -> usize {
        Document::load_mem(data).unwrap().get_pages().len()
    }

    fn media_box(data: &[u8], n: usize) -> [f32; 4] {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        crate::util::effective_media_box(&doc, pages[n]).unwrap()
    }

    fn page_content(data: &[u8], n: usize) -> Vec<u8> {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        doc.get_page_content(pages[n]).unwrap()
    }

    fn page_content_str(data: &[u8], n: usize) -> String {
        String::from_utf8_lossy(&page_content(data, n)).to_string()
    }

    fn assert_close(actual: f32, expected: f32, what: &str) {
        assert!(
            (actual - expected).abs() < 0.01,
            "{what}: expected {expected}, got {actual}"
        );
    }

    fn inflate(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        flate2::read::ZlibDecoder::new(data)
            .read_to_end(&mut out)
            .unwrap();
        out
    }

    // -- image builders -----------------------------------------------------

    fn encode_img(img: image::DynamicImage, format: image::ImageFormat) -> Vec<u8> {
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), format).unwrap();
        buf
    }

    fn rgb_png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 251) as u8, (y % 241) as u8, 9])
        });
        encode_img(image::DynamicImage::ImageRgb8(img), image::ImageFormat::Png)
    }

    fn rgba_png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_fn(w, h, |x, y| {
            image::Rgba([10, 20, 30, (40 + x * 10 + y) as u8])
        });
        encode_img(
            image::DynamicImage::ImageRgba8(img),
            image::ImageFormat::Png,
        )
    }

    fn gray_jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = image::GrayImage::from_pixel(w, h, image::Luma([130]));
        encode_img(
            image::DynamicImage::ImageLuma8(img),
            image::ImageFormat::Jpeg,
        )
    }

    fn rgb_jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([200, 100, 50]));
        encode_img(
            image::DynamicImage::ImageRgb8(img),
            image::ImageFormat::Jpeg,
        )
    }

    /// Build a `UFAP` pack from `(width_pt, height_pt, kind, bytes)` entries.
    fn image_pack(pages: &[(f32, f32, u8, &[u8])]) -> Vec<u8> {
        let mut w = PackWriter::new(IMAGE_PAGES_MAGIC).u32(pages.len() as u32);
        for (width, height, kind, bytes) in pages {
            w = w.f32(*width).f32(*height).u8(*kind).bytes(bytes);
        }
        w.finish()
    }

    /// The page's `/Im0` image XObject as (dict, raw stream content).
    fn image_xobject(data: &[u8], n: usize) -> (Dictionary, Vec<u8>) {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let page = doc.get_dictionary(pages[n]).unwrap();
        let resources = page.get(b"Resources").unwrap().as_dict().unwrap();
        let xobjects = resources.get(b"XObject").unwrap().as_dict().unwrap();
        let id = xobjects.get(b"Im0").unwrap().as_reference().unwrap();
        let stream = doc.get_object(id).unwrap().as_stream().unwrap();
        (stream.dict.clone(), stream.content.clone())
    }

    fn name_of(dict: &Dictionary, key: &[u8]) -> String {
        String::from_utf8(dict.get(key).unwrap().as_name().unwrap().to_vec()).unwrap()
    }

    fn cm_matrix(data: &[u8], n: usize) -> [f32; 6] {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let content = Content::decode(&doc.get_page_content(pages[n]).unwrap()).unwrap();
        let op = content
            .operations
            .iter()
            .find(|op| op.operator == "cm")
            .unwrap();
        let mut out = [0f32; 6];
        for (i, operand) in op.operands.iter().enumerate() {
            out[i] = crate::util::number_as_f32(operand).unwrap();
        }
        out
    }

    // ======================================================================
    // image_pages_to_pdf_native
    // ======================================================================

    #[test]
    fn image_pages_builds_pdf_with_one_page_per_entry() {
        let out = image_pages_to_pdf_native(&image_pack(&[
            (100.0, 200.0, 0, &rgb_png(10, 20)),
            (300.0, 150.0, 1, &rgb_jpeg(30, 15)),
        ]))
        .unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(n_pages(&out), 2);
    }

    #[test]
    fn image_pages_media_box_matches_requested_points() {
        let out =
            image_pages_to_pdf_native(&image_pack(&[(123.0, 456.0, 0, &rgb_png(4, 4))])).unwrap();
        assert_eq!(media_box(&out, 0), [0.0, 0.0, 123.0, 456.0]);
    }

    #[test]
    fn image_pages_content_fills_page_with_do() {
        let out =
            image_pages_to_pdf_native(&image_pack(&[(80.0, 60.0, 0, &rgb_png(8, 6))])).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let ops: Vec<String> = Content::decode(&doc.get_page_content(pages[0]).unwrap())
            .unwrap()
            .operations
            .iter()
            .map(|o| o.operator.clone())
            .collect();
        assert_eq!(ops, ["q", "cm", "Do", "Q"]);
        // Image scaled to the full page (width_pt, height_pt), not pixel size.
        assert_eq!(cm_matrix(&out, 0), [80.0, 0.0, 0.0, 60.0, 0.0, 0.0]);
    }

    #[test]
    fn image_pages_jpeg_passes_through_as_dctdecode() {
        let jpeg = rgb_jpeg(16, 8);
        let out = image_pages_to_pdf_native(&image_pack(&[(200.0, 100.0, 1, &jpeg)])).unwrap();
        let (dict, content) = image_xobject(&out, 0);
        assert_eq!(name_of(&dict, b"Filter"), "DCTDecode");
        assert_eq!(name_of(&dict, b"ColorSpace"), "DeviceRGB");
        assert_eq!(dict.get(b"Width").unwrap().as_i64().unwrap(), 16);
        assert_eq!(dict.get(b"Height").unwrap().as_i64().unwrap(), 8);
        assert_eq!(content, jpeg, "original JPEG bytes pass through unchanged");
    }

    #[test]
    fn image_pages_grayscale_jpeg_uses_devicegray_passthrough() {
        let jpeg = gray_jpeg(8, 8);
        assert_eq!(jpeg_component_count(&jpeg), Some(1));
        let out = image_pages_to_pdf_native(&image_pack(&[(50.0, 50.0, 1, &jpeg)])).unwrap();
        let (dict, content) = image_xobject(&out, 0);
        assert_eq!(name_of(&dict, b"Filter"), "DCTDecode");
        assert_eq!(name_of(&dict, b"ColorSpace"), "DeviceGray");
        assert_eq!(content, jpeg);
    }

    #[test]
    fn image_pages_opaque_png_is_flate_devicergb_without_smask() {
        let png = rgb_png(5, 4);
        let out = image_pages_to_pdf_native(&image_pack(&[(50.0, 40.0, 0, &png)])).unwrap();
        let (dict, content) = image_xobject(&out, 0);
        assert_eq!(name_of(&dict, b"Filter"), "FlateDecode");
        assert_eq!(name_of(&dict, b"ColorSpace"), "DeviceRGB");
        assert!(!dict.has(b"SMask"));
        let raw = image::load_from_memory(&png).unwrap().into_rgb8();
        assert_eq!(inflate(&content), *raw.as_raw());
    }

    #[test]
    fn image_pages_alpha_png_gets_devicegray_smask() {
        let out =
            image_pages_to_pdf_native(&image_pack(&[(30.0, 20.0, 0, &rgba_png(3, 2))])).unwrap();
        let (dict, _) = image_xobject(&out, 0);
        assert_eq!(name_of(&dict, b"Filter"), "FlateDecode");
        assert_eq!(name_of(&dict, b"ColorSpace"), "DeviceRGB");
        let doc = Document::load_mem(&out).unwrap();
        let smask_id = dict.get(b"SMask").unwrap().as_reference().unwrap();
        let smask = doc.get_object(smask_id).unwrap().as_stream().unwrap();
        assert_eq!(name_of(&smask.dict, b"ColorSpace"), "DeviceGray");
        // rgba_png alpha = 40 + x*10 + y
        assert_eq!(inflate(&smask.content), vec![40, 50, 60, 41, 51, 61]);
    }

    #[test]
    fn image_pages_fixture_png_takes_smask_path() {
        // PNG_1X1 is grayscale+alpha (LA8) -> SMask path with DeviceRGB color.
        let out = image_pages_to_pdf_native(&image_pack(&[(10.0, 10.0, 0, PNG_1X1)])).unwrap();
        let (dict, _) = image_xobject(&out, 0);
        assert!(dict.has(b"SMask"));
        assert_eq!(media_box(&out, 0), [0.0, 0.0, 10.0, 10.0]);
    }

    #[test]
    fn image_pages_keeps_pack_order_for_mixed_pages() {
        let out = image_pages_to_pdf_native(&image_pack(&[
            (10.0, 11.0, 0, &rgb_png(2, 2)),
            (20.0, 21.0, 1, &rgb_jpeg(4, 4)),
            (30.0, 31.0, 0, &rgba_png(2, 3)),
        ]))
        .unwrap();
        assert_eq!(n_pages(&out), 3);
        assert_close(media_box(&out, 0)[2], 10.0, "page 0 width");
        assert_close(media_box(&out, 1)[2], 20.0, "page 1 width");
        assert_close(media_box(&out, 2)[3], 31.0, "page 2 height");
    }

    #[test]
    fn image_pages_rejects_empty_pack() {
        let empty = PackWriter::new(IMAGE_PAGES_MAGIC).u32(0).finish();
        assert_eq!(
            image_pages_to_pdf_native(&empty).unwrap_err(),
            "No pages provided"
        );
    }

    #[test]
    fn image_pages_rejects_unknown_kind() {
        let bad = image_pack(&[(10.0, 10.0, 7, PNG_1X1)]);
        assert_eq!(
            image_pages_to_pdf_native(&bad).unwrap_err(),
            "Unknown image kind in input pack"
        );
    }

    #[test]
    fn image_pages_rejects_wrong_magic() {
        let bad = PackWriter::new(b"NOPE")
            .u32(1)
            .f32(10.0)
            .f32(10.0)
            .u8(0)
            .bytes(PNG_1X1)
            .finish();
        assert_eq!(
            image_pages_to_pdf_native(&bad).unwrap_err(),
            "Invalid input pack"
        );
    }

    #[test]
    fn image_pages_rejects_truncated_pack() {
        // declares two pages, supplies one
        let truncated = PackWriter::new(IMAGE_PAGES_MAGIC)
            .u32(2)
            .f32(10.0)
            .f32(10.0)
            .u8(0)
            .bytes(PNG_1X1)
            .finish();
        assert_eq!(
            image_pages_to_pdf_native(&truncated).unwrap_err(),
            "Input pack ended early"
        );
    }

    #[test]
    fn image_pages_rejects_trailing_data() {
        let mut padded = image_pack(&[(10.0, 10.0, 0, PNG_1X1)]);
        padded.push(0);
        assert_eq!(
            image_pages_to_pdf_native(&padded).unwrap_err(),
            "Input pack has trailing data"
        );
    }

    #[test]
    fn image_pages_rejects_bad_image_bytes() {
        let err =
            image_pages_to_pdf_native(&image_pack(&[(10.0, 10.0, 0, b"garbage")])).unwrap_err();
        assert!(err.starts_with("Invalid image data:"), "got: {err}");
    }

    #[test]
    fn image_pages_rejects_non_positive_dimensions() {
        let zero = image_pack(&[(0.0, 100.0, 0, &rgb_png(4, 4))]);
        assert_eq!(
            image_pages_to_pdf_native(&zero).unwrap_err(),
            "Page dimensions must be positive"
        );
        let negative = image_pack(&[(100.0, -5.0, 0, &rgb_png(4, 4))]);
        assert_eq!(
            image_pages_to_pdf_native(&negative).unwrap_err(),
            "Page dimensions must be positive"
        );
    }

    // ======================================================================
    // add_text_layer_native
    // ======================================================================

    /// `(x, y, font_size, text)` for one OCR span.
    type SpanArg<'a> = (f32, f32, f32, &'a str);
    /// `(page_index, spans)` for one page of the `UFTL` pack.
    type PageArg<'a> = (u32, Vec<SpanArg<'a>>);

    /// Build a `UFTL` pack: list of `(page_index, [(x, y, size, text)])`.
    fn text_pack(pages: &[PageArg]) -> Vec<u8> {
        let mut w = PackWriter::new(TEXT_LAYER_MAGIC).u32(pages.len() as u32);
        for (page_index, spans) in pages {
            w = w.u32(*page_index).u32(spans.len() as u32);
            for (x, y, size, text) in spans {
                w = w.f32(*x).f32(*y).f32(*size).str(text);
            }
        }
        w.finish()
    }

    /// The Font sub-dictionary reachable from page `n`'s Resources.
    fn font_dict(data: &[u8], n: usize) -> (Document, Dictionary) {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let page = doc.get_dictionary(pages[n]).unwrap();
        let res = match page.get(b"Resources").unwrap() {
            Object::Reference(id) => doc.get_dictionary(*id).unwrap().clone(),
            Object::Dictionary(d) => d.clone(),
            other => panic!("unexpected Resources: {other:?}"),
        };
        let font = match res.get(b"Font").unwrap() {
            Object::Reference(id) => doc.get_dictionary(*id).unwrap().clone(),
            Object::Dictionary(d) => d.clone(),
            other => panic!("unexpected Font: {other:?}"),
        };
        (doc, font)
    }

    #[test]
    fn text_layer_draws_invisible_text_on_listed_page() {
        let out = add_text_layer_native(
            &sample(2),
            &text_pack(&[(0, vec![(72.0, 700.0, 12.0, "Hello")])]),
        )
        .unwrap();
        let content = page_content_str(&out, 0);
        assert!(content.contains("3 Tr"), "render mode 3: {content}");
        assert!(content.contains("Hello"), "text drawn: {content}");
        assert!(content.contains("BT"));
        assert!(content.contains("ET"));
    }

    #[test]
    fn text_layer_registers_winansi_helvetica_font() {
        let out = add_text_layer_native(
            &sample(2),
            &text_pack(&[(0, vec![(10.0, 10.0, 8.0, "Hi")])]),
        )
        .unwrap();
        let (doc, font) = font_dict(&out, 0);
        let key = font
            .iter()
            .map(|(k, _)| String::from_utf8_lossy(k).to_string())
            .find(|k| k.starts_with("HelvOCR"))
            .expect("HelvOCR font key present");
        let entry = font.get(key.as_bytes()).unwrap();
        let fd = match entry {
            Object::Reference(id) => doc.get_dictionary(*id).unwrap().clone(),
            Object::Dictionary(d) => d.clone(),
            other => panic!("unexpected font entry: {other:?}"),
        };
        assert_eq!(name_of(&fd, b"Subtype"), "Type1");
        assert_eq!(name_of(&fd, b"BaseFont"), "Helvetica");
        assert_eq!(name_of(&fd, b"Encoding"), "WinAnsiEncoding");
    }

    #[test]
    fn text_layer_leaves_unlisted_pages_untouched() {
        let out = add_text_layer_native(
            &sample(2),
            &text_pack(&[(0, vec![(10.0, 10.0, 8.0, "Only0")])]),
        )
        .unwrap();
        assert!(page_content_str(&out, 0).contains("Only0"));
        let page1 = page_content_str(&out, 1);
        assert!(!page1.contains("3 Tr"), "page 1 untouched: {page1}");
        assert!(!page1.contains("Only0"));
    }

    #[test]
    fn text_layer_preserves_page_count_and_reparses() {
        let out =
            add_text_layer_native(&sample(3), &text_pack(&[(1, vec![(5.0, 5.0, 10.0, "x")])]))
                .unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(n_pages(&out), 3);
        // sanity: reload twice
        let again =
            add_text_layer_native(&out, &text_pack(&[(2, vec![(5.0, 5.0, 10.0, "y")])])).unwrap();
        assert_eq!(n_pages(&again), 3);
    }

    #[test]
    fn text_layer_makes_contents_an_array() {
        let out = add_text_layer_native(&sample(1), &text_pack(&[(0, vec![(1.0, 2.0, 9.0, "z")])]))
            .unwrap();
        let doc = Document::load_mem(&out).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let contents = doc
            .get_dictionary(pages[0])
            .unwrap()
            .get(b"Contents")
            .unwrap();
        assert!(matches!(contents, Object::Array(_)), "Contents is an array");
    }

    #[test]
    fn text_layer_supports_multiple_spans_one_page() {
        let out = add_text_layer_native(
            &sample(1),
            &text_pack(&[(
                0,
                vec![(10.0, 100.0, 10.0, "alpha"), (10.0, 80.0, 10.0, "beta")],
            )]),
        )
        .unwrap();
        let content = page_content_str(&out, 0);
        assert!(content.contains("alpha"));
        assert!(content.contains("beta"));
        assert_eq!(content.matches("3 Tr").count(), 2, "one Tr per span");
    }

    #[test]
    fn text_layer_replaces_non_winansi_with_question_mark() {
        // U+03A9 (Greek capital omega) is outside WinAnsi.
        let out = add_text_layer_native(
            &sample(1),
            &text_pack(&[(0, vec![(5.0, 5.0, 10.0, "A\u{03A9}B")])]),
        )
        .unwrap();
        let content = page_content_str(&out, 0);
        assert!(content.contains("A?B"), "omega -> '?': {content}");
    }

    #[test]
    fn text_layer_rejects_out_of_range_page_index() {
        let err = add_text_layer_native(&sample(2), &text_pack(&[(5, vec![(1.0, 1.0, 8.0, "x")])]))
            .unwrap_err();
        assert!(err.contains("out of range"), "got: {err}");
    }

    #[test]
    fn text_layer_rejects_wrong_magic() {
        let bad = PackWriter::new(b"NOPE").u32(0).finish();
        assert_eq!(
            add_text_layer_native(&sample(1), &bad).unwrap_err(),
            "Invalid input pack"
        );
    }

    #[test]
    fn text_layer_rejects_truncated_pack() {
        // declares a span but supplies no span bytes
        let truncated = PackWriter::new(TEXT_LAYER_MAGIC)
            .u32(1)
            .u32(0)
            .u32(1)
            .finish();
        assert_eq!(
            add_text_layer_native(&sample(1), &truncated).unwrap_err(),
            "Input pack ended early"
        );
    }

    #[test]
    fn text_layer_rejects_invalid_pdf() {
        assert!(add_text_layer_native(b"not a pdf", &text_pack(&[])).is_err());
    }

    #[test]
    fn text_layer_empty_pack_is_noop_passthrough() {
        let out = add_text_layer_native(&sample(2), &text_pack(&[])).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(n_pages(&out), 2);
        assert!(!page_content_str(&out, 0).contains("3 Tr"));
    }

    // ======================================================================
    // set_crop_boxes_native
    // ======================================================================

    /// Build a `UFCB` pack from `(page_index, x, y, w, h)` entries.
    fn crop_pack(entries: &[(u32, f32, f32, f32, f32)]) -> Vec<u8> {
        let mut w = PackWriter::new(CROP_BOXES_MAGIC).u32(entries.len() as u32);
        for (page_index, x, y, width, height) in entries {
            w = w.u32(*page_index).f32(*x).f32(*y).f32(*width).f32(*height);
        }
        w.finish()
    }

    fn crop_box(data: &[u8], n: usize) -> Option<[f32; 4]> {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let arr = doc
            .get_dictionary(pages[n])
            .unwrap()
            .get(b"CropBox")
            .ok()?
            .as_array()
            .unwrap()
            .clone();
        let mut out = [0f32; 4];
        for (i, obj) in arr.iter().enumerate() {
            out[i] = crate::util::number_as_f32(obj).unwrap();
        }
        Some(out)
    }

    #[test]
    fn crop_boxes_set_on_listed_pages_with_absolute_rects() {
        let out = set_crop_boxes_native(
            &sample(3),
            &crop_pack(&[(0, 10.0, 20.0, 100.0, 200.0), (2, 5.0, 5.0, 50.0, 60.0)]),
        )
        .unwrap();
        let cb0 = crop_box(&out, 0).expect("page 0 crop");
        assert_eq!(cb0, [10.0, 20.0, 110.0, 220.0]);
        let cb2 = crop_box(&out, 2).expect("page 2 crop");
        assert_eq!(cb2, [5.0, 5.0, 55.0, 65.0]);
    }

    #[test]
    fn crop_boxes_leave_unlisted_pages_unchanged() {
        let out = set_crop_boxes_native(
            &sample(3),
            &crop_pack(&[(0, 10.0, 20.0, 100.0, 200.0), (2, 5.0, 5.0, 50.0, 60.0)]),
        )
        .unwrap();
        assert!(crop_box(&out, 1).is_none(), "page 1 has no CropBox");
    }

    #[test]
    fn crop_boxes_preserve_page_count_and_reparse() {
        let out =
            set_crop_boxes_native(&sample(2), &crop_pack(&[(0, 0.0, 0.0, 10.0, 10.0)])).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(n_pages(&out), 2);
    }

    #[test]
    fn crop_boxes_does_not_touch_media_box() {
        let out =
            set_crop_boxes_native(&sample(1), &crop_pack(&[(0, 1.0, 1.0, 5.0, 5.0)])).unwrap();
        assert_eq!(media_box(&out, 0), [0.0, 0.0, 300.0, 400.0]);
    }

    #[test]
    fn crop_boxes_reject_out_of_range_page_index() {
        let err = set_crop_boxes_native(&sample(2), &crop_pack(&[(9, 0.0, 0.0, 10.0, 10.0)]))
            .unwrap_err();
        assert!(err.contains("out of range"), "got: {err}");
    }

    #[test]
    fn crop_boxes_reject_zero_dimensions() {
        let err =
            set_crop_boxes_native(&sample(1), &crop_pack(&[(0, 0.0, 0.0, 0.0, 10.0)])).unwrap_err();
        assert_eq!(err, "Crop box width and height must be positive");
        let err =
            set_crop_boxes_native(&sample(1), &crop_pack(&[(0, 0.0, 0.0, 10.0, 0.0)])).unwrap_err();
        assert_eq!(err, "Crop box width and height must be positive");
    }

    #[test]
    fn crop_boxes_reject_negative_dimensions() {
        let err = set_crop_boxes_native(&sample(1), &crop_pack(&[(0, 0.0, 0.0, -10.0, 10.0)]))
            .unwrap_err();
        assert_eq!(err, "Crop box width and height must be positive");
    }

    #[test]
    fn crop_boxes_reject_non_finite_values() {
        let err = set_crop_boxes_native(&sample(1), &crop_pack(&[(0, f32::NAN, 0.0, 10.0, 10.0)]))
            .unwrap_err();
        assert_eq!(err, "Crop box values must be finite numbers");
        let err = set_crop_boxes_native(
            &sample(1),
            &crop_pack(&[(0, 0.0, 0.0, f32::INFINITY, 10.0)]),
        )
        .unwrap_err();
        assert_eq!(err, "Crop box values must be finite numbers");
    }

    #[test]
    fn crop_boxes_reject_wrong_magic() {
        let bad = PackWriter::new(b"NOPE").u32(0).finish();
        assert_eq!(
            set_crop_boxes_native(&sample(1), &bad).unwrap_err(),
            "Invalid input pack"
        );
    }

    #[test]
    fn crop_boxes_reject_truncated_pack() {
        // declares one entry, supplies only the page index
        let truncated = PackWriter::new(CROP_BOXES_MAGIC).u32(1).u32(0).finish();
        assert_eq!(
            set_crop_boxes_native(&sample(1), &truncated).unwrap_err(),
            "Input pack ended early"
        );
    }

    #[test]
    fn crop_boxes_reject_trailing_data() {
        let mut padded = crop_pack(&[(0, 0.0, 0.0, 10.0, 10.0)]);
        padded.push(0);
        assert_eq!(
            set_crop_boxes_native(&sample(1), &padded).unwrap_err(),
            "Input pack has trailing data"
        );
    }

    #[test]
    fn crop_boxes_empty_pack_is_noop_passthrough() {
        let out = set_crop_boxes_native(&sample(2), &crop_pack(&[])).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(n_pages(&out), 2);
        assert!(crop_box(&out, 0).is_none());
    }
}
