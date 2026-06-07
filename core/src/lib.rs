//! unfleece-core — pure-Rust PDF object-graph operations, compiled to WebAssembly.
//!
//! This is the start of Unfleece's "moat": operations we own end-to-end via
//! `lopdf`, rather than wrapping a JS library. Native `cargo test` validates the
//! logic; `wasm-pack build` produces the browser module consumed by the app.
//!
//! The `*_native` functions are plain Rust (testable on the host). The
//! `#[wasm_bindgen]` wrappers (compiled only for wasm32) expose them to JS.

use lopdf::{Document, Error as LopdfError, Object, ObjectId};

const PDFA_PACK_MAGIC: &[u8; 4] = b"UFPA";

/// Rotate every page by `degrees` (relative to its current rotation).
pub fn rotate_all_native(data: &[u8], degrees: i64) -> Result<Vec<u8>, lopdf::Error> {
    let mut doc = Document::load_mem(data)?;
    let page_ids: Vec<_> = doc.get_pages().into_values().collect();
    for id in page_ids {
        if let Ok(obj) = doc.get_object_mut(id) {
            if let Ok(dict) = obj.as_dict_mut() {
                let current = dict.get(b"Rotate").ok().and_then(|o| o.as_i64().ok()).unwrap_or(0);
                let next = (((current + degrees) % 360) + 360) % 360;
                dict.set("Rotate", next);
            }
        }
    }
    let mut buf = Vec::new();
    doc.save_to(&mut buf)?;
    Ok(buf)
}

/// Losslessly optimize: prune unused objects and flate-compress streams.
pub fn optimize_native(data: &[u8]) -> Result<Vec<u8>, lopdf::Error> {
    let mut doc = Document::load_mem(data)?;
    doc.prune_objects();
    doc.compress();
    let mut buf = Vec::new();
    doc.save_to(&mut buf)?;
    Ok(buf)
}

/// Number of pages in the document.
pub fn page_count_native(data: &[u8]) -> Result<usize, lopdf::Error> {
    Ok(Document::load_mem(data)?.get_pages().len())
}

fn root_pages_id(doc: &Document) -> Result<ObjectId, LopdfError> {
    doc.catalog()?
        .get(b"Pages")?
        .as_reference()
}

fn inherited_page_value(doc: &Document, page_id: ObjectId, key: &[u8]) -> Result<Option<Object>, LopdfError> {
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

fn materialize_inherited_page_attrs(doc: &mut Document, page_id: ObjectId) -> Result<(), LopdfError> {
    for key in [b"Resources".as_slice(), b"MediaBox".as_slice(), b"CropBox".as_slice(), b"Rotate".as_slice()] {
        if !doc.get_dictionary(page_id)?.has(key) {
            if let Some(value) = inherited_page_value(doc, page_id, key)? {
                doc.get_object_mut(page_id)?.as_dict_mut()?.set(key.to_vec(), value);
            }
        }
    }
    Ok(())
}

/// Build a new PDF containing the selected 0-based page indices, in order.
///
/// This is intentionally a same-document page-tree rewrite rather than a cross-document
/// merger: all selected pages keep their resource/content references, while unreferenced
/// objects are pruned before writing.
pub fn select_pages_native(data: &[u8], indices: &[u32]) -> Result<Vec<u8>, lopdf::Error> {
    if indices.is_empty() {
        return Err(LopdfError::Syntax("No pages selected".to_string()));
    }

    let mut doc = Document::load_mem(data)?;
    let pages: Vec<ObjectId> = doc.page_iter().collect();
    let pages_id = root_pages_id(&doc)?;
    let mut kids = Vec::with_capacity(indices.len());

    for &index in indices {
        let page_id = *pages
            .get(index as usize)
            .ok_or_else(|| LopdfError::PageNumberNotFound(index + 1))?;
        materialize_inherited_page_attrs(&mut doc, page_id)?;
        doc.get_object_mut(page_id)?
            .as_dict_mut()?
            .set("Parent", pages_id);
        kids.push(Object::Reference(page_id));
    }

    let page_tree = doc.get_object_mut(pages_id)?.as_dict_mut()?;
    page_tree.set("Kids", kids);
    page_tree.set("Count", indices.len() as i64);

    if let Ok(catalog) = doc.catalog_mut() {
        // Page labels/outlines/actions/forms can point at removed pages. pdf-lib's
        // copyPages path also drops these document-level structures, so keep parity.
        for key in [b"Outlines".as_slice(), b"PageLabels".as_slice(), b"OpenAction".as_slice(), b"AA".as_slice(), b"Names".as_slice(), b"Dests".as_slice(), b"AcroForm".as_slice()] {
            catalog.remove(key);
        }
    }

    doc.prune_objects();
    doc.compress();
    doc.renumber_objects();
    let mut buf = Vec::new();
    doc.save_to(&mut buf)?;
    Ok(buf)
}

struct PdfaPackReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> PdfaPackReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| "PDF/A page pack is too large".to_string())?;
        if end > self.data.len() {
            return Err("PDF/A page pack ended early".to_string());
        }
        let out = &self.data[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let mut bytes = [0; 2];
        bytes.copy_from_slice(self.read_exact(2)?);
        Ok(u16::from_le_bytes(bytes))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let mut bytes = [0; 4];
        bytes.copy_from_slice(self.read_exact(4)?);
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_f32(&mut self) -> Result<f32, String> {
        let mut bytes = [0; 4];
        bytes.copy_from_slice(self.read_exact(4)?);
        Ok(f32::from_le_bytes(bytes))
    }
}

/// Build a PDF/A-2b file from a packed list of PNG page images.
///
/// The browser owns page rendering; Rust owns PDF/A emission through krilla.
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

    let mut reader = PdfaPackReader::new(pack);
    if reader.read_exact(PDFA_PACK_MAGIC.len())? != PDFA_PACK_MAGIC {
        return Err("Invalid PDF/A page pack".to_string());
    }

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
    let mut settings = SerializeSettings::default();
    settings.configuration = configuration;

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

        let png_len = reader.read_u32()? as usize;
        let png = reader.read_exact(png_len)?.to_vec();
        let image = Image::from_png(png.into(), false)
            .map_err(|e| format!("Invalid rendered page image: {e}"))?;
        let page_settings = PageSettings::from_wh(width, height)
            .ok_or_else(|| "Invalid PDF/A page size".to_string())?;
        let size = Size::from_wh(width, height)
            .ok_or_else(|| "Invalid PDF/A image size".to_string())?;

        let mut page = document.start_page_with(page_settings);
        let mut surface = page.surface();
        surface.draw_image(image, size);
        surface.finish();
        page.finish();
    }

    if reader.pos != pack.len() {
        return Err("PDF/A page pack has trailing data".to_string());
    }

    document
        .finish()
        .map_err(|e| format!("PDF/A export failed validation: {e:?}"))
}

// ---------------------------------------------------------------------------
// wasm bindings (compiled only for wasm32)
// ---------------------------------------------------------------------------
#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;

    fn map<T>(r: Result<T, lopdf::Error>) -> Result<T, JsError> {
        r.map_err(|e| JsError::new(&e.to_string()))
    }

    /// Rotate every page by `degrees`. Returns the new PDF bytes.
    #[wasm_bindgen]
    pub fn rotate_all(data: &[u8], degrees: i64) -> Result<Vec<u8>, JsError> {
        map(super::rotate_all_native(data, degrees))
    }

    /// Losslessly optimize the PDF. Returns the new PDF bytes.
    #[wasm_bindgen]
    pub fn optimize(data: &[u8]) -> Result<Vec<u8>, JsError> {
        map(super::optimize_native(data))
    }

    /// Count pages.
    #[wasm_bindgen]
    pub fn page_count(data: &[u8]) -> Result<usize, JsError> {
        map(super::page_count_native(data))
    }

    /// Build a new PDF containing the selected 0-based page indices, in order.
    #[wasm_bindgen]
    pub fn select_pages(data: &[u8], indices: &[u32]) -> Result<Vec<u8>, JsError> {
        map(super::select_pages_native(data, indices))
    }

    /// Build a PDF/A-2b file from a packed list of PNG page images.
    #[wasm_bindgen]
    pub fn pdfa_from_png_pages(pack: &[u8]) -> Result<Vec<u8>, JsError> {
        super::pdfa_from_png_pages_native(pack).map_err(|e| JsError::new(&e))
    }
}

// ---------------------------------------------------------------------------
// native tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Content;
    use lopdf::{dictionary, Document, Object, Stream};

    const PNG_1X1: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
        8, 4, 0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 96, 0,
        0, 0, 6, 0, 2, 48, 129, 208, 47, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    /// Build a minimal valid `pages`-page PDF for testing.
    fn sample(pages: usize) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();

        let mut kids: Vec<Object> = Vec::new();
        for i in 0..pages {
            let content = Content { operations: vec![] };
            let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
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

    #[test]
    fn counts_pages() {
        let pdf = sample(3);
        assert_eq!(page_count_native(&pdf).unwrap(), 3);
    }

    #[test]
    fn rotates_all_pages() {
        let pdf = sample(2);
        let out = rotate_all_native(&pdf, 90).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        for (_, id) in doc.get_pages() {
            let dict = doc.get_object(id).unwrap().as_dict().unwrap();
            assert_eq!(dict.get(b"Rotate").unwrap().as_i64().unwrap(), 90);
        }
    }

    #[test]
    fn rotation_wraps_modulo_360() {
        let pdf = sample(1);
        let once = rotate_all_native(&pdf, 270).unwrap();
        let twice = rotate_all_native(&once, 180).unwrap(); // 270 + 180 = 450 -> 90
        let doc = Document::load_mem(&twice).unwrap();
        let id = *doc.get_pages().values().next().unwrap();
        let dict = doc.get_object(id).unwrap().as_dict().unwrap();
        assert_eq!(dict.get(b"Rotate").unwrap().as_i64().unwrap(), 90);
    }

    #[test]
    fn optimize_preserves_pages_and_validity() {
        let pdf = sample(4);
        let out = optimize_native(&pdf).unwrap();
        assert_eq!(page_count_native(&out).unwrap(), 4);
        assert!(out.starts_with(b"%PDF-"));
    }

    fn first_page_width(data: &[u8]) -> i64 {
        let doc = Document::load_mem(data).unwrap();
        let first = *doc.get_pages().values().next().unwrap();
        let media_box = doc
            .get_object(first)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap();
        media_box[2].as_i64().unwrap()
    }

    #[test]
    fn select_pages_extracts_subset_in_requested_order() {
        let pdf = sample(4);
        let out = select_pages_native(&pdf, &[2, 0]).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count_native(&out).unwrap(), 2);
        assert_eq!(first_page_width(&out), 302);
    }

    fn pdfa_pack(png: &[u8], width: f32, height: f32) -> Vec<u8> {
        let mut pack = Vec::new();
        pack.extend_from_slice(PDFA_PACK_MAGIC);
        pack.extend_from_slice(&1_u32.to_le_bytes());
        pack.extend_from_slice(&2026_u16.to_le_bytes());
        pack.extend_from_slice(&[1, 2, 3, 4, 5]);
        pack.extend_from_slice(&width.to_le_bytes());
        pack.extend_from_slice(&height.to_le_bytes());
        pack.extend_from_slice(&(png.len() as u32).to_le_bytes());
        pack.extend_from_slice(png);
        pack
    }

    #[test]
    fn pdfa_from_png_pages_emits_pdfa_pdf() {
        let out = pdfa_from_png_pages_native(&pdfa_pack(PNG_1X1, 100.0, 120.0)).unwrap();
        assert!(out.starts_with(b"%PDF-"));
        assert_eq!(page_count_native(&out).unwrap(), 1);
        assert!(out.windows(b"pdfaid:part".len()).any(|w| w == b"pdfaid:part"));
    }
}
