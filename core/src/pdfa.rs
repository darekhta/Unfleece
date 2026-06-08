//! PDF/A-2b emission from rendered page images (krilla).
//!
//! The browser owns page rendering; Rust owns archival PDF emission.

use crate::pack::PackReader;

const PDFA_PACK_MAGIC: &[u8; 4] = b"UFPA";

/// Build a PDF/A-2b file from a packed list of PNG page images.
///
/// Pack format, little-endian:
/// - `UFPA` magic
/// - u32 page count
/// - u16/u8/u8/u8/u8/u8 UTC creation timestamp (year, month, day, hour, minute, second)
/// - repeated pages: f32 width_pt, f32 height_pt, u32 png_len, png bytes
pub fn pdfa_from_png_pages_native(pack: &[u8]) -> Result<Vec<u8>, String> {
    use krilla::configure::{Archival, ConfigurationBuilder};
    use krilla::geom::Size;
    use krilla::image::Image;
    use krilla::metadata::{DateTime, Metadata};
    use krilla::page::PageSettings;
    use krilla::{Document as KrillaDocument, SerializeSettings};

    let mut reader = PackReader::new(pack);
    reader
        .expect_magic(PDFA_PACK_MAGIC)
        .map_err(|_| "Invalid PDF/A page pack".to_string())?;

    let page_count = reader.read_u32()? as usize;
    if page_count == 0 {
        return Err("PDF/A export needs at least one page".to_string());
    }

    let year = reader.read_u16()?;
    let month = reader.read_u8()?;
    let day = reader.read_u8()?;
    let hour = reader.read_u8()?;
    let minute = reader.read_u8()?;
    let second = reader.read_u8()?;

    let configuration = ConfigurationBuilder::new()
        .with_archival_validator(Archival::A2_B)
        .finish()
        .map_err(|e| format!("PDF/A configuration failed: {e:?}"))?;
    let settings = SerializeSettings {
        configuration,
        ..Default::default()
    };

    let mut document = KrillaDocument::new_with(settings);
    document.set_metadata(
        Metadata::new()
            .title("Unfleece PDF/A export".to_string())
            .creator("Unfleece".to_string())
            .producer("Unfleece".to_string())
            .language("en".to_string())
            .creation_date(
                DateTime::new(year)
                    .month(month)
                    .day(day)
                    .hour(hour)
                    .minute(minute)
                    .second(second)
                    .utc_offset_hour(0)
                    .utc_offset_minute(0),
            ),
    );

    for _ in 0..page_count {
        let width = reader.read_f32()?;
        let height = reader.read_f32()?;
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err("Invalid PDF/A page size".to_string());
        }

        let png = reader.read_bytes()?.to_vec();
        let image = Image::from_png(png.into(), false)
            .map_err(|e| format!("Invalid rendered page image: {e}"))?;
        let page_settings = PageSettings::from_wh(width, height)
            .ok_or_else(|| "Invalid PDF/A page size".to_string())?;
        let size =
            Size::from_wh(width, height).ok_or_else(|| "Invalid PDF/A image size".to_string())?;

        let mut page = document.start_page_with(page_settings);
        let mut surface = page.surface();
        surface.draw_image(image, size);
        surface.finish();
        page.finish();
    }

    reader
        .expect_done()
        .map_err(|_| "PDF/A page pack has trailing data".to_string())?;

    document
        .finish()
        .map_err(|e| format!("PDF/A export failed validation: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackWriter;
    use crate::pages::page_count_native;
    use crate::util::fixtures::PNG_1X1;

    fn pdfa_pack(png: &[u8], width: f32, height: f32) -> Vec<u8> {
        PackWriter::new(PDFA_PACK_MAGIC)
            .u32(1)
            .u16(2026)
            .u8(1)
            .u8(2)
            .u8(3)
            .u8(4)
            .u8(5)
            .f32(width)
            .f32(height)
            .bytes(png)
            .finish()
    }

    #[test]
    fn pdfa_from_png_pages_emits_pdfa_pdf() {
        let out = pdfa_from_png_pages_native(&pdfa_pack(PNG_1X1, 100.0, 120.0)).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count_native(&out).unwrap(), 1);
        assert!(out
            .windows(b"pdfaid:part".len())
            .any(|w| w == b"pdfaid:part"));
    }

    #[test]
    fn rejects_zero_pages_and_bad_sizes() {
        let empty = PackWriter::new(PDFA_PACK_MAGIC)
            .u32(0)
            .u16(2026)
            .u8(1)
            .u8(2)
            .u8(3)
            .u8(4)
            .u8(5)
            .finish();
        assert!(pdfa_from_png_pages_native(&empty).is_err());
        assert!(pdfa_from_png_pages_native(&pdfa_pack(PNG_1X1, 0.0, 100.0)).is_err());
        assert!(pdfa_from_png_pages_native(&pdfa_pack(PNG_1X1, f32::NAN, 100.0)).is_err());
    }

    #[test]
    fn rejects_invalid_magic_and_trailing_data() {
        assert!(pdfa_from_png_pages_native(b"XXXX").is_err());
        let mut pack = pdfa_pack(PNG_1X1, 100.0, 120.0);
        pack.push(0);
        assert!(pdfa_from_png_pages_native(&pack).is_err());
    }
}
