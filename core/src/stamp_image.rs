//! Stamp raster images (drawn or typed signatures, logos…) onto PDF pages.
//!
//! Rust replacement for `stampImage` / `stampImageMany` in `src/lib/tools/sign.ts`:
//! the browser sends one `UFST` pack describing every placement, and this module
//! embeds each unique image once, then appends a draw operation to every target
//! page's content stream. These are *electronic* signatures (visual marks) — no
//! cryptographic proof is attached.
//!
//! Pack format (`UFST`, little-endian), produced by the TypeScript side:
//! - `UFST` magic
//! - u32 stamp count (must be ≥ 1)
//! - repeated stamps:
//!   - u8 image kind: 0 = PNG, 1 = JPEG
//!   - u32 0-based page index
//!   - f32 x, f32 y — bottom-left corner of the stamp, in PDF points
//!   - f32 width — in points, must be > 0
//!   - f32 height — in points; 0 means "derive from the image's aspect ratio"
//!   - f32 opacity — in 0..=1; 1 = fully opaque
//!   - u32 image byte length, then the raw PNG/JPEG bytes
//!
//! Parity notes (vs the pdf-lib implementation this replaces):
//! - PNGs are decoded and re-embedded as FlateDecode RGB, with the alpha channel
//!   (when present) split into an `/SMask` — the same strategy pdf-lib uses.
//! - JPEGs are embedded byte-for-byte with `/Filter /DCTDecode` (never re-encoded).
//! - Identical image bytes are embedded once per call (dedup by content + kind).
//! - Existing page content is wrapped in `q … Q` so a dangling transform left by
//!   the original stream cannot displace the stamp (pdf-lib wraps the same way).
//! - An `/ExtGState` is only emitted for opacity < 1 (pdf-lib emits `ca 1` too;
//!   rendering is identical).

use std::collections::{BTreeMap, BTreeSet};

use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};

use crate::pack::PackReader;
use crate::util::{materialize_inherited_page_attrs, save_compact};

/// Magic prefix of the stamp pack.
pub const STAMP_PACK_MAGIC: &[u8; 4] = b"UFST";

const KIND_PNG: u8 = 0;
const KIND_JPEG: u8 = 1;

/// One placement parsed from the pack.
struct Stamp<'a> {
    kind: u8,
    page_index: u32,
    x: f32,
    y: f32,
    width: f32,
    /// 0.0 = derive from the image's aspect ratio.
    height: f32,
    opacity: f32,
    image: &'a [u8],
}

/// An image already embedded into the document, reusable across stamps.
struct EmbeddedImage {
    xobject: ObjectId,
    name: Vec<u8>,
    pixel_width: u32,
    pixel_height: u32,
}

/// Stamp PNG/JPEG images onto pages as described by a `UFST` pack.
///
/// Coordinates are PDF points with `(x, y)` the bottom-left corner of the stamp.
/// A `height` of 0 keeps the image's aspect ratio (`width * h_px / w_px`).
/// Returns the new PDF bytes; the input is never modified.
pub fn stamp_images_native(data: &[u8], pack: &[u8]) -> Result<Vec<u8>, String> {
    let stamps = parse_pack(pack)?;
    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    let pages: Vec<ObjectId> = doc.page_iter().collect();
    validate(&stamps, pages.len())?;

    // Pull inherited attributes (notably /Resources) down onto every touched page
    // and collect resource names already in use so fresh names cannot collide.
    let touched: BTreeSet<ObjectId> = stamps
        .iter()
        .map(|s| pages[s.page_index as usize])
        .collect();
    let mut used_names: BTreeSet<Vec<u8>> = BTreeSet::new();
    for &page_id in &touched {
        materialize_inherited_page_attrs(&mut doc, page_id).map_err(|e| e.to_string())?;
        collect_resource_names(&doc, page_id, &mut used_names);
    }

    let mut img_counter: u32 = 0;
    let mut gs_counter: u32 = 0;
    // Embedded images, deduped by (kind, exact bytes).
    let mut images: Vec<(u8, &[u8], EmbeddedImage)> = Vec::new();
    // Graphics states, deduped by exact opacity value.
    let mut gstates: Vec<(u32, ObjectId, Vec<u8>)> = Vec::new();
    // Per-page accumulated draw operations and resource registrations.
    let mut page_ops: BTreeMap<ObjectId, Vec<u8>> = BTreeMap::new();
    let mut page_xobjects: BTreeMap<ObjectId, Vec<(Vec<u8>, ObjectId)>> = BTreeMap::new();
    let mut page_gstates: BTreeMap<ObjectId, Vec<(Vec<u8>, ObjectId)>> = BTreeMap::new();

    for stamp in &stamps {
        let page_id = pages[stamp.page_index as usize];

        let idx = match images
            .iter()
            .position(|(kind, bytes, _)| *kind == stamp.kind && *bytes == stamp.image)
        {
            Some(i) => i,
            None => {
                let (xobject, pixel_width, pixel_height) = match stamp.kind {
                    KIND_PNG => embed_png(&mut doc, stamp.image)?,
                    _ => embed_jpeg(&mut doc, stamp.image)?,
                };
                let name = fresh_name("UfImg", &mut img_counter, &mut used_names);
                images.push((
                    stamp.kind,
                    stamp.image,
                    EmbeddedImage {
                        xobject,
                        name,
                        pixel_width,
                        pixel_height,
                    },
                ));
                images.len() - 1
            }
        };
        let embedded = &images[idx].2;

        let height = if stamp.height == 0.0 {
            stamp.width * (embedded.pixel_height as f32 / embedded.pixel_width as f32)
        } else {
            stamp.height
        };

        let gs = if stamp.opacity < 1.0 {
            let bits = stamp.opacity.to_bits();
            let i = match gstates.iter().position(|(b, _, _)| *b == bits) {
                Some(i) => i,
                None => {
                    let id = doc.add_object(dictionary! {
                        "Type" => "ExtGState",
                        "ca" => stamp.opacity,
                        "CA" => stamp.opacity,
                    });
                    let name = fresh_name("UfGs", &mut gs_counter, &mut used_names);
                    gstates.push((bits, id, name));
                    gstates.len() - 1
                }
            };
            Some((gstates[i].1, gstates[i].2.clone()))
        } else {
            None
        };

        let mut operations = vec![
            Operation::new("q", vec![]),
            Operation::new(
                "cm",
                vec![
                    Object::Real(stamp.width),
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(height),
                    Object::Real(stamp.x),
                    Object::Real(stamp.y),
                ],
            ),
        ];
        if let Some((_, ref gs_name)) = gs {
            operations.push(Operation::new("gs", vec![Object::Name(gs_name.clone())]));
        }
        operations.push(Operation::new(
            "Do",
            vec![Object::Name(embedded.name.clone())],
        ));
        operations.push(Operation::new("Q", vec![]));
        let encoded = Content { operations }.encode().map_err(|e| e.to_string())?;
        let buf = page_ops.entry(page_id).or_default();
        if !buf.is_empty() {
            buf.push(b'\n');
        }
        buf.extend_from_slice(&encoded);

        push_unique(
            page_xobjects.entry(page_id).or_default(),
            embedded.name.clone(),
            embedded.xobject,
        );
        if let Some((gs_id, gs_name)) = gs {
            push_unique(page_gstates.entry(page_id).or_default(), gs_name, gs_id);
        }
    }

    let (push_id, pop_id) = wrap_stream_ids(&mut doc)?;
    for (page_id, ops) in &page_ops {
        register_page_resources(
            &mut doc,
            *page_id,
            page_xobjects
                .get(page_id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
            page_gstates
                .get(page_id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
        )?;
        append_page_content(&mut doc, *page_id, ops, push_id, pop_id)?;
    }

    save_compact(doc).map_err(|e| e.to_string())
}

fn content_bytes(operations: Vec<Operation>) -> Result<Vec<u8>, String> {
    let body = Content { operations }.encode().map_err(|e| e.to_string())?;
    let mut bytes = Vec::with_capacity(body.len() + 2);
    bytes.push(b'\n');
    bytes.extend(body);
    bytes.push(b'\n');
    Ok(bytes)
}

fn wrap_stream_ids(doc: &mut Document) -> Result<(ObjectId, ObjectId), String> {
    let push = content_bytes(vec![Operation::new("q", vec![])])?;
    let pop = content_bytes(vec![Operation::new("Q", vec![])])?;
    Ok((
        doc.add_object(Stream::new(dictionary! {}, push)),
        doc.add_object(Stream::new(dictionary! {}, pop)),
    ))
}

fn parse_pack(pack: &[u8]) -> Result<Vec<Stamp<'_>>, String> {
    let mut r = PackReader::new(pack);
    r.expect_magic(STAMP_PACK_MAGIC)?;
    let count = r.read_count(29)?;
    if count == 0 {
        return Err("No signatures placed".to_string());
    }
    let mut stamps = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let kind = r.read_u8()?;
        if kind != KIND_PNG && kind != KIND_JPEG {
            return Err("Unsupported image type in stamp pack".to_string());
        }
        stamps.push(Stamp {
            kind,
            page_index: r.read_u32()?,
            x: r.read_f32()?,
            y: r.read_f32()?,
            width: r.read_f32()?,
            height: r.read_f32()?,
            opacity: r.read_f32()?,
            image: r.read_bytes()?,
        });
    }
    r.expect_done()?;
    Ok(stamps)
}

fn validate(stamps: &[Stamp], page_count: usize) -> Result<(), String> {
    for s in stamps {
        if (s.page_index as usize) >= page_count {
            return Err(format!("Page index {} out of range", s.page_index));
        }
        if !s.x.is_finite() || !s.y.is_finite() {
            return Err("Stamp position must be a finite number".to_string());
        }
        if !s.width.is_finite() || s.width <= 0.0 {
            return Err("Stamp width must be a positive number".to_string());
        }
        if !s.height.is_finite() || s.height < 0.0 {
            return Err("Stamp height must be a positive number".to_string());
        }
        if !s.opacity.is_finite() || !(0.0..=1.0).contains(&s.opacity) {
            return Err("Stamp opacity must be between 0 and 1".to_string());
        }
    }
    Ok(())
}

/// Zlib-compress raw sample data for a FlateDecode stream.
fn flate(data: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Write;
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(data).map_err(|e| e.to_string())?;
    enc.finish().map_err(|e| e.to_string())
}

/// Decode a PNG and embed it as a FlateDecode RGB image XObject, splitting any
/// alpha channel into an `/SMask`. Returns `(xobject_id, pixel_w, pixel_h)`.
fn embed_png(doc: &mut Document, bytes: &[u8]) -> Result<(ObjectId, u32, u32), String> {
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|e| format!("Invalid PNG image: {e}"))?;
    let (width, height) = (img.width(), img.height());
    if width == 0 || height == 0 {
        return Err("Stamp image has zero width or height".to_string());
    }

    let mut dict = dictionary! {
        "Type" => "XObject",
        "Subtype" => "Image",
        "Width" => width as i64,
        "Height" => height as i64,
        "BitsPerComponent" => 8,
        "ColorSpace" => "DeviceRGB",
        "Filter" => "FlateDecode",
    };

    let rgb = if img.color().has_alpha() {
        let rgba = img.into_rgba8();
        let pixels = (width as usize) * (height as usize);
        let mut rgb = Vec::with_capacity(pixels * 3);
        let mut alpha = Vec::with_capacity(pixels);
        for px in rgba.pixels() {
            rgb.extend_from_slice(&px.0[..3]);
            alpha.push(px.0[3]);
        }
        let smask = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => width as i64,
                "Height" => height as i64,
                "BitsPerComponent" => 8,
                "ColorSpace" => "DeviceGray",
                "Filter" => "FlateDecode",
            },
            flate(&alpha)?,
        ));
        dict.set("SMask", Object::Reference(smask));
        rgb
    } else {
        img.into_rgb8().into_raw()
    };

    let id = doc.add_object(Stream::new(dict, flate(&rgb)?));
    Ok((id, width, height))
}

/// Embed a JPEG byte-for-byte as a DCTDecode image XObject (no re-encoding).
/// Returns `(xobject_id, pixel_w, pixel_h)`.
fn embed_jpeg(doc: &mut Document, bytes: &[u8]) -> Result<(ObjectId, u32, u32), String> {
    use image::codecs::jpeg::JpegDecoder;
    use image::{ExtendedColorType, ImageDecoder};

    let decoder = JpegDecoder::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("Invalid JPEG image: {e}"))?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 {
        return Err("Stamp image has zero width or height".to_string());
    }
    let color_space = match decoder.original_color_type() {
        ExtendedColorType::L8 | ExtendedColorType::L16 => "DeviceGray",
        ExtendedColorType::Cmyk8 => "DeviceCMYK",
        _ => "DeviceRGB",
    };

    let id = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => width as i64,
            "Height" => height as i64,
            "BitsPerComponent" => 8,
            "ColorSpace" => color_space,
            "Filter" => "DCTDecode",
        },
        bytes.to_vec(),
    ));
    Ok((id, width, height))
}

/// Generate a resource name like `UfImg0` that is not already in `used`.
fn fresh_name(prefix: &str, counter: &mut u32, used: &mut BTreeSet<Vec<u8>>) -> Vec<u8> {
    loop {
        let name = format!("{prefix}{counter}").into_bytes();
        *counter += 1;
        if used.insert(name.clone()) {
            return name;
        }
    }
}

fn push_unique(list: &mut Vec<(Vec<u8>, ObjectId)>, name: Vec<u8>, id: ObjectId) {
    if !list.iter().any(|(n, _)| *n == name) {
        list.push((name, id));
    }
}

/// Resolve an object that may be a direct dictionary or a reference to one.
fn resolve_dict<'a>(doc: &'a Document, obj: Option<&'a Object>) -> Option<&'a Dictionary> {
    match obj? {
        Object::Dictionary(d) => Some(d),
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        _ => None,
    }
}

/// Record every `/XObject` and `/ExtGState` resource name already used by a page.
fn collect_resource_names(doc: &Document, page_id: ObjectId, used: &mut BTreeSet<Vec<u8>>) {
    let Ok(page) = doc.get_dictionary(page_id) else {
        return;
    };
    let Some(res) = resolve_dict(doc, page.get(b"Resources").ok()) else {
        return;
    };
    for key in [b"XObject".as_slice(), b"ExtGState".as_slice()] {
        if let Some(sub) = resolve_dict(doc, res.get(key).ok()) {
            for (name, _) in sub.iter() {
                used.insert(name.clone());
            }
        }
    }
}

/// Merge XObject/ExtGState entries into the page's `/Resources`, normalizing it
/// (and its subdictionaries) to direct dictionaries on the page — the same
/// normalization pdf-lib performs, which also avoids mutating resource dicts
/// shared with untouched pages.
fn register_page_resources(
    doc: &mut Document,
    page_id: ObjectId,
    xobjects: &[(Vec<u8>, ObjectId)],
    gstates: &[(Vec<u8>, ObjectId)],
) -> Result<(), String> {
    let res_obj = doc
        .get_dictionary(page_id)
        .map_err(|e| e.to_string())?
        .get(b"Resources")
        .ok()
        .cloned();
    let mut res = match res_obj {
        Some(Object::Reference(id)) => doc.get_dictionary(id).map_err(|e| e.to_string())?.clone(),
        Some(Object::Dictionary(d)) => d,
        _ => Dictionary::new(),
    };
    merge_subdict(doc, &mut res, b"XObject", xobjects)?;
    merge_subdict(doc, &mut res, b"ExtGState", gstates)?;
    doc.get_object_mut(page_id)
        .map_err(|e| e.to_string())?
        .as_dict_mut()
        .map_err(|e| e.to_string())?
        .set("Resources", Object::Dictionary(res));
    Ok(())
}

fn merge_subdict(
    doc: &Document,
    res: &mut Dictionary,
    key: &[u8],
    entries: &[(Vec<u8>, ObjectId)],
) -> Result<(), String> {
    if entries.is_empty() {
        return Ok(());
    }
    let mut sub = match res.get(key) {
        Ok(Object::Reference(id)) => doc.get_dictionary(*id).map_err(|e| e.to_string())?.clone(),
        Ok(Object::Dictionary(d)) => d.clone(),
        _ => Dictionary::new(),
    };
    for (name, id) in entries {
        sub.set(name.clone(), Object::Reference(*id));
    }
    res.set(key.to_vec(), Object::Dictionary(sub));
    Ok(())
}

/// Append the stamp stream without decoding existing page content. Existing
/// streams are bracketed by shared `q`/`Q` wrapper streams so a dangling source
/// graphics state cannot displace the stamp.
fn append_page_content(
    doc: &mut Document,
    page_id: ObjectId,
    ops: &[u8],
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
    let stream_id = doc.add_object(Stream::new(dictionary! {}, padded_bytes(ops)));
    let contents: Vec<Object> = if items.is_empty() {
        vec![Object::Reference(stream_id)]
    } else {
        let mut v = Vec::with_capacity(items.len() + 3);
        v.push(Object::Reference(push_id));
        v.extend(items);
        v.push(Object::Reference(pop_id));
        v.push(Object::Reference(stream_id));
        v
    };
    doc.get_object_mut(page_id)
        .map_err(|e| e.to_string())?
        .as_dict_mut()
        .map_err(|e| e.to_string())?
        .set("Contents", contents);
    Ok(())
}

fn padded_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 2);
    out.push(b'\n');
    out.extend_from_slice(bytes);
    out.push(b'\n');
    out
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackWriter;
    use crate::pages::page_count_native;
    use crate::util::fixtures::{page_content_text, page_width, sample, sample_with_text, PNG_1X1};
    use crate::util::number_as_f32;
    use lopdf::Document;

    // ---- helpers ----------------------------------------------------------

    /// One placement, with sensible defaults for the common test case.
    struct Place<'a> {
        kind: u8,
        page: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        opacity: f32,
        image: &'a [u8],
    }

    impl<'a> Place<'a> {
        fn png(page: u32, image: &'a [u8]) -> Self {
            Self {
                kind: 0,
                page,
                x: 10.0,
                y: 20.0,
                w: 100.0,
                h: 0.0,
                opacity: 1.0,
                image,
            }
        }
        fn jpeg(page: u32, image: &'a [u8]) -> Self {
            Self {
                kind: 1,
                ..Self::png(page, image)
            }
        }
    }

    fn pack(stamps: &[Place]) -> Vec<u8> {
        let mut w = PackWriter::new(STAMP_PACK_MAGIC).u32(stamps.len() as u32);
        for s in stamps {
            w = w
                .u8(s.kind)
                .u32(s.page)
                .f32(s.x)
                .f32(s.y)
                .f32(s.w)
                .f32(s.h)
                .f32(s.opacity)
                .bytes(s.image);
        }
        w.finish()
    }

    fn rgb_png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x * 37) as u8, (y * 53) as u8, 128])
        });
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut out, image::ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    fn rgba_png(w: u32, h: u32) -> Vec<u8> {
        let img =
            image::RgbaImage::from_fn(w, h, |x, _| image::Rgba([200, 30, 40, (x * 80) as u8]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut out, image::ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    fn rgb_jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([180, 90, 45]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut out, image::ImageFormat::Jpeg)
            .unwrap();
        out.into_inner()
    }

    fn pdf_with_filtered_raw_content() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let content_id = doc.add_object(Stream::new(
            dictionary! { "Filter" => "DCTDecode" },
            b"raw-original-content-sentinel".to_vec(),
        ));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => Dictionary::new(),
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
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    fn gray_jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = image::GrayImage::from_pixel(w, h, image::Luma([99]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageLuma8(img)
            .write_to(&mut out, image::ImageFormat::Jpeg)
            .unwrap();
        out.into_inner()
    }

    /// Every image XObject stream in the document (includes SMasks).
    fn image_xobjects(data: &[u8]) -> Vec<lopdf::Stream> {
        let doc = Document::load_mem(data).unwrap();
        doc.objects
            .values()
            .filter_map(|o| match o {
                Object::Stream(s)
                    if s.dict.get(b"Subtype").ok().and_then(|v| v.as_name().ok())
                        == Some(b"Image".as_slice()) =>
                {
                    Some(s.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// Every ExtGState dictionary in the document.
    fn extgstates(data: &[u8]) -> Vec<Dictionary> {
        let doc = Document::load_mem(data).unwrap();
        doc.objects
            .values()
            .filter_map(|o| match o {
                Object::Dictionary(d)
                    if d.get(b"Type").ok().and_then(|v| v.as_name().ok())
                        == Some(b"ExtGState".as_slice()) =>
                {
                    Some(d.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// Operands of the first `cm` operator on page `n`.
    fn cm_operands(data: &[u8], n: usize) -> Vec<f32> {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        let content = doc.get_page_content(pages[n]).unwrap();
        let ops = Content::decode(&content).unwrap().operations;
        ops.iter()
            .find(|op| op.operator == "cm")
            .map(|op| {
                op.operands
                    .iter()
                    .map(|o| number_as_f32(o).unwrap())
                    .collect()
            })
            .expect("page has a cm operator")
    }

    fn assert_close(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-3, "{a} != {b}");
    }

    // ---- happy paths --------------------------------------------------------

    #[test]
    fn stamps_png_on_selected_page_only() {
        let out = stamp_images_native(&sample(3), &pack(&[Place::png(0, PNG_1X1)])).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count_native(&out).unwrap(), 3);
        assert!(page_content_text(&out, 0).contains("Do"));
        assert!(!page_content_text(&out, 1).contains("Do"));
        assert!(!page_content_text(&out, 2).contains("Do"));
        // Page geometry untouched.
        assert_eq!(page_width(&out, 0), 300.0);
        assert_eq!(page_width(&out, 1), 301.0);
        assert_eq!(page_width(&out, 2), 302.0);

        // The TS default (pageIndex ?? last) is resolved by the JS wrapper; the
        // pack always carries an explicit index — here, the last page.
        let out = stamp_images_native(&sample(2), &pack(&[Place::png(1, PNG_1X1)])).unwrap();
        assert!(!page_content_text(&out, 0).contains("Do"));
        assert!(page_content_text(&out, 1).contains("Do"));
    }

    #[test]
    fn preserves_existing_content_and_wraps_it_in_q_big_q() {
        let pdf = sample_with_text(1, Some("Hello"));
        let out = stamp_images_native(&pdf, &pack(&[Place::png(0, PNG_1X1)])).unwrap();
        let text = page_content_text(&out, 0);
        assert!(
            text.trim_start().starts_with("q\n"),
            "existing content should be wrapped: {text}"
        );
        assert!(text.contains("Hello 0"), "original text lost: {text}");
        assert!(text.contains("Tj"));
        assert!(text.contains("/UfImg0 Do"));
        // Stamp comes after (on top of) the original content.
        assert!(text.find("Tj").unwrap() < text.find("/UfImg0 Do").unwrap());
    }

    #[test]
    fn preserves_existing_content_stream_without_decoding_it() {
        let pdf = pdf_with_filtered_raw_content();
        assert!(pdf
            .windows(b"raw-original-content-sentinel".len())
            .any(|w| w == b"raw-original-content-sentinel"));

        let out = stamp_images_native(&pdf, &pack(&[Place::png(0, PNG_1X1)])).unwrap();
        assert!(out
            .windows(b"raw-original-content-sentinel".len())
            .any(|w| w == b"raw-original-content-sentinel"));

        let doc = Document::load_mem(&out).unwrap();
        let page_id = doc.page_iter().next().unwrap();
        let contents = doc
            .get_dictionary(page_id)
            .unwrap()
            .get(b"Contents")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(contents.len(), 4, "q wrapper, original, Q wrapper, stamp");
    }

    #[test]
    fn cm_encodes_size_and_bottom_left_position() {
        let mut p = Place::png(0, PNG_1X1);
        (p.x, p.y, p.w, p.h) = (10.0, 20.0, 100.0, 40.0);
        let out = stamp_images_native(&sample(1), &pack(&[p])).unwrap();
        let cm = cm_operands(&out, 0);
        assert_eq!(cm.len(), 6);
        assert_close(cm[0], 100.0); // width scale
        assert_close(cm[1], 0.0);
        assert_close(cm[2], 0.0);
        assert_close(cm[3], 40.0); // explicit height honored
        assert_close(cm[4], 10.0); // x translation
        assert_close(cm[5], 20.0); // y translation (from page bottom)
    }

    #[test]
    fn computes_height_from_aspect_ratio_when_zero() {
        // 2×1 image -> aspect 0.5 -> width 100 gives height 50.
        let png = rgba_png(2, 1);
        let mut p = Place::png(0, &png);
        (p.w, p.h) = (100.0, 0.0);
        let out = stamp_images_native(&sample(1), &pack(&[p])).unwrap();
        assert_close(cm_operands(&out, 0)[3], 50.0);
    }

    #[test]
    fn opacity_below_one_creates_shared_extgstate() {
        let mut p = Place::png(0, PNG_1X1);
        p.opacity = 0.25;
        let out = stamp_images_native(&sample(1), &pack(&[p])).unwrap();
        let gs = extgstates(&out);
        assert_eq!(gs.len(), 1);
        assert_close(number_as_f32(gs[0].get(b"ca").unwrap()).unwrap(), 0.25);
        assert_close(number_as_f32(gs[0].get(b"CA").unwrap()).unwrap(), 0.25);
        assert!(page_content_text(&out, 0).contains("/UfGs0 gs"));
    }

    #[test]
    fn zero_opacity_is_valid_and_full_opacity_skips_extgstate() {
        let mut transparent = Place::png(0, PNG_1X1);
        transparent.opacity = 0.0;
        let out = stamp_images_native(&sample(1), &pack(&[transparent])).unwrap();
        assert_eq!(extgstates(&out).len(), 1);
        assert_close(
            number_as_f32(extgstates(&out)[0].get(b"ca").unwrap()).unwrap(),
            0.0,
        );

        let opaque = Place::png(0, PNG_1X1); // opacity 1.0 default
        let out = stamp_images_native(&sample(1), &pack(&[opaque])).unwrap();
        assert_eq!(extgstates(&out).len(), 0);
        assert!(!page_content_text(&out, 0).contains(" gs"));
    }

    // ---- image embedding ----------------------------------------------------

    #[test]
    fn jpeg_is_embedded_verbatim_with_dctdecode() {
        let jpeg = rgb_jpeg(4, 4);
        let out = stamp_images_native(&sample(1), &pack(&[Place::jpeg(0, &jpeg)])).unwrap();
        let imgs = image_xobjects(&out);
        assert_eq!(imgs.len(), 1);
        let s = &imgs[0];
        assert_eq!(
            s.dict.get(b"Filter").unwrap().as_name().unwrap(),
            b"DCTDecode"
        );
        assert_eq!(
            s.dict.get(b"ColorSpace").unwrap().as_name().unwrap(),
            b"DeviceRGB"
        );
        assert_eq!(s.dict.get(b"Width").unwrap().as_i64().unwrap(), 4);
        assert_eq!(s.dict.get(b"Height").unwrap().as_i64().unwrap(), 4);
        // Byte-for-byte passthrough — never re-encoded.
        assert_eq!(s.content, jpeg);
    }

    #[test]
    fn grayscale_jpeg_uses_devicegray() {
        let jpeg = gray_jpeg(3, 3);
        let out = stamp_images_native(&sample(1), &pack(&[Place::jpeg(0, &jpeg)])).unwrap();
        let imgs = image_xobjects(&out);
        assert_eq!(imgs.len(), 1);
        assert_eq!(
            imgs[0].dict.get(b"ColorSpace").unwrap().as_name().unwrap(),
            b"DeviceGray"
        );
        assert_eq!(imgs[0].content, jpeg);
    }

    #[test]
    fn png_with_alpha_gets_a_devicegray_smask() {
        // PNG_1X1 is grayscale+alpha, so an SMask must be split out.
        let out = stamp_images_native(&sample(1), &pack(&[Place::png(0, PNG_1X1)])).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        let (main, smask_id) = doc
            .objects
            .values()
            .find_map(|o| match o {
                Object::Stream(s) => s
                    .dict
                    .get(b"SMask")
                    .ok()
                    .and_then(|v| v.as_reference().ok())
                    .map(|r| (s.clone(), r)),
                _ => None,
            })
            .expect("an image XObject with an SMask");
        assert_eq!(
            main.dict.get(b"ColorSpace").unwrap().as_name().unwrap(),
            b"DeviceRGB"
        );
        assert_eq!(
            main.dict.get(b"Filter").unwrap().as_name().unwrap(),
            b"FlateDecode"
        );
        let smask = doc.get_object(smask_id).unwrap().as_stream().unwrap();
        assert_eq!(
            smask.dict.get(b"ColorSpace").unwrap().as_name().unwrap(),
            b"DeviceGray"
        );
        assert_eq!(
            smask
                .dict
                .get(b"BitsPerComponent")
                .unwrap()
                .as_i64()
                .unwrap(),
            8
        );
    }

    #[test]
    fn opaque_png_has_no_smask() {
        let png = rgb_png(2, 2);
        let out = stamp_images_native(&sample(1), &pack(&[Place::png(0, &png)])).unwrap();
        let imgs = image_xobjects(&out);
        assert_eq!(imgs.len(), 1, "no separate SMask stream expected");
        assert!(imgs[0].dict.get(b"SMask").is_err());
        assert_eq!(
            imgs[0].dict.get(b"ColorSpace").unwrap().as_name().unwrap(),
            b"DeviceRGB"
        );
        assert_eq!(
            imgs[0].dict.get(b"Filter").unwrap().as_name().unwrap(),
            b"FlateDecode"
        );
    }

    // ---- deduplication --------------------------------------------------------

    #[test]
    fn identical_bytes_are_embedded_once_across_pages() {
        let png = rgb_png(2, 2);
        let out = stamp_images_native(
            &sample(2),
            &pack(&[Place::png(0, &png), Place::png(1, &png)]),
        )
        .unwrap();
        assert_eq!(image_xobjects(&out).len(), 1);
        assert!(page_content_text(&out, 0).contains("/UfImg0 Do"));
        assert!(page_content_text(&out, 1).contains("/UfImg0 Do"));
    }

    #[test]
    fn same_opacity_shares_one_extgstate() {
        let png = rgb_png(2, 2);
        let mut a = Place::png(0, &png);
        let mut b = Place::png(1, &png);
        (a.opacity, b.opacity) = (0.7, 0.7);
        let out = stamp_images_native(&sample(2), &pack(&[a, b])).unwrap();
        assert_eq!(image_xobjects(&out).len(), 1);
        assert_eq!(extgstates(&out).len(), 1);
    }

    #[test]
    fn different_opacities_create_distinct_extgstates() {
        let png = rgb_png(2, 2);
        let mut a = Place::png(0, &png);
        let mut b = Place::png(1, &png);
        (a.opacity, b.opacity) = (0.7, 0.5);
        let out = stamp_images_native(&sample(2), &pack(&[a, b])).unwrap();
        assert_eq!(image_xobjects(&out).len(), 1, "image still deduped");
        assert_eq!(extgstates(&out).len(), 2);
    }

    #[test]
    fn multiple_stamps_on_one_page_draw_in_pack_order() {
        let png_a = rgb_png(2, 2);
        let png_b = rgb_png(3, 3);
        let out = stamp_images_native(
            &sample(1),
            &pack(&[Place::png(0, &png_a), Place::png(0, &png_b)]),
        )
        .unwrap();
        assert_eq!(
            image_xobjects(&out).len(),
            2,
            "distinct bytes embed separately"
        );
        let text = page_content_text(&out, 0);
        let first = text.find("/UfImg0 Do").expect("first stamp drawn");
        let second = text.find("/UfImg1 Do").expect("second stamp drawn");
        assert!(first < second, "stamps must draw in pack order: {text}");
    }

    #[test]
    fn restamping_an_already_stamped_pdf_picks_fresh_names() {
        let png_a = rgb_png(2, 2);
        let png_b = rgb_png(3, 3);
        let once = stamp_images_native(&sample(1), &pack(&[Place::png(0, &png_a)])).unwrap();
        let twice = stamp_images_native(&once, &pack(&[Place::png(0, &png_b)])).unwrap();
        assert_eq!(image_xobjects(&twice).len(), 2);
        let text = page_content_text(&twice, 0);
        assert!(
            text.contains("/UfImg0 Do"),
            "first run's stamp kept: {text}"
        );
        assert!(
            text.contains("/UfImg1 Do"),
            "second run avoided the used name: {text}"
        );
        assert_eq!(page_count_native(&twice).unwrap(), 1);
    }

    // ---- errors ---------------------------------------------------------------

    #[test]
    fn rejects_empty_stamp_list() {
        let empty = PackWriter::new(STAMP_PACK_MAGIC).u32(0).finish();
        let err = stamp_images_native(&sample(1), &empty).unwrap_err();
        assert_eq!(err, "No signatures placed");
    }

    #[test]
    fn rejects_wrong_magic() {
        let bad = PackWriter::new(b"NOPE").u32(1).finish();
        let err = stamp_images_native(&sample(1), &bad).unwrap_err();
        assert_eq!(err, "Invalid input pack");
    }

    #[test]
    fn rejects_truncated_and_trailing_packs() {
        // Declares two stamps but provides one.
        let mut truncated = pack(&[Place::png(0, PNG_1X1)]);
        truncated[4..8].copy_from_slice(&2u32.to_le_bytes());
        let err = stamp_images_native(&sample(1), &truncated).unwrap_err();
        assert!(err.contains("ended early"), "{err}");

        let mut trailing = pack(&[Place::png(0, PNG_1X1)]);
        trailing.push(0xff);
        let err = stamp_images_native(&sample(1), &trailing).unwrap_err();
        assert!(err.contains("trailing data"), "{err}");
    }

    #[test]
    fn rejects_out_of_range_page_index() {
        let err = stamp_images_native(&sample(2), &pack(&[Place::png(99, PNG_1X1)])).unwrap_err();
        assert_eq!(err, "Page index 99 out of range");
        // A "negative" JS index arrives as a huge u32 after the cast.
        let err =
            stamp_images_native(&sample(2), &pack(&[Place::png(u32::MAX, PNG_1X1)])).unwrap_err();
        assert!(err.contains("out of range"), "{err}");
    }

    #[test]
    fn rejects_invalid_image_bytes() {
        let err = stamp_images_native(&sample(1), &pack(&[Place::png(0, b"garbage")])).unwrap_err();
        assert!(err.contains("Invalid PNG image"), "{err}");
        let err =
            stamp_images_native(&sample(1), &pack(&[Place::jpeg(0, b"garbage")])).unwrap_err();
        assert!(err.contains("Invalid JPEG image"), "{err}");
        let err = stamp_images_native(&sample(1), &pack(&[Place::png(0, b"")])).unwrap_err();
        assert!(err.contains("Invalid PNG image"), "{err}");
    }

    #[test]
    fn rejects_unknown_image_kind() {
        let mut p = Place::png(0, PNG_1X1);
        p.kind = 7;
        let err = stamp_images_native(&sample(1), &pack(&[p])).unwrap_err();
        assert_eq!(err, "Unsupported image type in stamp pack");
    }

    #[test]
    fn rejects_nonpositive_or_nonfinite_width() {
        for w in [0.0_f32, -5.0, f32::NAN, f32::INFINITY] {
            let mut p = Place::png(0, PNG_1X1);
            p.w = w;
            let err = stamp_images_native(&sample(1), &pack(&[p])).unwrap_err();
            assert!(err.contains("width"), "width={w}: {err}");
        }
    }

    #[test]
    fn rejects_bad_opacity_negative_height_and_nonfinite_position() {
        for o in [-0.1_f32, 1.5, f32::NAN] {
            let mut p = Place::png(0, PNG_1X1);
            p.opacity = o;
            let err = stamp_images_native(&sample(1), &pack(&[p])).unwrap_err();
            assert!(err.contains("opacity"), "opacity={o}: {err}");
        }
        let mut p = Place::png(0, PNG_1X1);
        p.h = -10.0;
        let err = stamp_images_native(&sample(1), &pack(&[p])).unwrap_err();
        assert!(err.contains("height"), "{err}");

        let mut p = Place::png(0, PNG_1X1);
        p.x = f32::NAN;
        let err = stamp_images_native(&sample(1), &pack(&[p])).unwrap_err();
        assert!(err.contains("position"), "{err}");
    }

    #[test]
    fn rejects_invalid_pdf_bytes() {
        assert!(stamp_images_native(b"not a pdf", &pack(&[Place::png(0, PNG_1X1)])).is_err());
    }

    // ---- output validity --------------------------------------------------------

    #[test]
    fn output_reparses_and_keeps_stamps_after_optimize_roundtrip() {
        let png = rgb_png(2, 2);
        let out = stamp_images_native(
            &sample(3),
            &pack(&[Place::png(0, &png), Place::jpeg(2, &rgb_jpeg(2, 2))]),
        )
        .unwrap();
        assert!(out.starts_with(b"%PDF-"));
        let reloaded = Document::load_mem(&out).unwrap();
        assert_eq!(reloaded.get_pages().len(), 3);
        // Survives another full load/save cycle.
        let again = crate::pages::optimize_native(&out).unwrap();
        assert!(page_content_text(&again, 0).contains("Do"));
        assert!(page_content_text(&again, 2).contains("Do"));
    }
}
