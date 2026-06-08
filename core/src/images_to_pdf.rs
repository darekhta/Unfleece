//! Build a PDF from a list of PNG/JPEG images, one image per page.
//!
//! Port of `src/lib/tools/imagesToPdf.ts` (pdf-lib) with behavior parity:
//! - `pageSize: "fit"` (default) — each page is sized to its image, 1 px = 1 pt
//!   (pdf-lib's `embedPng`/`embedJpg` report pixel dimensions as points).
//! - `pageSize: "a4" | "letter"` — fixed page; the image is scaled to fit inside
//!   the margins (never upscaled, `scale = min(maxW/w, maxH/h, 1)`) and centered.
//! - JPEGs are embedded by DCT passthrough (no re-encode) when they are plain
//!   1- or 3-component scans; CMYK/exotic JPEGs fall back to decoded RGB pixels.
//! - PNGs are flate-embedded; an alpha channel becomes a DeviceGray `/SMask`.

use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, ObjectId, Stream};
use serde::Deserialize;

use crate::pack::PackReader;
use crate::util::save_compact;

const IMAGES_PACK_MAGIC: &[u8; 4] = b"UFIP";

/// A4 page size in points (210mm × 297mm @ 72dpi) — mirrors PAGE_SIZES in pdf.ts.
const A4: (f32, f32) = (595.28, 841.89);
/// US Letter page size in points (8.5" × 11" @ 72dpi).
const LETTER: (f32, f32) = (612.0, 792.0);

/// `margin` is only meaningful within this range (mirrors the tool registry's
/// 0–200 bound); values outside it are clamped so a fixed page can never end up
/// with a zero or negative content area.
const MARGIN_RANGE: (f32, f32) = (0.0, 200.0);

#[derive(Deserialize)]
struct ImagesToPdfOptions {
    #[serde(default = "default_page_size", rename = "pageSize")]
    page_size: String,
    #[serde(default = "default_margin")]
    margin: f32,
}

fn default_page_size() -> String {
    "fit".to_string()
}

fn default_margin() -> f32 {
    24.0
}

enum PageSize {
    /// Page matches the image exactly.
    Fit,
    /// Fixed page (width, height) in points.
    Fixed(f32, f32),
}

/// Build a PDF from a packed list of images, one page per image, in pack order.
///
/// Pack format (`UFIP`), little-endian:
/// - `UFIP` magic
/// - u32 image count
/// - repeated images: u8 kind (0 = PNG, 1 = JPEG), u32 byte length, image bytes
///
/// `opts_json` mirrors `ImagesToPdfOptions` in imagesToPdf.ts:
/// `{"pageSize": "fit" | "a4" | "letter", "margin": <points>}` — both optional,
/// defaulting to `"fit"` and `24`. `margin` only applies to fixed page sizes.
pub fn images_to_pdf_native(pack: &[u8], opts_json: &str) -> Result<Vec<u8>, String> {
    let mut reader = PackReader::new(pack);
    reader.expect_magic(IMAGES_PACK_MAGIC)?;
    let count = reader.read_count(5)?;
    if count == 0 {
        return Err("No images provided".to_string());
    }

    let opts: ImagesToPdfOptions =
        serde_json::from_str(opts_json).map_err(|e| format!("Invalid options: {e}"))?;
    let page_size = match opts.page_size.as_str() {
        "fit" => PageSize::Fit,
        "a4" => PageSize::Fixed(A4.0, A4.1),
        "letter" => PageSize::Fixed(LETTER.0, LETTER.1),
        _ => return Err("Invalid page size option".to_string()),
    };
    let margin = opts.margin.clamp(MARGIN_RANGE.0, MARGIN_RANGE.1);

    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut kids: Vec<Object> = Vec::with_capacity(count);

    for _ in 0..count {
        let kind = reader.read_u8()?;
        let bytes = reader.read_bytes()?;
        let image = match kind {
            0 => embed_png(&mut doc, bytes)?,
            1 => embed_jpeg(&mut doc, bytes)?,
            _ => return Err("Unknown image kind in input pack".to_string()),
        };

        let (page_w, page_h, draw) =
            layout(&page_size, margin, image.width as f32, image.height as f32);
        let content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![
                        draw[2].into(),
                        0f32.into(),
                        0f32.into(),
                        draw[3].into(),
                        draw[0].into(),
                        draw[1].into(),
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
            "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image.id } },
            "MediaBox" => vec![0f32.into(), 0f32.into(), page_w.into(), page_h.into()],
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

/// Page size plus draw rectangle `[x, y, w, h]` for one image (bottom-left origin).
fn layout(page_size: &PageSize, margin: f32, img_w: f32, img_h: f32) -> (f32, f32, [f32; 4]) {
    match *page_size {
        PageSize::Fit => (img_w, img_h, [0.0, 0.0, img_w, img_h]),
        PageSize::Fixed(pw, ph) => {
            let max_w = pw - margin * 2.0;
            let max_h = ph - margin * 2.0;
            let scale = (max_w / img_w).min(max_h / img_h).min(1.0);
            let w = img_w * scale;
            let h = img_h * scale;
            (pw, ph, [(pw - w) / 2.0, (ph - h) / 2.0, w, h])
        }
    }
}

struct EmbeddedImage {
    id: ObjectId,
    width: u32,
    height: u32,
}

/// Embed a PNG as a flate-compressed image XObject.
///
/// Alpha channels become a separate DeviceGray `/SMask` stream (the lopdf
/// equivalent of pdf-lib's `embedPng` handling); opaque grayscale stays
/// DeviceGray, everything else is stored as DeviceRGB.
fn embed_png(doc: &mut Document, bytes: &[u8]) -> Result<EmbeddedImage, String> {
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
    Ok(EmbeddedImage { id, width, height })
}

/// Embed a JPEG, preferring DCT passthrough (original bytes, no re-encode) like
/// pdf-lib's `embedJpg`. CMYK/YCCK (4-component) and other unusual scans fall
/// back to flate-compressed decoded RGB pixels, which every viewer renders
/// identically.
fn embed_jpeg(doc: &mut Document, bytes: &[u8]) -> Result<EmbeddedImage, String> {
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
    Ok(EmbeddedImage { id, width, height })
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
///
/// The `image` crate converts CMYK to RGB during decode and reports `Rgb8`
/// either way, so the only reliable way to know whether DCT passthrough with
/// `/DeviceRGB` is safe is to read the Start-of-Frame marker ourselves.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackWriter;
    use crate::pages::page_count_native;
    use crate::util::fixtures::PNG_1X1;
    use crate::util::number_as_f32;
    use lopdf::Dictionary;
    use std::io::Cursor;

    // -- test image builders ------------------------------------------------

    fn encode(img: image::DynamicImage, format: image::ImageFormat) -> Vec<u8> {
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), format).unwrap();
        buf
    }

    fn rgb_png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 251) as u8, (y % 241) as u8, 9])
        });
        encode(image::DynamicImage::ImageRgb8(img), image::ImageFormat::Png)
    }

    fn rgba_png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_fn(w, h, |x, y| {
            image::Rgba([10, 20, 30, (40 + x * 10 + y) as u8])
        });
        encode(
            image::DynamicImage::ImageRgba8(img),
            image::ImageFormat::Png,
        )
    }

    fn gray_png(w: u32, h: u32) -> Vec<u8> {
        let img = image::GrayImage::from_fn(w, h, |x, y| image::Luma([(x + y) as u8]));
        encode(
            image::DynamicImage::ImageLuma8(img),
            image::ImageFormat::Png,
        )
    }

    fn rgb_jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([200, 100, 50]));
        encode(
            image::DynamicImage::ImageRgb8(img),
            image::ImageFormat::Jpeg,
        )
    }

    fn gray_jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = image::GrayImage::from_pixel(w, h, image::Luma([130]));
        encode(
            image::DynamicImage::ImageLuma8(img),
            image::ImageFormat::Jpeg,
        )
    }

    fn pack(images: &[(u8, &[u8])]) -> Vec<u8> {
        let mut w = PackWriter::new(IMAGES_PACK_MAGIC).u32(images.len() as u32);
        for (kind, bytes) in images {
            w = w.u8(*kind).bytes(bytes);
        }
        w.finish()
    }

    // -- output inspection helpers -------------------------------------------

    fn media_box(data: &[u8], n: usize) -> [f32; 4] {
        let doc = Document::load_mem(data).unwrap();
        let pages: Vec<_> = doc.page_iter().collect();
        crate::util::effective_media_box(&doc, pages[n]).unwrap()
    }

    /// The `[a b c d e f]` operands of the page's `cm` operator, i.e.
    /// `[draw_w, 0, 0, draw_h, x, y]`.
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
            out[i] = number_as_f32(operand).unwrap();
        }
        out
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

    /// lopdf refuses to decode Image-subtype streams, so inflate them directly.
    fn inflate(data: &[u8]) -> Vec<u8> {
        use std::io::Read;
        let mut out = Vec::new();
        flate2::read::ZlibDecoder::new(data)
            .read_to_end(&mut out)
            .unwrap();
        out
    }

    fn assert_close(actual: f32, expected: f32, what: &str) {
        assert!(
            (actual - expected).abs() < 0.01,
            "{what}: expected {expected}, got {actual}"
        );
    }

    // -- fit mode --------------------------------------------------------------

    #[test]
    fn fit_page_matches_image_and_draws_at_origin() {
        for opts in ["{}", r#"{"pageSize":"fit"}"#] {
            let out = images_to_pdf_native(&pack(&[(0, &rgb_png(30, 40))]), opts).unwrap();
            assert!(out.starts_with(b"%PDF-"));
            assert_eq!(page_count_native(&out).unwrap(), 1);
            assert_eq!(media_box(&out, 0), [0.0, 0.0, 30.0, 40.0]);
            assert_eq!(cm_matrix(&out, 0), [30.0, 0.0, 0.0, 40.0, 0.0, 0.0]);
        }
    }

    #[test]
    fn fit_handles_multiple_mixed_images_in_order() {
        let images = [
            (0u8, rgb_png(10, 20)),
            (1u8, rgb_jpeg(40, 30)),
            (0u8, rgba_png(5, 6)),
        ];
        let packed: Vec<(u8, &[u8])> = images.iter().map(|(k, b)| (*k, b.as_slice())).collect();
        let out = images_to_pdf_native(&pack(&packed), "{}").unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 3);
        assert_eq!(media_box(&out, 0), [0.0, 0.0, 10.0, 20.0]);
        assert_eq!(media_box(&out, 1), [0.0, 0.0, 40.0, 30.0]);
        assert_eq!(media_box(&out, 2), [0.0, 0.0, 5.0, 6.0]);
    }

    #[test]
    fn fit_ignores_margin() {
        let out =
            images_to_pdf_native(&pack(&[(0, &rgb_png(30, 40))]), r#"{"margin":150}"#).unwrap();
        assert_eq!(media_box(&out, 0), [0.0, 0.0, 30.0, 40.0]);
        assert_eq!(cm_matrix(&out, 0), [30.0, 0.0, 0.0, 40.0, 0.0, 0.0]);
    }

    // -- fixed page sizes -----------------------------------------------------

    #[test]
    fn a4_sets_a4_page_size() {
        let out =
            images_to_pdf_native(&pack(&[(0, &rgb_png(30, 40))]), r#"{"pageSize":"a4"}"#).unwrap();
        let mb = media_box(&out, 0);
        assert_close(mb[2], 595.28, "A4 width");
        assert_close(mb[3], 841.89, "A4 height");
    }

    #[test]
    fn letter_sets_letter_page_size() {
        let out = images_to_pdf_native(&pack(&[(0, &rgb_png(30, 40))]), r#"{"pageSize":"letter"}"#)
            .unwrap();
        let mb = media_box(&out, 0);
        assert_close(mb[2], 612.0, "Letter width");
        assert_close(mb[3], 792.0, "Letter height");
    }

    #[test]
    fn fixed_mode_centers_small_image_without_upscaling() {
        // scale = min(547.28/300, 793.89/400, 1) = 1 -> drawn at natural size, centered
        let out = images_to_pdf_native(&pack(&[(0, &rgb_png(300, 400))]), r#"{"pageSize":"a4"}"#)
            .unwrap();
        let cm = cm_matrix(&out, 0);
        assert_close(cm[0], 300.0, "draw width");
        assert_close(cm[3], 400.0, "draw height");
        assert_close(cm[4], (595.28 - 300.0) / 2.0, "x offset");
        assert_close(cm[5], (841.89 - 400.0) / 2.0, "y offset");
    }

    #[test]
    fn fixed_mode_downscales_tall_image_to_fit_a4() {
        // scale = min(547.28/2000, 793.89/3000, 1) = 793.89/3000 -> height-bound
        let out = images_to_pdf_native(&pack(&[(0, &rgb_png(2000, 3000))]), r#"{"pageSize":"a4"}"#)
            .unwrap();
        let cm = cm_matrix(&out, 0);
        assert_close(cm[3], 841.89 - 48.0, "draw height fills margin box");
        assert_close(cm[5], 24.0, "y offset equals margin");
        assert_close(cm[0] / cm[3], 2000.0 / 3000.0, "aspect ratio preserved");
        assert!(cm[0] < 595.28 - 48.0, "width stays inside margins");
    }

    #[test]
    fn fixed_mode_downscales_wide_image_to_fit_letter() {
        // scale = min(564/3000, 744/2000, 1) = 564/3000 -> width-bound
        let out = images_to_pdf_native(
            &pack(&[(1, &rgb_jpeg(3000, 2000))]),
            r#"{"pageSize":"letter"}"#,
        )
        .unwrap();
        let cm = cm_matrix(&out, 0);
        assert_close(cm[0], 564.0, "draw width fills margin box");
        assert_close(cm[3], 376.0, "draw height scales with aspect");
        assert_close(cm[4], 24.0, "x offset equals margin");
        assert_close(cm[5], (792.0 - 376.0) / 2.0, "y offset centers vertically");
    }

    #[test]
    fn margin_zero_uses_full_page() {
        let out = images_to_pdf_native(
            &pack(&[(0, &rgb_png(1000, 1000))]),
            r#"{"pageSize":"a4","margin":0}"#,
        )
        .unwrap();
        let cm = cm_matrix(&out, 0);
        assert_close(cm[0], 595.28, "draw width fills page width");
        assert_close(cm[4], 0.0, "x offset");
        assert_close(
            cm[5],
            (841.89 - 595.28) / 2.0,
            "y offset centers vertically",
        );
    }

    #[test]
    fn large_margin_constrains_image() {
        let out = images_to_pdf_native(
            &pack(&[(0, &rgb_png(300, 400))]),
            r#"{"pageSize":"a4","margin":200}"#,
        )
        .unwrap();
        let cm = cm_matrix(&out, 0);
        assert_close(cm[0], 595.28 - 400.0, "draw width fills shrunken box");
        assert_close(cm[4], 200.0, "x offset equals margin");
    }

    #[test]
    fn margin_clamps_to_supported_range() {
        let png = rgb_png(300, 400);
        let over = images_to_pdf_native(&pack(&[(0, &png)]), r#"{"pageSize":"a4","margin":5000}"#)
            .unwrap();
        let max =
            images_to_pdf_native(&pack(&[(0, &png)]), r#"{"pageSize":"a4","margin":200}"#).unwrap();
        assert_eq!(cm_matrix(&over, 0), cm_matrix(&max, 0));

        let negative =
            images_to_pdf_native(&pack(&[(0, &png)]), r#"{"pageSize":"a4","margin":-50}"#).unwrap();
        let zero =
            images_to_pdf_native(&pack(&[(0, &png)]), r#"{"pageSize":"a4","margin":0}"#).unwrap();
        assert_eq!(cm_matrix(&negative, 0), cm_matrix(&zero, 0));
    }

    // -- option / pack validation ----------------------------------------------

    #[test]
    fn rejects_unknown_page_size() {
        let err =
            images_to_pdf_native(&pack(&[(0, PNG_1X1)]), r#"{"pageSize":"tabloid"}"#).unwrap_err();
        assert_eq!(err, "Invalid page size option");
    }

    #[test]
    fn rejects_invalid_options_json() {
        let err = images_to_pdf_native(&pack(&[(0, PNG_1X1)]), "not json").unwrap_err();
        assert!(err.starts_with("Invalid options:"), "got: {err}");
    }

    #[test]
    fn rejects_empty_image_list() {
        let empty = PackWriter::new(IMAGES_PACK_MAGIC).u32(0).finish();
        assert_eq!(
            images_to_pdf_native(&empty, "{}").unwrap_err(),
            "No images provided"
        );
    }

    #[test]
    fn rejects_wrong_magic() {
        let bad = PackWriter::new(b"NOPE")
            .u32(1)
            .u8(0)
            .bytes(PNG_1X1)
            .finish();
        assert_eq!(
            images_to_pdf_native(&bad, "{}").unwrap_err(),
            "Invalid input pack"
        );
        assert_eq!(
            images_to_pdf_native(b"", "{}").unwrap_err(),
            "Input pack ended early"
        );
    }

    #[test]
    fn rejects_truncated_pack() {
        // Declares two images but only carries one.
        let truncated = PackWriter::new(IMAGES_PACK_MAGIC)
            .u32(2)
            .u8(0)
            .bytes(PNG_1X1)
            .finish();
        assert_eq!(
            images_to_pdf_native(&truncated, "{}").unwrap_err(),
            "Input pack ended early"
        );
    }

    #[test]
    fn rejects_trailing_data() {
        let mut padded = pack(&[(0, PNG_1X1)]);
        padded.push(0);
        assert_eq!(
            images_to_pdf_native(&padded, "{}").unwrap_err(),
            "Input pack has trailing data"
        );
    }

    #[test]
    fn rejects_bad_image_payloads() {
        let err = images_to_pdf_native(&pack(&[(7, PNG_1X1)]), "{}").unwrap_err();
        assert_eq!(err, "Unknown image kind in input pack");

        let err = images_to_pdf_native(&pack(&[(0, b"garbage")]), "{}").unwrap_err();
        assert!(err.starts_with("Invalid image data:"), "got: {err}");

        // Declared JPEG, actually PNG bytes.
        let err = images_to_pdf_native(&pack(&[(1, PNG_1X1)]), "{}").unwrap_err();
        assert!(err.starts_with("Invalid image data:"), "got: {err}");

        let err = images_to_pdf_native(&pack(&[(0, b"")]), "{}").unwrap_err();
        assert!(err.starts_with("Invalid image data:"), "got: {err}");
    }

    // -- embedding details -------------------------------------------------------

    #[test]
    fn jpeg_embedded_via_dctdecode_passthrough() {
        let jpeg = rgb_jpeg(8, 4);
        let out = images_to_pdf_native(&pack(&[(1, &jpeg)]), "{}").unwrap();
        let (dict, content) = image_xobject(&out, 0);
        assert_eq!(name_of(&dict, b"Filter"), "DCTDecode");
        assert_eq!(name_of(&dict, b"ColorSpace"), "DeviceRGB");
        assert_eq!(dict.get(b"Width").unwrap().as_i64().unwrap(), 8);
        assert_eq!(dict.get(b"Height").unwrap().as_i64().unwrap(), 4);
        assert_eq!(
            content, jpeg,
            "original JPEG bytes must pass through unchanged"
        );
    }

    #[test]
    fn grayscale_jpeg_uses_devicegray_passthrough() {
        let jpeg = gray_jpeg(6, 6);
        assert_eq!(jpeg_component_count(&jpeg), Some(1));
        let out = images_to_pdf_native(&pack(&[(1, &jpeg)]), "{}").unwrap();
        let (dict, content) = image_xobject(&out, 0);
        assert_eq!(name_of(&dict, b"Filter"), "DCTDecode");
        assert_eq!(name_of(&dict, b"ColorSpace"), "DeviceGray");
        assert_eq!(content, jpeg);
    }

    #[test]
    fn jpeg_component_parser_handles_marker_edge_cases() {
        let sof_rgb = [
            0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01, 0x03,
        ];
        assert_eq!(jpeg_component_count(&sof_rgb), Some(3));

        let fill_bytes_before_sof = [
            0xFF, 0xD8, 0xFF, 0xFF, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01, 0x04,
        ];
        assert_eq!(jpeg_component_count(&fill_bytes_before_sof), Some(4));

        let restart_before_sof = [
            0xFF, 0xD8, 0xFF, 0xD0, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01, 0x01,
        ];
        assert_eq!(jpeg_component_count(&restart_before_sof), Some(1));

        assert_eq!(
            jpeg_component_count(&[0xFF, 0xD8, 0x00, 0xC0, 0x00, 0x00]),
            None
        );
        assert_eq!(
            jpeg_component_count(&[0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x08]),
            None
        );
        assert_eq!(
            jpeg_component_count(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x02]),
            None
        );
    }

    #[test]
    fn png_with_alpha_gets_flate_smask() {
        let out = images_to_pdf_native(&pack(&[(0, &rgba_png(3, 2))]), "{}").unwrap();
        let (dict, _) = image_xobject(&out, 0);
        assert_eq!(name_of(&dict, b"Filter"), "FlateDecode");
        assert_eq!(name_of(&dict, b"ColorSpace"), "DeviceRGB");

        let doc = Document::load_mem(&out).unwrap();
        let smask_id = dict.get(b"SMask").unwrap().as_reference().unwrap();
        let smask = doc.get_object(smask_id).unwrap().as_stream().unwrap();
        assert_eq!(name_of(&smask.dict, b"ColorSpace"), "DeviceGray");
        // rgba_png alpha = 40 + x*10 + y, rows top to bottom
        let expected_alpha = vec![40, 50, 60, 41, 51, 61];
        assert_eq!(inflate(&smask.content), expected_alpha);

        // The 1x1 LA fixture also has alpha and must take the SMask path.
        let out = images_to_pdf_native(&pack(&[(0, PNG_1X1)]), "{}").unwrap();
        assert_eq!(media_box(&out, 0), [0.0, 0.0, 1.0, 1.0]);
        let (dict, _) = image_xobject(&out, 0);
        assert!(dict.has(b"SMask"));
    }

    #[test]
    fn opaque_png_is_flate_rgb_without_smask() {
        let png = rgb_png(4, 3);
        let out = images_to_pdf_native(&pack(&[(0, &png)]), "{}").unwrap();
        let (dict, content) = image_xobject(&out, 0);
        assert_eq!(name_of(&dict, b"Filter"), "FlateDecode");
        assert_eq!(name_of(&dict, b"ColorSpace"), "DeviceRGB");
        assert!(!dict.has(b"SMask"));

        let raw = image::load_from_memory(&png).unwrap().into_rgb8();
        assert_eq!(inflate(&content), *raw.as_raw());
    }

    #[test]
    fn grayscale_png_uses_devicegray() {
        let out = images_to_pdf_native(&pack(&[(0, &gray_png(4, 4))]), "{}").unwrap();
        let (dict, _) = image_xobject(&out, 0);
        assert_eq!(name_of(&dict, b"ColorSpace"), "DeviceGray");
        assert_eq!(name_of(&dict, b"Filter"), "FlateDecode");
        assert!(!dict.has(b"SMask"));
    }

    // -- output validity ------------------------------------------------------

    #[test]
    fn output_reparses_and_draws_on_every_page() {
        let images: Vec<Vec<u8>> = (1..=5).map(|i| rgb_png(10 * i, 8 * i)).collect();
        let packed: Vec<(u8, &[u8])> = images.iter().map(|b| (0u8, b.as_slice())).collect();
        let out =
            images_to_pdf_native(&pack(&packed), r#"{"pageSize":"letter","margin":12}"#).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count_native(&out).unwrap(), 5);

        let doc = Document::load_mem(&out).unwrap();
        for (n, page_id) in doc.page_iter().enumerate() {
            let content = Content::decode(&doc.get_page_content(page_id).unwrap()).unwrap();
            let ops: Vec<&str> = content
                .operations
                .iter()
                .map(|o| o.operator.as_str())
                .collect();
            assert_eq!(
                ops,
                ["q", "cm", "Do", "Q"],
                "page {n} draws exactly one image"
            );
        }
    }
}
