//! Image XObject embedding — the single home for turning PNG/JPEG bytes into
//! lopdf image XObjects.
//!
//! This logic was previously copy-pasted into `images_to_pdf.rs` and
//! `assemble.rs`; both now call these `pub(crate)` helpers so the embedding code
//! — and, crucially, its byte-level output — lives in exactly one place.
//!
//! Behavior (matching pdf-lib's `embedPng` / `embedJpg`):
//! - **PNG** → `FlateDecode`. Opaque grayscale stays `DeviceGray`; everything
//!   else is `DeviceRGB`; an alpha channel becomes a separate `DeviceGray`
//!   `/SMask` stream.
//! - **JPEG** → `DCTDecode` passthrough (original bytes, never re-encoded) for
//!   plain 1- or 3-component scans; CMYK/exotic scans fall back to decoded-RGB
//!   `FlateDecode`, which every viewer renders identically.
//!
//! `stamp_image.rs` deliberately keeps its *own* embedding (always-`DeviceRGB`
//! PNG, CMYK DCT passthrough via `original_color_type`, distinct error messages,
//! and a different dictionary key order) and so only borrows [`flate_compress`]
//! from here; the rest of this module is the canonical builder/embedder used by
//! the image-only PDF tools.

use lopdf::{dictionary, Document, ObjectId, Stream};

/// An image embedded into the document, with its source pixel dimensions.
#[derive(Debug)]
pub(crate) struct EmbeddedImage {
    pub id: ObjectId,
    pub width: u32,
    pub height: u32,
}

/// Embed a PNG as a flate-compressed image XObject.
///
/// Alpha channels become a separate DeviceGray `/SMask` stream (the lopdf
/// equivalent of pdf-lib's `embedPng` handling); opaque grayscale stays
/// DeviceGray, everything else is stored as DeviceRGB.
pub(crate) fn embed_png(doc: &mut Document, bytes: &[u8]) -> Result<EmbeddedImage, String> {
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
pub(crate) fn embed_jpeg(doc: &mut Document, bytes: &[u8]) -> Result<EmbeddedImage, String> {
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
pub(crate) fn flate_image_stream(
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
pub(crate) fn dct_image_stream(width: u32, height: u32, color_space: &str, jpeg: &[u8]) -> Stream {
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

/// Zlib-compress raw bytes for a `/FlateDecode` stream.
pub(crate) fn flate_compress(data: &[u8]) -> Result<Vec<u8>, String> {
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
pub(crate) fn jpeg_component_count(bytes: &[u8]) -> Option<u8> {
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
    use std::io::{Cursor, Read};

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

    // -- inspection helpers --------------------------------------------------

    fn name_of(stream: &Stream, key: &[u8]) -> String {
        String::from_utf8(stream.dict.get(key).unwrap().as_name().unwrap().to_vec()).unwrap()
    }

    /// lopdf refuses to decode Image-subtype streams, so inflate them directly.
    fn inflate(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        flate2::read::ZlibDecoder::new(data)
            .read_to_end(&mut out)
            .unwrap();
        out
    }

    // -- PNG embedding ------------------------------------------------------

    #[test]
    fn embed_png_opaque_rgb_is_devicergb_flate_without_smask() {
        let png = rgb_png(4, 3);
        let mut doc = Document::with_version("1.5");
        let emb = embed_png(&mut doc, &png).unwrap();
        assert_eq!((emb.width, emb.height), (4, 3));
        let stream = doc.get_object(emb.id).unwrap().as_stream().unwrap();
        assert_eq!(name_of(stream, b"ColorSpace"), "DeviceRGB");
        assert_eq!(name_of(stream, b"Filter"), "FlateDecode");
        assert!(!stream.dict.has(b"SMask"));
        let raw = image::load_from_memory(&png).unwrap().into_rgb8();
        assert_eq!(inflate(&stream.content), *raw.as_raw());
    }

    #[test]
    fn embed_png_opaque_grayscale_is_devicegray() {
        let png = gray_png(4, 4);
        let mut doc = Document::with_version("1.5");
        let emb = embed_png(&mut doc, &png).unwrap();
        let stream = doc.get_object(emb.id).unwrap().as_stream().unwrap();
        assert_eq!(name_of(stream, b"ColorSpace"), "DeviceGray");
        assert_eq!(name_of(stream, b"Filter"), "FlateDecode");
        assert!(!stream.dict.has(b"SMask"));
    }

    #[test]
    fn embed_png_alpha_gets_devicegray_smask() {
        let png = rgba_png(3, 2);
        let mut doc = Document::with_version("1.5");
        let emb = embed_png(&mut doc, &png).unwrap();
        let main = doc.get_object(emb.id).unwrap().as_stream().unwrap();
        assert_eq!(name_of(main, b"ColorSpace"), "DeviceRGB");
        assert_eq!(name_of(main, b"Filter"), "FlateDecode");
        let smask_id = main.dict.get(b"SMask").unwrap().as_reference().unwrap();
        let smask = doc.get_object(smask_id).unwrap().as_stream().unwrap();
        assert_eq!(name_of(smask, b"ColorSpace"), "DeviceGray");
        // rgba_png alpha = 40 + x*10 + y, rows top to bottom.
        assert_eq!(inflate(&smask.content), vec![40, 50, 60, 41, 51, 61]);
    }

    #[test]
    fn embed_png_rejects_garbage() {
        let mut doc = Document::with_version("1.5");
        let err = embed_png(&mut doc, b"not a png").unwrap_err();
        assert!(err.starts_with("Invalid image data:"), "got: {err}");
    }

    // -- JPEG embedding -----------------------------------------------------

    #[test]
    fn embed_jpeg_rgb_is_dctdecode_passthrough() {
        let jpeg = rgb_jpeg(8, 4);
        let mut doc = Document::with_version("1.5");
        let emb = embed_jpeg(&mut doc, &jpeg).unwrap();
        assert_eq!((emb.width, emb.height), (8, 4));
        let stream = doc.get_object(emb.id).unwrap().as_stream().unwrap();
        assert_eq!(name_of(stream, b"Filter"), "DCTDecode");
        assert_eq!(name_of(stream, b"ColorSpace"), "DeviceRGB");
        assert_eq!(stream.content, jpeg, "JPEG bytes pass through unchanged");
    }

    #[test]
    fn embed_jpeg_grayscale_is_devicegray_passthrough() {
        let jpeg = gray_jpeg(6, 6);
        assert_eq!(jpeg_component_count(&jpeg), Some(1));
        let mut doc = Document::with_version("1.5");
        let emb = embed_jpeg(&mut doc, &jpeg).unwrap();
        let stream = doc.get_object(emb.id).unwrap().as_stream().unwrap();
        assert_eq!(name_of(stream, b"Filter"), "DCTDecode");
        assert_eq!(name_of(stream, b"ColorSpace"), "DeviceGray");
        assert_eq!(stream.content, jpeg);
    }

    // -- SOF parser ---------------------------------------------------------

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

    // -- compression --------------------------------------------------------

    #[test]
    fn flate_compress_round_trips() {
        let data: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
        let compressed = flate_compress(&data).unwrap();
        assert_eq!(inflate(&compressed), data);
    }
}
