//! unfleece-core — pure-Rust PDF engine, compiled to WebAssembly.
//!
//! This crate is Unfleece's "moat": every PDF operation that does not require
//! page *rendering* is owned here, built on `lopdf` (object graph) and `krilla`
//! (archival generation) rather than wrapping a JS library. Native `cargo test`
//! validates the logic; `wasm-pack build` produces the browser module.
//!
//! Layout:
//! - `pack`        — length-prefixed binary packs crossing the JS↔WASM boundary
//! - `util`        — shared lopdf helpers + test fixtures
//! - `pages`       — rotate / count / select / reorder
//! - `pdfa`        — PDF/A-2b emission (krilla)
//!
//! Convention: every operation is a plain-Rust `*_native` function (host-testable),
//! re-exported at the crate root, with a thin `#[wasm_bindgen]` wrapper in the
//! `wasm` module compiled only for `wasm32`.

pub mod pack;
pub mod util;

pub mod assemble;
pub mod boxes;
pub mod crypto;
pub mod epub;
pub mod forms;
pub mod images_to_pdf;
pub mod impose;
pub mod merge;
pub mod meta;
pub mod office;
pub mod organize_extra;
pub mod pages;
pub mod pdfa;
pub mod sanitize;
pub mod stamp_image;
pub mod stamp_text;
pub mod text_pdf;

#[cfg(test)]
mod fuzz_tests;

pub use merge::merge_pdfs_native;
pub use pages::{
    optimize_native, page_count_native, rotate_all_native, rotate_pages_native, select_pages_native,
};
pub use pdfa::pdfa_from_png_pages_native;
// organize_extra: contract tests only — no public *_native functions to re-export (organize.ts reduces to pages.rs fns + TS index math)
pub use assemble::{add_text_layer_native, image_pages_to_pdf_native, set_crop_boxes_native};
pub use boxes::crop_margins_native;
pub use crypto::{is_encrypted_native, protect_native, unlock_native};
pub use epub::{fixed_pages_to_epub_native, text_pages_to_epub_native};
pub use forms::{fill_form_native, flatten_form_native, list_form_fields_native};
pub use images_to_pdf::images_to_pdf_native;
pub use impose::{booklet_native, n_up_native};
pub use meta::{read_metadata_native, set_metadata_native, strip_metadata_native};
pub use office::{text_pages_to_docx_native, text_pages_to_pptx_native, text_pages_to_xlsx_native};
pub use sanitize::{sanitize_native, sanitize_report_native};
pub use stamp_image::stamp_images_native;
pub use stamp_text::{add_page_numbers_native, add_watermark_native};
pub use text_pdf::text_to_pdf_native;

// ---------------------------------------------------------------------------
// wasm bindings (compiled only for wasm32)
// ---------------------------------------------------------------------------
#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;

    fn map<T>(r: Result<T, lopdf::Error>) -> Result<T, JsError> {
        r.map_err(|e| JsError::new(&e.to_string()))
    }

    fn map_str<T>(r: Result<T, String>) -> Result<T, JsError> {
        r.map_err(|e| JsError::new(&e))
    }

    /// Rotate every page by `degrees`. Returns the new PDF bytes.
    #[wasm_bindgen]
    pub fn rotate_all(data: &[u8], degrees: i64) -> Result<Vec<u8>, JsError> {
        map(super::rotate_all_native(data, degrees))
    }

    /// Rotate only the given 0-based pages by `degrees`.
    #[wasm_bindgen]
    pub fn rotate_pages(data: &[u8], indices: &[u32], degrees: i64) -> Result<Vec<u8>, JsError> {
        map(super::rotate_pages_native(data, indices, degrees))
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
        map_str(super::pdfa_from_png_pages_native(pack))
    }

    /// Merge multiple PDFs (packed as `UFMG`) into one document, in pack order.
    #[wasm_bindgen]
    pub fn merge_pdfs(pack: &[u8]) -> Result<Vec<u8>, JsError> {
        map_str(super::merge_pdfs_native(pack))
    }

    // organize_extra: no new wasm bindings. organize.ts uses the existing bindings
    // select_pages / rotate_all / rotate_pages / page_count (all already exposed in
    // the wasm module); the remaining organize logic (complement, permutation
    // validation, split loops, angle % 90 validation, progress) stays TS-side.

    /// Trim margins (in points) from every page by setting the page CropBox.
    #[wasm_bindgen]
    pub fn crop_margins(
        data: &[u8],
        top: f32,
        right: f32,
        bottom: f32,
        left: f32,
    ) -> Result<Vec<u8>, JsError> {
        map_str(super::crop_margins_native(data, top, right, bottom, left))
    }

    /// Read document metadata as a JSON string
    /// (`{title?, author?, subject?, keywords?, creator?, producer?, creationDate?, modificationDate?, pageCount}`).
    #[wasm_bindgen]
    pub fn read_metadata(data: &[u8]) -> Result<String, JsError> {
        map_str(super::read_metadata_native(data))
    }

    /// Overwrite the metadata fields present in the options JSON; an empty string clears a field.
    #[wasm_bindgen]
    pub fn set_metadata(data: &[u8], json: &str) -> Result<Vec<u8>, JsError> {
        map_str(super::set_metadata_native(data, json))
    }

    /// Remove the Info dictionary and the XMP metadata stream (privacy hygiene).
    #[wasm_bindgen]
    pub fn strip_metadata(data: &[u8]) -> Result<Vec<u8>, JsError> {
        map_str(super::strip_metadata_native(data))
    }

    /// Strip metadata, scripts, embedded files and other risky structures.
    /// `opts_json` mirrors the sanitize-pdf tool options (`removeAnnotations`,
    /// `removeForms`; both default to true).
    #[wasm_bindgen]
    pub fn sanitize(data: &[u8], opts_json: &str) -> Result<Vec<u8>, JsError> {
        map_str(super::sanitize_native(data, opts_json))
    }

    /// JSON report of what a default sanitize pass would remove, without
    /// producing output bytes.
    #[wasm_bindgen]
    pub fn sanitize_report(data: &[u8]) -> Result<String, JsError> {
        map_str(super::sanitize_report_native(data))
    }

    /// Stamp page numbers onto every page (JSON options: format with `{n}`/`{total}`,
    /// 6-zone position, fontSize, margin, startAt, padTo — also powers Bates numbering
    /// via `format: "PREFIX{n}"`).
    #[wasm_bindgen]
    pub fn add_page_numbers(data: &[u8], opts_json: &str) -> Result<Vec<u8>, JsError> {
        map_str(super::add_page_numbers_native(data, opts_json))
    }

    /// Draw a rotated, semi-transparent text watermark centered on every page
    /// (JSON options: text, fontSize, opacity, angle, color).
    #[wasm_bindgen]
    pub fn add_watermark(data: &[u8], opts_json: &str) -> Result<Vec<u8>, JsError> {
        map_str(super::add_watermark_native(data, opts_json))
    }

    /// Stamp PNG/JPEG images (e.g. signatures) onto pages, per the `UFST` pack.
    #[wasm_bindgen]
    pub fn stamp_images(data: &[u8], pack: &[u8]) -> Result<Vec<u8>, JsError> {
        map_str(super::stamp_images_native(data, pack))
    }

    /// Build a PDF from a packed list of PNG/JPEG images, one page per image.
    /// `opts_json` mirrors imagesToPdf.ts: `{"pageSize":"fit"|"a4"|"letter","margin":24}`.
    #[wasm_bindgen]
    pub fn images_to_pdf(pack: &[u8], opts_json: &str) -> Result<Vec<u8>, JsError> {
        map_str(super::images_to_pdf_native(pack, opts_json))
    }

    /// Place `per_sheet` source pages onto each A4 sheet (handout-style N-up).
    #[wasm_bindgen]
    pub fn n_up(data: &[u8], per_sheet: u32, opts_json: &str) -> Result<Vec<u8>, JsError> {
        map_str(super::n_up_native(data, per_sheet, opts_json))
    }

    /// Reorder pages into two-up booklet spreads on landscape sheets.
    #[wasm_bindgen]
    pub fn booklet(data: &[u8], opts_json: &str) -> Result<Vec<u8>, JsError> {
        map_str(super::booklet_native(data, opts_json))
    }

    /// Build an image-only PDF (one page per packed image) — redact / raster compress.
    #[wasm_bindgen]
    pub fn image_pages_to_pdf(pack: &[u8]) -> Result<Vec<u8>, JsError> {
        map_str(super::image_pages_to_pdf_native(pack))
    }

    /// Append an invisible Helvetica text layer onto existing pages (OCR).
    #[wasm_bindgen]
    pub fn add_text_layer(data: &[u8], pack: &[u8]) -> Result<Vec<u8>, JsError> {
        map_str(super::add_text_layer_native(data, pack))
    }

    /// Set a per-page CropBox on listed pages (auto-crop).
    #[wasm_bindgen]
    pub fn set_crop_boxes(data: &[u8], pack: &[u8]) -> Result<Vec<u8>, JsError> {
        map_str(super::set_crop_boxes_native(data, pack))
    }

    /// List AcroForm fields as a JSON array of `{name, type, options?, value?}`.
    #[wasm_bindgen]
    pub fn list_form_fields(data: &[u8]) -> Result<String, JsError> {
        map_str(super::list_form_fields_native(data))
    }

    /// Fill form fields from a `{fieldName: string|bool}` JSON object,
    /// regenerating appearances and clearing NeedAppearances.
    #[wasm_bindgen]
    pub fn fill_form(data: &[u8], values_json: &str) -> Result<Vec<u8>, JsError> {
        map_str(super::fill_form_native(data, values_json))
    }

    /// Flatten the form: bake appearances into page content and drop widgets + AcroForm.
    #[wasm_bindgen]
    pub fn flatten_form(data: &[u8]) -> Result<Vec<u8>, JsError> {
        map_str(super::flatten_form_native(data))
    }

    /// Report whether the PDF carries an `/Encrypt` dictionary (password peek).
    #[wasm_bindgen]
    pub fn is_encrypted(data: &[u8]) -> Result<bool, JsError> {
        map_str(super::is_encrypted_native(data))
    }

    /// Decrypt with a user or owner password; returns the unencrypted PDF.
    #[wasm_bindgen]
    pub fn unlock(data: &[u8], password: &str) -> Result<Vec<u8>, JsError> {
        map_str(super::unlock_native(data, password))
    }

    /// Encrypt with AESV2 Standard Security. `opts_json` mirrors
    /// `ProtectOptions` (`userPassword`, `ownerPassword?`, `allowPrinting?`,
    /// `allowCopying?`, `allowModifying?`).
    #[wasm_bindgen]
    pub fn protect(data: &[u8], opts_json: &str) -> Result<Vec<u8>, JsError> {
        map_str(super::protect_native(data, opts_json))
    }

    /// Lay out preprocessed plain text into a paged PDF.
    /// `opts_json` mirrors documentPdf.ts: `{title?, pageSize?: "a4"|"letter", fontSize?, margin?}`.
    #[wasm_bindgen]
    pub fn text_to_pdf(text: &str, opts_json: &str) -> Result<Vec<u8>, JsError> {
        map_str(super::text_to_pdf_native(text, opts_json))
    }

    /// Build a DOCX from extracted PDF text pages (the `UFTP` pack).
    #[wasm_bindgen]
    pub fn text_pages_to_docx(pack: &[u8]) -> Result<Vec<u8>, JsError> {
        map_str(super::text_pages_to_docx_native(pack))
    }

    /// Build an XLSX from extracted PDF text pages (the `UFTP` pack).
    #[wasm_bindgen]
    pub fn text_pages_to_xlsx(pack: &[u8]) -> Result<Vec<u8>, JsError> {
        map_str(super::text_pages_to_xlsx_native(pack))
    }

    /// Build a PPTX from extracted PDF text pages (the `UFTP` pack).
    #[wasm_bindgen]
    pub fn text_pages_to_pptx(pack: &[u8]) -> Result<Vec<u8>, JsError> {
        map_str(super::text_pages_to_pptx_native(pack))
    }

    /// Build a reflowable EPUB from a `UFTP` text-pages pack. `opts_json` mirrors
    /// ReflowableEpubOptions (`{title?, author?, language?, identifier?, modified?,
    /// removeHeadersFooters?, unwrapParagraphs?, repairHyphenation?}`).
    #[wasm_bindgen]
    pub fn text_pages_to_epub(pack: &[u8], opts_json: &str) -> Result<Vec<u8>, JsError> {
        map_str(super::text_pages_to_epub_native(pack, opts_json))
    }

    /// Build a fixed-layout EPUB from a `UFXP` page-image pack. `opts_json` mirrors
    /// FixedLayoutEpubOptions (`{title?, author?, language?, identifier?, modified?}`).
    #[wasm_bindgen]
    pub fn fixed_pages_to_epub(pack: &[u8], opts_json: &str) -> Result<Vec<u8>, JsError> {
        map_str(super::fixed_pages_to_epub_native(pack, opts_json))
    }
}
