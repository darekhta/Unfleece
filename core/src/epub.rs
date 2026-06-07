//! EPUB 3 generation from extracted PDF/Office content.
//!
//! Two flavours, mirroring `src/lib/tools/epub.ts` exactly:
//!
//! * **Reflowable** ([`text_pages_to_epub_native`]) — rebuilds prose from
//!   positioned text items: groups items into lines, detects multi-column
//!   layouts, drops repeated headers/footers, and reconstructs paragraphs.
//! * **Fixed-layout** ([`fixed_pages_to_epub_native`]) — one rendered page image
//!   per spread, with a `rendition:layout pre-paginated` package.
//!
//! Input crosses the WASM boundary as a [`crate::pack`] pack (`UFTP` / `UFXP`);
//! metadata + behaviour toggles arrive as a small JSON options object. Output is
//! a complete, OCF-compliant EPUB ZIP (`mimetype` first and *stored*, everything
//! else DEFLATE).
//!
//! ## WASM safety
//! No `SystemTime`, threads or filesystem. ZIP entries use a fixed 1980-01-01
//! timestamp (deterministic output). Random book identifiers use `getrandom`
//! (the crate's `js` feature backs this in the browser). Because the engine
//! cannot read the wall clock, callers should pass `modified` (an ISO-8601
//! string); when omitted a fixed epoch is substituted.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Write};

use serde::Deserialize;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::pack::PackReader;

/// The OCF media type stored verbatim as the first ZIP entry.
const EPUB_MIME: &str = "application/epub+zip";
/// Title used when none is supplied (matches the TS default).
const DEFAULT_TITLE: &str = "Converted PDF";
/// Fallback `dcterms:modified` when the caller omits a timestamp (WASM has no
/// clock). Callers that want "now" must pass `modified` explicitly.
const DEFAULT_MODIFIED: &str = "1970-01-01T00:00:00Z";

// ===========================================================================
// Options (serde mirrors of the TS option objects)
// ===========================================================================

/// Options for [`text_pages_to_epub_native`] (`ReflowableEpubOptions`).
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ReflowableOpts {
    title: Option<String>,
    author: Option<String>,
    language: Option<String>,
    identifier: Option<String>,
    modified: Option<String>,
    /// Strip repeated headers/footers. Only an explicit `false` disables it.
    remove_headers_footers: Option<bool>,
    /// Reflow wrapped lines into paragraphs. Only an explicit `false` disables.
    unwrap_paragraphs: Option<bool>,
    /// Join `word-\n continuation`. Only an explicit `false` disables it.
    repair_hyphenation: Option<bool>,
}

/// Options for [`fixed_pages_to_epub_native`] (`FixedLayoutEpubOptions`).
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct FixedOpts {
    title: Option<String>,
    author: Option<String>,
    language: Option<String>,
    identifier: Option<String>,
    modified: Option<String>,
}

/// Resolved, escaped-ready metadata.
#[derive(Debug, Clone)]
struct Metadata {
    title: String,
    author: String,
    language: String,
    identifier: String,
    modified: String,
}

// ===========================================================================
// Parsed pack model
// ===========================================================================

#[derive(Debug, Clone)]
struct Item {
    text: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone)]
struct Page {
    page_number: u32,
    /// `None` when unknown (0 / NaN in the pack).
    width: Option<f32>,
    /// `None` when unknown (0 / NaN in the pack).
    height: Option<f32>,
    /// Full-text fallback (always empty when sourced from a `UFTP` pack, which
    /// carries only positioned items; kept for parity with the TS pipeline and
    /// unit-tested directly).
    text: String,
    items: Vec<Item>,
}

#[derive(Debug, Clone)]
struct FixedPage {
    page_number: u32,
    width: f32,
    height: f32,
    kind: u8,
    bytes: Vec<u8>,
}

/// A reconstructed text line with its geometry (`TextLine` in the TS).
#[derive(Debug, Clone)]
struct TextLine {
    text: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    page_width: Option<f32>,
    page_height: Option<f32>,
}

/// One spine document.
struct ContentDoc {
    id: String,
    href: String,
    title: String,
    xml: String,
}

/// One embedded image (fixed-layout only).
struct EpubImage {
    id: String,
    href: String,
    bytes: Vec<u8>,
    media_type: &'static str,
}

// ===========================================================================
// Public entry points
// ===========================================================================

/// Build a reflowable EPUB from a `UFTP` text-pages pack.
///
/// Pack layout (little-endian): magic `UFTP`, `u32 page_count`, then per page
/// `u32 page_number`, `f32 width`, `f32 height` (0/NaN ⇒ unknown), `u32
/// item_count`, then per item `f32 x`, `f32 y`, `f32 width`, `f32 height`,
/// `u32`-length-prefixed UTF-8 `text`.
pub fn text_pages_to_epub_native(pack: &[u8], opts_json: &str) -> Result<Vec<u8>, String> {
    let pages = parse_uftp(pack)?;
    let opts = parse_reflowable_opts(opts_json)?;
    let metadata = metadata_from_options(
        opts.title.as_deref(),
        opts.author.as_deref(),
        opts.language.as_deref(),
        opts.identifier.as_deref(),
        opts.modified.as_deref(),
    );

    let page_lines: Vec<Vec<TextLine>> = pages.iter().map(ordered_lines).collect();
    let cleaned: Vec<Vec<TextLine>> = if opts.remove_headers_footers == Some(false) {
        page_lines
    } else {
        remove_repeated_headers_footers(&page_lines)
    };

    let mut docs: Vec<ContentDoc> = Vec::with_capacity(cleaned.len());
    for (index, lines) in cleaned.iter().enumerate() {
        let page_number = pages
            .get(index)
            .map(|p| p.page_number)
            .unwrap_or((index + 1) as u32);
        let paragraphs = reconstruct_paragraphs(lines, &opts);
        let padded = pad_page(index + 1);
        let title = format!("Page {page_number}");
        let xml = xhtml_page(
            &title,
            &page_body(page_number, &paragraphs),
            "../styles/book.css",
            &metadata.language,
        );
        docs.push(ContentDoc {
            id: format!("page-{padded}"),
            href: format!("text/page-{padded}.xhtml"),
            title,
            xml,
        });
    }

    if docs.is_empty() {
        docs.push(empty_doc(&metadata.language));
    }

    write_epub(
        &metadata,
        &docs,
        &[(
            "css".to_string(),
            "styles/book.css".to_string(),
            reflowable_css(),
        )],
        &[],
        false,
    )
}

/// Build a fixed-layout EPUB from a `UFXP` page-image pack.
///
/// Pack layout: magic `UFXP`, `u32 page_count`, then per page `u32 page_number`,
/// `f32 width`, `f32 height`, `u8 kind` (0 = PNG, anything else = JPEG), and a
/// `u32`-length-prefixed image byte blob.
pub fn fixed_pages_to_epub_native(pack: &[u8], opts_json: &str) -> Result<Vec<u8>, String> {
    let pages = parse_ufxp(pack)?;
    let opts = parse_fixed_opts(opts_json)?;
    let metadata = metadata_from_options(
        opts.title.as_deref(),
        opts.author.as_deref(),
        opts.language.as_deref(),
        opts.identifier.as_deref(),
        opts.modified.as_deref(),
    );

    let mut docs: Vec<ContentDoc> = Vec::with_capacity(pages.len());
    let mut images: Vec<EpubImage> = Vec::with_capacity(pages.len());
    for (index, page) in pages.iter().enumerate() {
        let padded = pad_page(index + 1);
        // kind 0 = PNG; everything else falls back to JPEG (mirrors the TS
        // `imageExtensionAndType`, which defaults to JPEG).
        let (ext, media_type) = if page.kind == 0 {
            ("png", "image/png")
        } else {
            ("jpg", "image/jpeg")
        };
        let image_href = format!("images/page-{padded}.{ext}");
        let title = format!("Page {}", page.page_number);
        docs.push(ContentDoc {
            id: format!("page-{padded}"),
            href: format!("text/page-{padded}.xhtml"),
            xml: fixed_xhtml_page(
                &title,
                &format!("../{image_href}"),
                page.width,
                page.height,
                &metadata.language,
            ),
            title,
        });
        images.push(EpubImage {
            id: format!("image-{padded}"),
            href: image_href,
            bytes: page.bytes.clone(),
            media_type,
        });
    }

    if docs.is_empty() {
        docs.push(empty_doc(&metadata.language));
    }

    write_epub(
        &metadata,
        &docs,
        &[(
            "fixed-css".to_string(),
            "styles/fixed.css".to_string(),
            fixed_css(),
        )],
        &images,
        true,
    )
}

/// The placeholder page emitted when there is no content (parity with TS).
fn empty_doc(language: &str) -> ContentDoc {
    ContentDoc {
        id: "page-001".to_string(),
        href: "text/page-001.xhtml".to_string(),
        title: "Page 1".to_string(),
        xml: xhtml_page("Page 1", &page_body(1, &[]), "../styles/book.css", language),
    }
}

// ===========================================================================
// Pack parsing
// ===========================================================================

/// 0 / non-finite ⇒ "unknown"; otherwise the positive value.
fn opt_pos(v: f32) -> Option<f32> {
    if v.is_finite() && v > 0.0 {
        Some(v)
    } else {
        None
    }
}

fn parse_uftp(pack: &[u8]) -> Result<Vec<Page>, String> {
    let mut r = PackReader::new(pack);
    r.expect_magic(b"UFTP")?;
    let page_count = r.read_u32()? as usize;
    // Note: never pre-allocate from the declared count — a malicious/huge count
    // must fail on the first short read, not OOM up front.
    let mut pages = Vec::new();
    for _ in 0..page_count {
        let page_number = r.read_u32()?;
        let width = r.read_f32()?;
        let height = r.read_f32()?;
        let item_count = r.read_u32()? as usize;
        let mut items = Vec::new();
        for _ in 0..item_count {
            let x = r.read_f32()?;
            let y = r.read_f32()?;
            let w = r.read_f32()?;
            let h = r.read_f32()?;
            let text = r.read_str()?.to_string();
            items.push(Item {
                text,
                x,
                y,
                width: w,
                height: h,
            });
        }
        pages.push(Page {
            page_number,
            width: opt_pos(width),
            height: opt_pos(height),
            text: String::new(),
            items,
        });
    }
    r.expect_done()?;
    Ok(pages)
}

fn parse_ufxp(pack: &[u8]) -> Result<Vec<FixedPage>, String> {
    let mut r = PackReader::new(pack);
    r.expect_magic(b"UFXP")?;
    let page_count = r.read_u32()? as usize;
    let mut pages = Vec::new();
    for _ in 0..page_count {
        let page_number = r.read_u32()?;
        let width = r.read_f32()?;
        let height = r.read_f32()?;
        let kind = r.read_u8()?;
        let bytes = r.read_bytes()?.to_vec();
        pages.push(FixedPage {
            page_number,
            width,
            height,
            kind,
            bytes,
        });
    }
    r.expect_done()?;
    Ok(pages)
}

fn parse_reflowable_opts(s: &str) -> Result<ReflowableOpts, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(ReflowableOpts::default());
    }
    serde_json::from_str(trimmed).map_err(|e| format!("Invalid EPUB options JSON: {e}"))
}

fn parse_fixed_opts(s: &str) -> Result<FixedOpts, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(FixedOpts::default());
    }
    serde_json::from_str(trimmed).map_err(|e| format!("Invalid EPUB options JSON: {e}"))
}

// ===========================================================================
// String hygiene (cleanXmlText / xmlEscape / language / pad / timestamp)
// ===========================================================================

/// Mirrors the set matched by JavaScript's `\s` in a `RegExp`.
fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{00A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// Strip XML-illegal control characters, collapse whitespace runs to a single
/// space, and trim. Equivalent to the TS `cleanXmlText`.
fn clean_xml_text(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut pending_space = false;
    let mut started = false;
    for c in value.chars() {
        let u = c as u32;
        // Control chars are removed *before* whitespace collapsing (this is why
        // U+000B / U+000C are dropped rather than treated as spaces).
        if u <= 0x08 || u == 0x0B || u == 0x0C || (0x0E..=0x1F).contains(&u) {
            continue;
        }
        if is_js_space(c) {
            if started {
                pending_space = true;
            }
            continue;
        }
        if pending_space {
            out.push(' ');
            pending_space = false;
        }
        out.push(c);
        started = true;
    }
    out
}

/// `cleanXmlText` followed by XML entity escaping.
fn xml_escape(value: &str) -> String {
    let cleaned = clean_xml_text(value);
    let mut out = String::with_capacity(cleaned.len());
    for c in cleaned.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Attribute escaping is identical to text escaping in the TS implementation.
fn attribute_escape(value: &str) -> String {
    xml_escape(value)
}

/// UTF-16 code-unit length, matching JavaScript's `String.length`.
fn js_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Zero-pad to (at least) three digits.
fn pad_page(n: usize) -> String {
    format!("{n:03}")
}

/// Validate against RFC 5646's basic shape `[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8})*`.
fn is_valid_language(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    if n == 0 {
        return false;
    }
    let mut i = 0;
    let mut letters = 0;
    while i < n && letters < 3 && chars[i].is_ascii_alphabetic() {
        i += 1;
        letters += 1;
    }
    if letters < 2 {
        return false;
    }
    while i < n {
        if chars[i] != '-' {
            return false;
        }
        i += 1;
        let mut sub = 0;
        while i < n && sub < 8 && chars[i].is_ascii_alphanumeric() {
            i += 1;
            sub += 1;
        }
        if sub < 2 {
            return false;
        }
    }
    i == n
}

fn normalize_language(value: Option<&str>) -> String {
    let candidate = match value {
        Some(v) if !v.is_empty() => v.trim_matches(is_js_space),
        _ => "en",
    };
    if is_valid_language(candidate) {
        candidate.to_string()
    } else {
        "en".to_string()
    }
}

/// Drop a trailing `.NNN` before a `Z` (`new Date().toISOString()` form).
fn strip_millis(s: String) -> String {
    let bytes = s.as_bytes();
    let n = bytes.len();
    if n >= 5
        && bytes[n - 1] == b'Z'
        && bytes[n - 5] == b'.'
        && bytes[n - 4].is_ascii_digit()
        && bytes[n - 3].is_ascii_digit()
        && bytes[n - 2].is_ascii_digit()
    {
        let mut t = String::with_capacity(n - 4);
        t.push_str(&s[..n - 5]);
        t.push('Z');
        return t;
    }
    s
}

/// A v4-shaped UUID URN. Uses `getrandom`; on the (extremely unlikely) failure
/// path it degrades to a zero-filled value rather than erroring.
fn random_uuid_urn() -> String {
    let mut b = [0u8; 16];
    let _ = getrandom::getrandom(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "urn:uuid:{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13],
        b[14], b[15]
    )
}

fn metadata_from_options(
    title: Option<&str>,
    author: Option<&str>,
    language: Option<&str>,
    identifier: Option<&str>,
    modified: Option<&str>,
) -> Metadata {
    let title = {
        let base = match title {
            Some(t) if !t.is_empty() => t,
            _ => DEFAULT_TITLE,
        };
        let cleaned = clean_xml_text(base);
        if cleaned.is_empty() {
            DEFAULT_TITLE.to_string()
        } else {
            cleaned
        }
    };
    let author = clean_xml_text(author.unwrap_or(""));
    let language = normalize_language(language);
    let identifier = match identifier {
        Some(id) if !id.is_empty() => clean_xml_text(id),
        _ => clean_xml_text(&random_uuid_urn()),
    };
    let modified = match modified {
        Some(m) if !m.is_empty() => strip_millis(clean_xml_text(m)),
        _ => DEFAULT_MODIFIED.to_string(),
    };
    Metadata {
        title,
        author,
        language,
        identifier,
        modified,
    }
}

// ===========================================================================
// Numeric helpers + stable sorting
// ===========================================================================

fn median(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() & 1 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

/// Ordering from the sign of a difference, treating `0` and `NaN` as "equal"
/// (matching how JS `||` falls through on a `0`/`NaN` comparator result).
fn sign_ord(v: f32) -> Ordering {
    if v > 0.0 {
        Ordering::Greater
    } else if v < 0.0 {
        Ordering::Less
    } else {
        Ordering::Equal
    }
}

/// `(a, b) => b.y - a.y || a.x - b.x` — descending y, then ascending x.
fn cmp_desc_y_asc_x(a: &TextLine, b: &TextLine) -> Ordering {
    let dy = b.y - a.y;
    if dy != 0.0 && !dy.is_nan() {
        return sign_ord(dy);
    }
    sign_ord(a.x - b.x)
}

/// Indices of `items` in stably-sorted order. A hand-rolled bottom-up merge sort
/// so the result is deterministic and never panics even if the comparator is not
/// a total order (JS `Array.sort` is likewise tolerant + stable).
fn stable_sorted_indices<T>(items: &[T], cmp: impl Fn(&T, &T) -> Ordering) -> Vec<usize> {
    let n = items.len();
    let mut idx: Vec<usize> = (0..n).collect();
    if n < 2 {
        return idx;
    }
    let mut buf = vec![0usize; n];
    let mut width = 1;
    while width < n {
        let mut i = 0;
        while i < n {
            let left = i;
            let mid = (i + width).min(n);
            let right = (i + 2 * width).min(n);
            let (mut a, mut b, mut k) = (left, mid, left);
            while a < mid && b < right {
                if cmp(&items[idx[a]], &items[idx[b]]) != Ordering::Greater {
                    buf[k] = idx[a];
                    a += 1;
                } else {
                    buf[k] = idx[b];
                    b += 1;
                }
                k += 1;
            }
            while a < mid {
                buf[k] = idx[a];
                a += 1;
                k += 1;
            }
            while b < right {
                buf[k] = idx[b];
                b += 1;
                k += 1;
            }
            i += 2 * width;
        }
        idx.copy_from_slice(&buf);
        width *= 2;
    }
    idx
}

fn stable_sort_clone<T: Clone>(items: &[T], cmp: impl Fn(&T, &T) -> Ordering) -> Vec<T> {
    stable_sorted_indices(items, cmp)
        .into_iter()
        .map(|i| items[i].clone())
        .collect()
}

// ===========================================================================
// Line grouping + column-aware ordering
// ===========================================================================

fn trimmed_nonempty(s: &str) -> bool {
    s.chars().any(|c| !is_js_space(c))
}

fn join_line_items(items: &[Item]) -> String {
    let mut out = String::new();
    let mut prev: Option<&Item> = None;
    for item in items {
        let text = item.text.trim_matches(is_js_space);
        if text.is_empty() {
            continue;
        }
        let needs_space = match prev {
            Some(p) => {
                let gap = item.x - (p.x + p.width);
                gap > 1.5f32.max(p.height * 0.18)
            }
            None => false,
        };
        if !out.is_empty() && needs_space {
            out.push(' ');
        }
        out.push_str(text);
        prev = Some(item);
    }
    clean_xml_text(&out)
}

fn fallback_lines_from_text(page: &Page) -> Vec<TextLine> {
    let lines: Vec<String> = page
        .text
        .split('\n')
        .map(clean_xml_text)
        .filter(|s| !s.is_empty())
        .collect();
    let page_height = page.height.unwrap_or(lines.len() as f32 * 14.0);
    lines
        .iter()
        .enumerate()
        .map(|(index, text)| TextLine {
            x: 0.0,
            y: page_height - index as f32 * 14.0,
            width: js_len(text) as f32 * 7.0,
            height: 12.0,
            page_width: page.width,
            page_height: Some(page_height),
            text: text.clone(),
        })
        .collect()
}

fn group_items_into_lines(page: &Page) -> Vec<TextLine> {
    let items: Vec<Item> = page
        .items
        .iter()
        .filter(|it| trimmed_nonempty(&it.text))
        .cloned()
        .collect();
    if items.is_empty() {
        return fallback_lines_from_text(page);
    }

    let heights: Vec<f32> = items
        .iter()
        .map(|i| i.height)
        .filter(|&h| h > 0.0)
        .collect();
    let typical_height = {
        let m = median(&heights);
        if m == 0.0 {
            10.0
        } else {
            m
        }
    };
    let tolerance = 3f32.max(typical_height * 0.65);

    let sorted = stable_sort_clone(&items, |a, b| {
        let v = if (b.y - a.y).abs() > tolerance {
            b.y - a.y
        } else {
            a.x - b.x
        };
        sign_ord(v)
    });

    let mut groups: Vec<Vec<Item>> = Vec::new();
    for item in sorted {
        match groups.last() {
            Some(last) if (last[0].y - item.y).abs() <= tolerance => {
                groups.last_mut().unwrap().push(item);
            }
            _ => groups.push(vec![item]),
        }
    }

    let mut out = Vec::new();
    for group in groups {
        let line_items = stable_sort_clone(&group, |a, b| sign_ord(a.x - b.x));
        let x1 = line_items.iter().map(|i| i.x).fold(f32::INFINITY, f32::min);
        let x2 = line_items
            .iter()
            .map(|i| i.x + i.width)
            .fold(f32::NEG_INFINITY, f32::max);
        let text = join_line_items(&line_items);
        let ys: Vec<f32> = line_items.iter().map(|i| i.y).collect();
        let y = median(&ys);
        let width = (x2 - x1).max(0.0);
        let height = line_items
            .iter()
            .map(|i| {
                if i.height == 0.0 || i.height.is_nan() {
                    typical_height
                } else {
                    i.height
                }
            })
            .fold(f32::NEG_INFINITY, f32::max);
        if !text.is_empty() {
            out.push(TextLine {
                text,
                x: x1,
                y,
                width,
                height,
                page_width: page.width,
                page_height: page.height,
            });
        }
    }
    out
}

fn ordered_lines(page: &Page) -> Vec<TextLine> {
    let lines = group_items_into_lines(page);
    if lines.len() < 8 {
        return stable_sort_clone(&lines, cmp_desc_y_asc_x);
    }

    let max_x = lines
        .iter()
        .map(|l| l.x + l.width)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_x = lines.iter().map(|l| l.x).fold(f32::INFINITY, f32::min);
    let page_width = page.width.unwrap_or((max_x - min_x).max(1.0));

    let candidate_idx: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].width > page_width * 0.08 && lines[i].width < page_width * 0.72)
        .collect();
    if candidate_idx.len() < 8 {
        return stable_sort_clone(&lines, cmp_desc_y_asc_x);
    }

    let mut left_edges: Vec<f32> = candidate_idx.iter().map(|&i| lines[i].x).collect();
    left_edges.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mut largest_gap = 0.0f32;
    let mut gap_index: isize = -1;
    for i in 1..left_edges.len() {
        let gap = left_edges[i] - left_edges[i - 1];
        if gap > largest_gap {
            largest_gap = gap;
            gap_index = i as isize;
        }
    }

    let min_cluster = 3usize.max((candidate_idx.len() as f32 * 0.25).floor() as usize) as isize;
    if gap_index < min_cluster
        || (candidate_idx.len() as isize - gap_index) < min_cluster
        || largest_gap < page_width * 0.16
    {
        return stable_sort_clone(&lines, cmp_desc_y_asc_x);
    }

    let gi = gap_index as usize;
    let split = (left_edges[gi - 1] + left_edges[gi]) / 2.0;

    let mut in_left = vec![false; lines.len()];
    let mut in_right = vec![false; lines.len()];
    for (i, l) in lines.iter().enumerate() {
        if l.x < split && l.x + l.width < split + page_width * 0.12 {
            in_left[i] = true;
        }
        if l.x >= split || l.x + l.width >= split + page_width * 0.55 {
            in_right[i] = true;
        }
    }

    let mut column_ys: Vec<f32> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if in_left[i] {
            column_ys.push(l.y);
        }
    }
    for (i, l) in lines.iter().enumerate() {
        if in_right[i] {
            column_ys.push(l.y);
        }
    }
    let top_col_y = column_ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let bottom_col_y = column_ys.iter().copied().fold(f32::INFINITY, f32::min);

    let spanning_idx: Vec<usize> = (0..lines.len())
        .filter(|&i| !in_left[i] && !in_right[i])
        .collect();
    let collect_sorted = |pred: &dyn Fn(f32) -> bool| -> Vec<TextLine> {
        let sub: Vec<TextLine> = spanning_idx
            .iter()
            .filter(|&&i| pred(lines[i].y))
            .map(|&i| lines[i].clone())
            .collect();
        stable_sort_clone(&sub, cmp_desc_y_asc_x)
    };
    let top_spanning = collect_sorted(&|y| y > top_col_y);
    let middle_spanning = collect_sorted(&|y| y <= top_col_y && y >= bottom_col_y);
    let bottom_spanning = collect_sorted(&|y| y < bottom_col_y);

    let left_lines: Vec<TextLine> = (0..lines.len())
        .filter(|&i| in_left[i])
        .map(|i| lines[i].clone())
        .collect();
    let right_lines: Vec<TextLine> = (0..lines.len())
        .filter(|&i| in_right[i])
        .map(|i| lines[i].clone())
        .collect();

    let mut result = Vec::with_capacity(lines.len());
    result.extend(top_spanning);
    result.extend(stable_sort_clone(&left_lines, cmp_desc_y_asc_x));
    result.extend(stable_sort_clone(&right_lines, cmp_desc_y_asc_x));
    result.extend(middle_spanning);
    result.extend(bottom_spanning);
    result
}

// ===========================================================================
// Header/footer removal
// ===========================================================================

fn normalized_repeated_line_key(text: &str) -> String {
    clean_xml_text(text)
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_digit() { '#' } else { c })
        .collect()
}

/// `^(?:page\s*)?\d+(?:\s*(?:/|of)\s*\d+)?$` (case-insensitive).
fn is_standalone_page_number(text: &str) -> bool {
    let cleaned = clean_xml_text(text);
    if cleaned.is_empty() {
        return false;
    }
    let chars: Vec<char> = cleaned.to_lowercase().chars().collect();
    let n = chars.len();
    let mut i = 0;

    if n >= 4 && chars[..4] == ['p', 'a', 'g', 'e'] {
        i = 4;
        while i < n && chars[i] == ' ' {
            i += 1;
        }
    }

    let digits_start = i;
    while i < n && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        return false;
    }

    if i < n {
        let mut j = i;
        while j < n && chars[j] == ' ' {
            j += 1;
        }
        if j < n && chars[j] == '/' {
            j += 1;
        } else if j + 1 < n && chars[j] == 'o' && chars[j + 1] == 'f' {
            j += 2;
        } else {
            return false;
        }
        while j < n && chars[j] == ' ' {
            j += 1;
        }
        let ds = j;
        while j < n && chars[j].is_ascii_digit() {
            j += 1;
        }
        if j == ds {
            return false;
        }
        i = j;
    }

    i == n
}

fn is_in_header_footer_band(line: &TextLine) -> bool {
    match line.page_height {
        Some(ph) if ph > 0.0 => line.y >= ph * 0.9 || line.y <= ph * 0.1,
        _ => false,
    }
}

fn remove_repeated_headers_footers(pages: &[Vec<TextLine>]) -> Vec<Vec<TextLine>> {
    let mut counts: HashMap<String, HashSet<usize>> = HashMap::new();
    for (page_index, lines) in pages.iter().enumerate() {
        for line in lines.iter().filter(|l| is_in_header_footer_band(l)) {
            let key = normalized_repeated_line_key(&line.text);
            if key.is_empty() {
                continue;
            }
            counts.entry(key).or_default().insert(page_index);
        }
    }

    let total = pages.len().max(1) as f32;
    let mut repeated: HashSet<String> = HashSet::new();
    for (key, set) in &counts {
        let size = set.len();
        if size >= 3 || (size >= 2 && (size as f32) / total >= 0.4) {
            repeated.insert(key.clone());
        }
    }

    pages
        .iter()
        .map(|lines| {
            lines
                .iter()
                .filter(|line| {
                    if is_standalone_page_number(&line.text) {
                        return false;
                    }
                    if !is_in_header_footer_band(line) {
                        return true;
                    }
                    !repeated.contains(&normalized_repeated_line_key(&line.text))
                })
                .cloned()
                .collect()
        })
        .collect()
}

// ===========================================================================
// Paragraph reconstruction
// ===========================================================================

fn is_roman(c: char) -> bool {
    matches!(
        c.to_ascii_lowercase(),
        'i' | 'v' | 'x' | 'l' | 'c' | 'd' | 'm'
    )
}

/// `^(?:[-*•]|\d{1,3}[.)]|[ivxlcdm]{1,8}[.)])\s+` (case-insensitive).
fn starts_with_list_marker(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    if n == 0 {
        return false;
    }
    let after: Option<usize> = if matches!(chars[0], '-' | '*' | '\u{2022}') {
        Some(1)
    } else {
        let mut digits = 0;
        while digits < n && digits < 3 && chars[digits].is_ascii_digit() {
            digits += 1;
        }
        if (1..=3).contains(&digits) && digits < n && matches!(chars[digits], '.' | ')') {
            Some(digits + 1)
        } else {
            let mut romans = 0;
            while romans < n && romans < 8 && is_roman(chars[romans]) {
                romans += 1;
            }
            if (1..=8).contains(&romans) && romans < n && matches!(chars[romans], '.' | ')') {
                Some(romans + 1)
            } else {
                None
            }
        }
    };

    match after {
        Some(pos) => pos < n && is_js_space(chars[pos]),
        None => false,
    }
}

fn starts_az_or_digit(word: &str) -> bool {
    match word.chars().next() {
        Some(c) => c.is_ascii_uppercase() || c.is_ascii_digit(),
        None => false,
    }
}

fn looks_like_heading(text: &str, median_length: f32) -> bool {
    let cleaned = clean_xml_text(text);
    if cleaned.is_empty() {
        return false;
    }
    if js_len(&cleaned) as f32 > 80f32.max(median_length * 1.4) {
        return false;
    }
    if let Some(last) = cleaned.chars().last() {
        if matches!(last, '.' | '!' | '?' | ';' | ':') {
            return false;
        }
    }
    let words: Vec<&str> = cleaned.split(' ').filter(|w| !w.is_empty()).collect();
    if words.is_empty() {
        return false;
    }
    let capitalized = words.iter().filter(|w| starts_az_or_digit(w)).count();
    words.len() <= 9 && (capitalized as f32) / (words.len() as f32) > 0.55
}

fn starts_uppercase(text: &str) -> bool {
    match text.chars().next() {
        Some(c) => c.is_ascii_uppercase() || ('\u{00C0}'..='\u{00DE}').contains(&c),
        None => false,
    }
}

fn ends_terminal(text: &str) -> bool {
    match text.chars().last() {
        Some(c) => matches!(c, '.' | '!' | '?' | ':' | '"' | '\'' | ')' | ']'),
        None => false,
    }
}

fn is_wrap_letter(c: char) -> bool {
    c.is_ascii_alphabetic()
        || ('\u{00C0}'..='\u{00D6}').contains(&c)
        || ('\u{00D8}'..='\u{00F6}').contains(&c)
        || ('\u{00F8}'..='\u{00FF}').contains(&c)
}

fn ends_letter_hyphen(s: &str) -> bool {
    let mut it = s.chars().rev();
    matches!(it.next(), Some('-')) && matches!(it.next(), Some(c) if is_wrap_letter(c))
}

fn starts_lower_letter(s: &str) -> bool {
    match s.chars().next() {
        Some(c) => {
            c.is_ascii_lowercase()
                || ('\u{00E0}'..='\u{00F6}').contains(&c)
                || ('\u{00F8}'..='\u{00FF}').contains(&c)
        }
        None => false,
    }
}

fn append_wrapped_line(paragraph: &str, next: &str, repair_hyphenation: bool) -> String {
    if repair_hyphenation && ends_letter_hyphen(paragraph) && starts_lower_letter(next) {
        let mut joined = paragraph.to_string();
        joined.pop(); // drop the trailing hyphen
        joined.push_str(next);
        joined
    } else {
        format!("{paragraph} {next}")
    }
}

fn reconstruct_paragraphs(lines: &[TextLine], opts: &ReflowableOpts) -> Vec<String> {
    if lines.is_empty() {
        return Vec::new();
    }
    if opts.unwrap_paragraphs == Some(false) {
        return lines
            .iter()
            .map(|l| l.text.clone())
            .filter(|t| !t.is_empty())
            .collect();
    }

    let repair = opts.repair_hyphenation != Some(false);

    let lengths: Vec<f32> = lines
        .iter()
        .map(|l| js_len(&l.text) as f32)
        .filter(|&v| v > 0.0)
        .collect();
    let median_length = {
        let m = median(&lengths);
        if m == 0.0 {
            64.0
        } else {
            m
        }
    };

    let gaps: Vec<f32> = (1..lines.len())
        .map(|i| (lines[i - 1].y - lines[i].y).max(0.0))
        .filter(|&g| g > 0.0)
        .collect();
    let normal_gap = {
        let m = median(&gaps);
        if m == 0.0 {
            let heights: Vec<f32> = lines
                .iter()
                .map(|l| l.height)
                .filter(|&h| h > 0.0)
                .collect();
            12f32.max(median(&heights) * 1.35)
        } else {
            m
        }
    };

    let mut paragraphs: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut previous: Option<&TextLine> = None;

    for line in lines {
        let text = &line.text;
        if text.is_empty() {
            continue;
        }
        if current.is_empty() || previous.is_none() {
            current = text.clone();
            previous = Some(line);
            continue;
        }
        let prev = previous.unwrap();
        let gap = (prev.y - line.y).max(0.0);
        let short_previous = (js_len(&prev.text) as f32) < median_length * 0.72;
        let page_width = line.page_width.unwrap_or(600.0);
        let indented = line.x - prev.x > 12f32.max(page_width * 0.035);
        let heading_boundary = gap > normal_gap * 1.2
            && (looks_like_heading(&prev.text, median_length)
                || looks_like_heading(text, median_length));
        let boundary = gap > normal_gap * 1.65
            || starts_with_list_marker(text)
            || starts_with_list_marker(&prev.text)
            || heading_boundary
            || indented
            || (short_previous && ends_terminal(&prev.text) && starts_uppercase(text));

        if boundary {
            paragraphs.push(current.clone());
            current = text.clone();
        } else {
            current = append_wrapped_line(&current, text, repair);
        }
        previous = Some(line);
    }

    if !current.is_empty() {
        paragraphs.push(current);
    }

    paragraphs
        .iter()
        .map(|p| clean_xml_text(p))
        .filter(|p| !p.is_empty())
        .collect()
}

// ===========================================================================
// XML / CSS templates
// ===========================================================================

fn reflowable_css() -> String {
    r#"html {
  color-scheme: light;
}
body {
  margin: 5%;
  font-family: serif;
  line-height: 1.55;
  color: #111827;
}
.page-heading {
  margin: 0 0 1rem;
  font: 700 1rem/1.3 sans-serif;
  color: #4b5563;
}
p {
  margin: 0 0 0.85rem;
}
.empty-page {
  color: #6b7280;
  font-style: italic;
}
"#
    .to_string()
}

fn fixed_css() -> String {
    r#"html, body {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
}
body {
  background: #fff;
}
img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
}
"#
    .to_string()
}

fn xhtml_page(title: &str, body: &str, css_href: &str, language: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="{lang}" lang="{lang}">
  <head>
    <title>{title}</title>
    <link rel="stylesheet" type="text/css" href="{css}"/>
  </head>
  <body>
{body}
  </body>
</html>"#,
        lang = attribute_escape(language),
        title = xml_escape(title),
        css = attribute_escape(css_href),
        body = body,
    )
}

fn fixed_xhtml_page(
    title: &str,
    image_href: &str,
    width: f32,
    height: f32,
    language: &str,
) -> String {
    let safe_width = (width.round() as i64).max(1);
    let safe_height = (height.round() as i64).max(1);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="{lang}" lang="{lang}">
  <head>
    <title>{title}</title>
    <meta name="viewport" content="width={w}, height={h}"/>
    <link rel="stylesheet" type="text/css" href="../styles/fixed.css"/>
  </head>
  <body>
    <img src="{src}" alt="{alt}"/>
  </body>
</html>"#,
        lang = attribute_escape(language),
        title = xml_escape(title),
        w = safe_width,
        h = safe_height,
        src = attribute_escape(image_href),
        alt = attribute_escape(title),
    )
}

fn page_body(page_number: u32, paragraphs: &[String]) -> String {
    let content = if paragraphs.is_empty() {
        "    <p class=\"empty-page\">No selectable text found on this page.</p>".to_string()
    } else {
        paragraphs
            .iter()
            .map(|p| format!("    <p>{}</p>", xml_escape(p)))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "    <section epub:type=\"chapter\" aria-label=\"Page {pn}\">\n      <h1 class=\"page-heading\">Page {pn}</h1>\n{content}\n    </section>",
        pn = page_number,
        content = content,
    )
}

fn nav_document(metadata: &Metadata, docs: &[ContentDoc]) -> String {
    let toc_items = docs
        .iter()
        .map(|d| {
            format!(
                "      <li><a href=\"{}\">{}</a></li>",
                attribute_escape(&d.href),
                xml_escape(&d.title)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let start_href = docs
        .first()
        .map(|d| d.href.clone())
        .unwrap_or_else(|| "nav.xhtml".to_string());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="{lang}" lang="{lang}">
  <head>
    <title>{title} navigation</title>
  </head>
  <body>
    <nav epub:type="toc" id="toc">
      <h1>{title}</h1>
      <ol>
{toc}
      </ol>
    </nav>
    <nav epub:type="landmarks" id="landmarks" hidden="">
      <h2>Landmarks</h2>
      <ol>
        <li><a epub:type="bodymatter" href="{start}">Start</a></li>
      </ol>
    </nav>
  </body>
</html>"#,
        lang = attribute_escape(&metadata.language),
        title = xml_escape(&metadata.title),
        toc = toc_items,
        start = attribute_escape(&start_href),
    )
}

fn container_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="EPUB/package.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#
        .to_string()
}

fn package_document(
    metadata: &Metadata,
    docs: &[ContentDoc],
    css: &[(String, String)],
    images: &[EpubImage],
    fixed_layout: bool,
) -> String {
    let creator = if metadata.author.is_empty() {
        String::new()
    } else {
        format!(
            "    <dc:creator>{}</dc:creator>\n",
            xml_escape(&metadata.author)
        )
    };
    let fixed_meta = if fixed_layout {
        "    <meta property=\"rendition:layout\">pre-paginated</meta>\n    <meta property=\"rendition:spread\">none</meta>\n    <meta property=\"rendition:orientation\">auto</meta>\n".to_string()
    } else {
        String::new()
    };
    let css_items = css
        .iter()
        .map(|(id, href)| {
            format!(
                "    <item id=\"{}\" href=\"{}\" media-type=\"text/css\"/>",
                attribute_escape(id),
                attribute_escape(href)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let content_items = docs
        .iter()
        .map(|d| {
            format!(
                "    <item id=\"{}\" href=\"{}\" media-type=\"application/xhtml+xml\"/>",
                attribute_escape(&d.id),
                attribute_escape(&d.href)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let image_items = images
        .iter()
        .map(|im| {
            format!(
                "    <item id=\"{}\" href=\"{}\" media-type=\"{}\"/>",
                attribute_escape(&im.id),
                attribute_escape(&im.href),
                attribute_escape(im.media_type)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let spine = docs
        .iter()
        .map(|d| format!("    <itemref idref=\"{}\"/>", attribute_escape(&d.id)))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" unique-identifier="book-id" xmlns="http://www.idpf.org/2007/opf" prefix="rendition: http://www.idpf.org/vocab/rendition/#">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="book-id">{identifier}</dc:identifier>
    <dc:title>{title}</dc:title>
    <dc:language>{language}</dc:language>
{creator}    <meta property="dcterms:modified">{modified}</meta>
{fixed_meta}  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
{css_items}
{content_items}
{image_items}
  </manifest>
  <spine>
{spine}
  </spine>
</package>"##,
        identifier = xml_escape(&metadata.identifier),
        title = xml_escape(&metadata.title),
        language = xml_escape(&metadata.language),
        creator = creator,
        modified = xml_escape(&metadata.modified),
        fixed_meta = fixed_meta,
        css_items = css_items,
        content_items = content_items,
        image_items = image_items,
        spine = spine,
    )
}

// ===========================================================================
// ZIP assembly
// ===========================================================================

/// `mimetype` options: STORED, fixed 1980 timestamp (no `SystemTime`).
fn stored_opts() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default())
}

/// Everything-else options: DEFLATE, fixed 1980 timestamp.
fn deflated_opts() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default())
}

fn add_entry(
    zip: &mut ZipWriter<Cursor<Vec<u8>>>,
    name: &str,
    data: &[u8],
    opts: SimpleFileOptions,
) -> Result<(), String> {
    zip.start_file(name, opts).map_err(|e| e.to_string())?;
    zip.write_all(data).map_err(|e| e.to_string())?;
    Ok(())
}

fn write_epub(
    metadata: &Metadata,
    docs: &[ContentDoc],
    css_files: &[(String, String, String)],
    images: &[EpubImage],
    fixed_layout: bool,
) -> Result<Vec<u8>, String> {
    let css_meta: Vec<(String, String)> = css_files
        .iter()
        .map(|(id, href, _)| (id.clone(), href.clone()))
        .collect();

    let mut zip = ZipWriter::new(Cursor::new(Vec::<u8>::new()));

    // OCF: the mimetype entry MUST be first and stored uncompressed.
    add_entry(&mut zip, "mimetype", EPUB_MIME.as_bytes(), stored_opts())?;

    let deflated = deflated_opts();
    add_entry(
        &mut zip,
        "META-INF/container.xml",
        container_xml().as_bytes(),
        deflated,
    )?;
    add_entry(
        &mut zip,
        "EPUB/package.opf",
        package_document(metadata, docs, &css_meta, images, fixed_layout).as_bytes(),
        deflated,
    )?;
    add_entry(
        &mut zip,
        "EPUB/nav.xhtml",
        nav_document(metadata, docs).as_bytes(),
        deflated,
    )?;
    for (_, href, content) in css_files {
        add_entry(
            &mut zip,
            &format!("EPUB/{href}"),
            content.as_bytes(),
            deflated,
        )?;
    }
    for doc in docs {
        add_entry(
            &mut zip,
            &format!("EPUB/{}", doc.href),
            doc.xml.as_bytes(),
            deflated,
        )?;
    }
    for image in images {
        add_entry(
            &mut zip,
            &format!("EPUB/{}", image.href),
            &image.bytes,
            deflated,
        )?;
    }

    let cursor = zip.finish().map_err(|e| e.to_string())?;
    Ok(cursor.into_inner())
}

// ===========================================================================
// Tests
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackWriter;
    use std::io::Read;
    use zip::ZipArchive;

    // --- pack builders -----------------------------------------------------

    type ItemSpec<'a> = (f32, f32, f32, f32, &'a str);
    type PageSpec<'a> = (u32, f32, f32, Vec<ItemSpec<'a>>);

    fn uftp(pages: &[PageSpec]) -> Vec<u8> {
        let mut w = PackWriter::new(b"UFTP").u32(pages.len() as u32);
        for (pn, pw, ph, items) in pages {
            w = w.u32(*pn).f32(*pw).f32(*ph).u32(items.len() as u32);
            for (x, y, iw, ih, t) in items {
                w = w.f32(*x).f32(*y).f32(*iw).f32(*ih).str(t);
            }
        }
        w.finish()
    }

    fn ufxp(pages: &[(u32, f32, f32, u8, Vec<u8>)]) -> Vec<u8> {
        let mut w = PackWriter::new(b"UFXP").u32(pages.len() as u32);
        for (pn, pw, ph, kind, img) in pages {
            w = w.u32(*pn).f32(*pw).f32(*ph).u8(*kind).bytes(img);
        }
        w.finish()
    }

    // --- zip readers -------------------------------------------------------

    fn open(epub: &[u8]) -> ZipArchive<Cursor<Vec<u8>>> {
        ZipArchive::new(Cursor::new(epub.to_vec())).expect("output is a valid zip")
    }

    fn entry(epub: &[u8], name: &str) -> String {
        let mut ar = open(epub);
        let mut f = ar
            .by_name(name)
            .unwrap_or_else(|_| panic!("missing entry {name}"));
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        s
    }

    fn entry_bytes(epub: &[u8], name: &str) -> Vec<u8> {
        let mut ar = open(epub);
        let mut f = ar.by_name(name).unwrap();
        let mut v = Vec::new();
        f.read_to_end(&mut v).unwrap();
        v
    }

    fn names(epub: &[u8]) -> Vec<String> {
        open(epub).file_names().map(|s| s.to_string()).collect()
    }

    fn has(epub: &[u8], name: &str) -> bool {
        names(epub).iter().any(|n| n == name)
    }

    const TINY_JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xD9];

    // --- basic structure ---------------------------------------------------

    #[test]
    fn test_empty_pack_minimal_doc() {
        let epub = text_pages_to_epub_native(&uftp(&[]), "{}").unwrap();
        assert!(has(&epub, "EPUB/text/page-001.xhtml"));
        let opf = entry(&epub, "EPUB/package.opf");
        assert!(opf.contains("<itemref idref=\"page-001\"/>"));
    }

    #[test]
    fn test_single_page_basic() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![(50.0, 700.0, 100.0, 12.0, "Hello world")],
            )]),
            "{}",
        )
        .unwrap();
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        assert!(page.contains("<p>Hello world</p>"));
        assert!(page.contains("<h1 class=\"page-heading\">Page 1</h1>"));
    }

    #[test]
    fn test_multiple_pages() {
        let epub = text_pages_to_epub_native(
            &uftp(&[
                (1, 600.0, 800.0, vec![(50.0, 700.0, 100.0, 12.0, "Alpha")]),
                (2, 600.0, 800.0, vec![(50.0, 700.0, 100.0, 12.0, "Beta")]),
                (3, 600.0, 800.0, vec![(50.0, 700.0, 100.0, 12.0, "Gamma")]),
            ]),
            "{}",
        )
        .unwrap();
        assert!(has(&epub, "EPUB/text/page-001.xhtml"));
        assert!(has(&epub, "EPUB/text/page-002.xhtml"));
        assert!(has(&epub, "EPUB/text/page-003.xhtml"));
        assert!(entry(&epub, "EPUB/text/page-002.xhtml").contains("Beta"));
    }

    #[test]
    fn test_page_numbers_preserved() {
        // Pack page numbers are non-sequential; the heading must echo them.
        let epub = text_pages_to_epub_native(
            &uftp(&[
                (7, 600.0, 800.0, vec![(50.0, 700.0, 100.0, 12.0, "Seven")]),
                (
                    42,
                    600.0,
                    800.0,
                    vec![(50.0, 700.0, 100.0, 12.0, "Forty two")],
                ),
            ]),
            "{}",
        )
        .unwrap();
        assert!(entry(&epub, "EPUB/text/page-001.xhtml").contains("Page 7"));
        assert!(entry(&epub, "EPUB/text/page-002.xhtml").contains("Page 42"));
    }

    #[test]
    fn test_page_number_padding() {
        let pages: Vec<PageSpec> = (1..=12)
            .map(|i| (i, 600.0, 800.0, vec![(50.0, 700.0, 100.0, 12.0, "x")]))
            .collect();
        let epub = text_pages_to_epub_native(&uftp(&pages), "{}").unwrap();
        assert!(has(&epub, "EPUB/text/page-001.xhtml"));
        assert!(has(&epub, "EPUB/text/page-012.xhtml"));
    }

    // --- metadata ----------------------------------------------------------

    #[test]
    fn test_metadata_defaults() {
        let epub = text_pages_to_epub_native(&uftp(&[]), "{}").unwrap();
        let opf = entry(&epub, "EPUB/package.opf");
        assert!(opf.contains("<dc:title>Converted PDF</dc:title>"));
        assert!(opf.contains("<dc:language>en</dc:language>"));
        assert!(opf.contains(&format!(
            "<meta property=\"dcterms:modified\">{DEFAULT_MODIFIED}</meta>"
        )));
        // No author => no creator element.
        assert!(!opf.contains("<dc:creator>"));
        // Random identifier should look like a uuid urn.
        assert!(opf.contains("urn:uuid:"));
    }

    #[test]
    fn test_metadata_custom() {
        let opts = r#"{"title":"My Book","author":"Jane Doe","identifier":"urn:isbn:123","modified":"2020-01-02T03:04:05Z","language":"fr"}"#;
        let epub = text_pages_to_epub_native(&uftp(&[]), opts).unwrap();
        let opf = entry(&epub, "EPUB/package.opf");
        assert!(opf.contains("<dc:title>My Book</dc:title>"));
        assert!(opf.contains("<dc:creator>Jane Doe</dc:creator>"));
        assert!(opf.contains("<dc:identifier id=\"book-id\">urn:isbn:123</dc:identifier>"));
        assert!(opf.contains("<dc:language>fr</dc:language>"));
        assert!(opf.contains("<meta property=\"dcterms:modified\">2020-01-02T03:04:05Z</meta>"));
    }

    #[test]
    fn test_metadata_modified_strips_millis() {
        let opts = r#"{"modified":"2021-06-07T12:00:00.123Z"}"#;
        let epub = text_pages_to_epub_native(&uftp(&[]), opts).unwrap();
        assert!(entry(&epub, "EPUB/package.opf")
            .contains("<meta property=\"dcterms:modified\">2021-06-07T12:00:00Z</meta>"));
    }

    #[test]
    fn test_random_identifier_is_uuid_v4() {
        let id = random_uuid_urn();
        assert!(id.starts_with("urn:uuid:"));
        let hex = &id["urn:uuid:".len()..];
        let parts: Vec<&str> = hex.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert!(parts[2].starts_with('4'), "version nibble should be 4");
        assert!(matches!(
            parts[3].chars().next().unwrap(),
            '8' | '9' | 'a' | 'b'
        ));
    }

    #[test]
    fn test_language_validation() {
        for (input, expected) in [
            ("en", "en"),
            ("en-US", "en-US"),
            ("zh-Hans-CN", "zh-Hans-CN"),
            ("eng", "eng"),
            ("english", "en"),
            ("e", "en"),
            ("123", "en"),
            ("", "en"),
            ("  ", "en"),
        ] {
            assert_eq!(normalize_language(Some(input)), expected, "input={input:?}");
        }
        assert_eq!(normalize_language(None), "en");
    }

    #[test]
    fn test_language_attribute_propagated() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(1, 600.0, 800.0, vec![(50.0, 700.0, 100.0, 12.0, "Bonjour")])]),
            r#"{"language":"fr-CA"}"#,
        )
        .unwrap();
        assert!(entry(&epub, "EPUB/text/page-001.xhtml").contains("xml:lang=\"fr-CA\""));
        assert!(entry(&epub, "EPUB/nav.xhtml").contains("lang=\"fr-CA\""));
        assert!(entry(&epub, "EPUB/package.opf").contains("<dc:language>fr-CA</dc:language>"));
    }

    // --- line grouping & ordering -----------------------------------------

    #[test]
    fn test_items_grouped_into_lines() {
        // Two items on the same y join into a single line (with a space).
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![
                    (50.0, 700.0, 40.0, 12.0, "Hello"),
                    (120.0, 700.0, 40.0, 12.0, "World"),
                ],
            )]),
            "{}",
        )
        .unwrap();
        assert!(entry(&epub, "EPUB/text/page-001.xhtml").contains("<p>Hello World</p>"));
    }

    #[test]
    fn test_single_column_top_to_bottom_order() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![
                    (50.0, 700.0, 100.0, 12.0, "First."),
                    (50.0, 600.0, 100.0, 12.0, "Second."),
                    (50.0, 500.0, 100.0, 12.0, "Third."),
                ],
            )]),
            r#"{"unwrapParagraphs":false}"#,
        )
        .unwrap();
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        let first = page.find("First.").unwrap();
        let third = page.find("Third.").unwrap();
        assert!(first < third);
    }

    #[test]
    fn test_multi_column_detection() {
        let mut items: Vec<ItemSpec> = Vec::new();
        let left_y = [700.0, 650.0, 600.0, 550.0];
        let right_y = [500.0, 450.0, 400.0, 350.0];
        let left_text = ["Left1", "Left2", "Left3", "Left4"];
        let right_text = ["Right1", "Right2", "Right3", "Right4"];
        for i in 0..4 {
            items.push((50.0, left_y[i], 200.0, 12.0, left_text[i]));
            items.push((350.0, right_y[i], 200.0, 12.0, right_text[i]));
        }
        let epub = text_pages_to_epub_native(&uftp(&[(1, 600.0, 800.0, items)]), "{}").unwrap();
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        let l = page.find("Left1").unwrap();
        let r = page.find("Right1").unwrap();
        assert!(l < r, "left column must precede right column");
        assert!(page.contains("Left1 Left2 Left3 Left4"));
        assert!(page.contains("Right1 Right2 Right3 Right4"));
    }

    // --- header/footer removal --------------------------------------------

    fn header_pages(count: usize, header_on: usize) -> Vec<PageSpec<'static>> {
        (0..count)
            .map(|i| {
                let mut items: Vec<ItemSpec> =
                    vec![(50.0, 400.0, 200.0, 12.0, "Body content line")];
                if i < header_on {
                    items.insert(0, (50.0, 780.0, 200.0, 12.0, "Confidential Header"));
                }
                ((i + 1) as u32, 600.0, 800.0, items)
            })
            .collect()
    }

    #[test]
    fn test_headers_footers_removed_repeated() {
        let epub = text_pages_to_epub_native(&uftp(&header_pages(3, 3)), "{}").unwrap();
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        assert!(!page.contains("Confidential Header"));
        assert!(page.contains("Body content line"));
    }

    #[test]
    fn test_headers_footers_removed_threshold() {
        // 2 of 5 pages = 40% => removed.
        let epub = text_pages_to_epub_native(&uftp(&header_pages(5, 2)), "{}").unwrap();
        assert!(!entry(&epub, "EPUB/text/page-001.xhtml").contains("Confidential Header"));
    }

    #[test]
    fn test_headers_footers_kept_singleton() {
        // 1 of 3 pages => kept.
        let epub = text_pages_to_epub_native(&uftp(&header_pages(3, 1)), "{}").unwrap();
        assert!(entry(&epub, "EPUB/text/page-001.xhtml").contains("Confidential Header"));
    }

    #[test]
    fn test_headers_footers_disabled() {
        let epub = text_pages_to_epub_native(
            &uftp(&header_pages(3, 3)),
            r#"{"removeHeadersFooters":false}"#,
        )
        .unwrap();
        // Repeated header survives, and standalone page numbers are not stripped.
        assert!(entry(&epub, "EPUB/text/page-001.xhtml").contains("Confidential Header"));
    }

    #[test]
    fn test_page_number_exclusion() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![
                    (50.0, 400.0, 20.0, 12.0, "5"),
                    (50.0, 300.0, 200.0, 12.0, "Real content here"),
                ],
            )]),
            "{}",
        )
        .unwrap();
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        assert!(!page.contains("<p>5</p>"));
        assert!(page.contains("Real content here"));
    }

    #[test]
    fn test_is_standalone_page_number_cases() {
        for yes in [
            "5",
            "Page 5",
            "page5",
            "5 / 10",
            "5 of 10",
            "Page 5 of 10",
            "12/34",
        ] {
            assert!(is_standalone_page_number(yes), "should match: {yes:?}");
        }
        for no in ["Chapter 5", "5 apples", "Section", "5a", "of 5", ""] {
            assert!(!is_standalone_page_number(no), "should not match: {no:?}");
        }
    }

    // --- paragraph reconstruction -----------------------------------------

    #[test]
    fn test_paragraph_unwrapping() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![
                    (50.0, 700.0, 120.0, 12.0, "the quick brown"),
                    (50.0, 688.0, 120.0, 12.0, "fox jumps over"),
                    (50.0, 676.0, 120.0, 12.0, "the lazy dog"),
                ],
            )]),
            "{}",
        )
        .unwrap();
        assert!(entry(&epub, "EPUB/text/page-001.xhtml")
            .contains("<p>the quick brown fox jumps over the lazy dog</p>"));
    }

    #[test]
    fn test_paragraph_boundaries_on_gap() {
        // A large vertical gap forces a paragraph break.
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![
                    (50.0, 700.0, 120.0, 12.0, "first paragraph text"),
                    (50.0, 688.0, 120.0, 12.0, "still first paragraph"),
                    (50.0, 500.0, 120.0, 12.0, "second paragraph begins"),
                ],
            )]),
            "{}",
        )
        .unwrap();
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        assert!(page.contains("<p>first paragraph text still first paragraph</p>"));
        assert!(page.contains("<p>second paragraph begins</p>"));
    }

    #[test]
    fn test_paragraph_boundaries_on_list_marker() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![
                    (50.0, 700.0, 120.0, 12.0, "intro line one"),
                    (50.0, 688.0, 120.0, 12.0, "- bullet item"),
                ],
            )]),
            "{}",
        )
        .unwrap();
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        assert!(page.contains("<p>intro line one</p>"));
        assert!(page.contains("<p>- bullet item</p>"));
    }

    #[test]
    fn test_paragraph_boundaries_on_heading() {
        // A short capitalized heading after a gap starts a new paragraph.
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![
                    (
                        50.0,
                        700.0,
                        300.0,
                        12.0,
                        "some ordinary running prose here that is long",
                    ),
                    (
                        50.0,
                        688.0,
                        300.0,
                        12.0,
                        "more ordinary running prose that continues",
                    ),
                    (50.0, 640.0, 120.0, 16.0, "New Section Heading"),
                    (
                        50.0,
                        628.0,
                        300.0,
                        12.0,
                        "body text under the heading continues on",
                    ),
                ],
            )]),
            "{}",
        )
        .unwrap();
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        assert!(page.contains("New Section Heading"));
        // The heading must not be glued onto the preceding prose paragraph.
        assert!(!page.contains("continues New Section Heading"));
    }

    #[test]
    fn test_paragraph_boundaries_on_indent() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![
                    (50.0, 700.0, 200.0, 12.0, "left aligned line one"),
                    (120.0, 688.0, 200.0, 12.0, "indented new paragraph"),
                ],
            )]),
            "{}",
        )
        .unwrap();
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        assert!(page.contains("<p>left aligned line one</p>"));
        assert!(page.contains("<p>indented new paragraph</p>"));
    }

    #[test]
    fn test_hyphenation_repair_enabled() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![
                    (50.0, 700.0, 120.0, 12.0, "exam-"),
                    (50.0, 688.0, 120.0, 12.0, "ple continues here"),
                ],
            )]),
            "{}",
        )
        .unwrap();
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        assert!(page.contains("example continues here"));
        assert!(!page.contains("exam- ple"));
    }

    #[test]
    fn test_hyphenation_repair_disabled() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![
                    (50.0, 700.0, 120.0, 12.0, "exam-"),
                    (50.0, 688.0, 120.0, 12.0, "ple continues here"),
                ],
            )]),
            r#"{"repairHyphenation":false}"#,
        )
        .unwrap();
        assert!(entry(&epub, "EPUB/text/page-001.xhtml").contains("exam- ple continues here"));
    }

    #[test]
    fn test_unwrap_paragraphs_false() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![
                    (50.0, 700.0, 120.0, 12.0, "line one"),
                    (50.0, 688.0, 120.0, 12.0, "line two"),
                    (50.0, 676.0, 120.0, 12.0, "line three"),
                ],
            )]),
            r#"{"unwrapParagraphs":false}"#,
        )
        .unwrap();
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        assert!(page.contains("<p>line one</p>"));
        assert!(page.contains("<p>line two</p>"));
        assert!(page.contains("<p>line three</p>"));
        assert_eq!(page.matches("<p>").count(), 3);
    }

    // --- escaping & hygiene -----------------------------------------------

    #[test]
    fn test_xml_escaping() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![(50.0, 700.0, 200.0, 12.0, "a<b>&\"'c")],
            )]),
            r#"{"title":"T<>&\"'"}"#,
        )
        .unwrap();
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        assert!(page.contains("a&lt;b&gt;&amp;&quot;&apos;c"));
        let opf = entry(&epub, "EPUB/package.opf");
        assert!(opf.contains("<dc:title>T&lt;&gt;&amp;&quot;&apos;</dc:title>"));
        assert!(!page.contains("a<b>"));
    }

    #[test]
    fn test_xml_control_chars_stripped() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![(50.0, 700.0, 200.0, 12.0, "a\u{0001}b\u{0007}c")],
            )]),
            "{}",
        )
        .unwrap();
        assert!(entry(&epub, "EPUB/text/page-001.xhtml").contains("<p>abc</p>"));
    }

    #[test]
    fn test_whitespace_normalization() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(
                1,
                600.0,
                800.0,
                vec![(50.0, 700.0, 200.0, 12.0, "a    b\t\tc")],
            )]),
            "{}",
        )
        .unwrap();
        assert!(entry(&epub, "EPUB/text/page-001.xhtml").contains("<p>a b c</p>"));
    }

    #[test]
    fn test_clean_xml_text_idempotent() {
        for s in ["  a  b  ", "x\u{0001}\ty\nz", "café\u{00A0}au lait", ""] {
            let once = clean_xml_text(s);
            assert_eq!(clean_xml_text(&once), once, "not idempotent for {s:?}");
        }
    }

    // --- container/opf/zip structure --------------------------------------

    #[test]
    fn test_epub_mimetype_first_stored() {
        let epub = text_pages_to_epub_native(&uftp(&[]), "{}").unwrap();
        // Local-header inspection: signature, method=stored, no extra field.
        assert_eq!(&epub[0..4], b"PK\x03\x04");
        let flags = u16::from_le_bytes([epub[6], epub[7]]);
        assert_eq!(flags & 0x08, 0, "no streaming data descriptor on mimetype");
        let method = u16::from_le_bytes([epub[8], epub[9]]);
        assert_eq!(method, 0, "mimetype must be stored");
        let name_len = u16::from_le_bytes([epub[26], epub[27]]) as usize;
        let extra_len = u16::from_le_bytes([epub[28], epub[29]]) as usize;
        assert_eq!(&epub[30..30 + name_len], b"mimetype");
        assert_eq!(extra_len, 0, "OCF requires mimetype without extra field");
        // And via the archive API the first entry agrees.
        let mut ar = open(&epub);
        let f0 = ar.by_index(0).unwrap();
        assert_eq!(f0.name(), "mimetype");
        assert_eq!(f0.compression(), CompressionMethod::Stored);
    }

    #[test]
    fn test_epub_mimetype_content() {
        let epub = text_pages_to_epub_native(&uftp(&[]), "{}").unwrap();
        assert_eq!(entry(&epub, "mimetype"), EPUB_MIME);
    }

    #[test]
    fn test_epub_structure_well_formed() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(1, 600.0, 800.0, vec![(50.0, 700.0, 100.0, 12.0, "hi")])]),
            "{}",
        )
        .unwrap();
        for required in [
            "mimetype",
            "META-INF/container.xml",
            "EPUB/package.opf",
            "EPUB/nav.xhtml",
            "EPUB/styles/book.css",
            "EPUB/text/page-001.xhtml",
        ] {
            assert!(has(&epub, required), "missing {required}");
        }
        let container = entry(&epub, "META-INF/container.xml");
        assert!(container.contains("full-path=\"EPUB/package.opf\""));
    }

    #[test]
    fn test_epub_css_included() {
        let epub = text_pages_to_epub_native(&uftp(&[]), "{}").unwrap();
        let css = entry(&epub, "EPUB/styles/book.css");
        assert!(css.contains("font-family: serif"));
        assert!(entry(&epub, "EPUB/package.opf")
            .contains("href=\"styles/book.css\" media-type=\"text/css\""));
    }

    #[test]
    fn test_opf_manifest_spine() {
        let epub = text_pages_to_epub_native(
            &uftp(&[
                (1, 600.0, 800.0, vec![(50.0, 700.0, 100.0, 12.0, "a")]),
                (2, 600.0, 800.0, vec![(50.0, 700.0, 100.0, 12.0, "b")]),
            ]),
            "{}",
        )
        .unwrap();
        let opf = entry(&epub, "EPUB/package.opf");
        assert!(opf.contains("<item id=\"nav\" href=\"nav.xhtml\""));
        assert!(opf.contains(
            "<item id=\"page-001\" href=\"text/page-001.xhtml\" media-type=\"application/xhtml+xml\"/>"
        ));
        assert!(opf.contains("<itemref idref=\"page-001\"/>"));
        assert!(opf.contains("<itemref idref=\"page-002\"/>"));
        assert!(!opf.contains("rendition:layout"));
    }

    #[test]
    fn test_nav_document_start_href() {
        let epub = text_pages_to_epub_native(
            &uftp(&[(1, 600.0, 800.0, vec![(50.0, 700.0, 100.0, 12.0, "hi")])]),
            "{}",
        )
        .unwrap();
        let nav = entry(&epub, "EPUB/nav.xhtml");
        assert!(nav.contains("epub:type=\"bodymatter\" href=\"text/page-001.xhtml\""));
        assert!(nav.contains("<a href=\"text/page-001.xhtml\">Page 1</a>"));
    }

    // --- fallback / empty paths -------------------------------------------

    #[test]
    fn test_empty_page_text() {
        // A page with zero items and no text fallback => empty-page message.
        let epub = text_pages_to_epub_native(&uftp(&[(1, 600.0, 800.0, vec![])]), "{}").unwrap();
        assert!(entry(&epub, "EPUB/text/page-001.xhtml")
            .contains("No selectable text found on this page."));
    }

    #[test]
    fn test_fallback_lines_from_text() {
        // Exercises the fallback helper directly (the UFTP pack carries no
        // page-level text, so this path is only reachable programmatically).
        let page = Page {
            page_number: 1,
            width: Some(600.0),
            height: Some(800.0),
            text: "line one\nline two\n\nline three".to_string(),
            items: vec![],
        };
        let lines = fallback_lines_from_text(&page);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].text, "line one");
        assert_eq!(lines[2].text, "line three");
        assert!(lines[0].y > lines[1].y);
    }

    #[test]
    fn test_round_trip_pack() {
        let pack = uftp(&[
            (3, 600.0, 800.0, vec![(10.0, 20.0, 30.0, 40.0, "héllo")]),
            (4, 0.0, 0.0, vec![]),
        ]);
        let pages = parse_uftp(&pack).unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].page_number, 3);
        assert_eq!(pages[0].width, Some(600.0));
        assert_eq!(pages[0].items.len(), 1);
        assert_eq!(pages[0].items[0].text, "héllo");
        assert_eq!(pages[1].width, None); // 0 => unknown
        assert!(pages[1].items.is_empty());
    }

    // --- fixed layout ------------------------------------------------------

    #[test]
    fn test_fixed_single_page_png() {
        let epub = fixed_pages_to_epub_native(
            &ufxp(&[(1, 800.0, 600.0, 0, crate::util::fixtures::PNG_1X1.to_vec())]),
            "{}",
        )
        .unwrap();
        assert!(has(&epub, "EPUB/images/page-001.png"));
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        assert!(page.contains("src=\"../images/page-001.png\""));
        assert_eq!(
            entry_bytes(&epub, "EPUB/images/page-001.png"),
            crate::util::fixtures::PNG_1X1
        );
    }

    #[test]
    fn test_fixed_single_page_jpeg() {
        let epub =
            fixed_pages_to_epub_native(&ufxp(&[(1, 800.0, 600.0, 1, TINY_JPEG.to_vec())]), "{}")
                .unwrap();
        assert!(has(&epub, "EPUB/images/page-001.jpg"));
        assert!(entry(&epub, "EPUB/text/page-001.xhtml").contains("page-001.jpg"));
    }

    #[test]
    fn test_fixed_multiple_pages_mixed() {
        let epub = fixed_pages_to_epub_native(
            &ufxp(&[
                (1, 800.0, 600.0, 0, crate::util::fixtures::PNG_1X1.to_vec()),
                (2, 640.0, 480.0, 1, TINY_JPEG.to_vec()),
            ]),
            "{}",
        )
        .unwrap();
        assert!(has(&epub, "EPUB/images/page-001.png"));
        assert!(has(&epub, "EPUB/images/page-002.jpg"));
        let opf = entry(&epub, "EPUB/package.opf");
        assert!(opf.contains("media-type=\"image/png\""));
        assert!(opf.contains("media-type=\"image/jpeg\""));
    }

    #[test]
    fn test_fixed_metadata_in_opf() {
        let epub = fixed_pages_to_epub_native(
            &ufxp(&[(1, 800.0, 600.0, 0, crate::util::fixtures::PNG_1X1.to_vec())]),
            r#"{"title":"Scan","author":"Bob"}"#,
        )
        .unwrap();
        let opf = entry(&epub, "EPUB/package.opf");
        assert!(opf.contains("<dc:title>Scan</dc:title>"));
        assert!(opf.contains("<dc:creator>Bob</dc:creator>"));
    }

    #[test]
    fn test_fixed_pre_paginated_meta() {
        let epub = fixed_pages_to_epub_native(
            &ufxp(&[(1, 800.0, 600.0, 0, crate::util::fixtures::PNG_1X1.to_vec())]),
            "{}",
        )
        .unwrap();
        let opf = entry(&epub, "EPUB/package.opf");
        assert!(opf.contains("<meta property=\"rendition:layout\">pre-paginated</meta>"));
        assert!(opf.contains("<meta property=\"rendition:spread\">none</meta>"));
        assert!(opf.contains("<meta property=\"rendition:orientation\">auto</meta>"));
    }

    #[test]
    fn test_fixed_viewport_and_dimensions() {
        let epub = fixed_pages_to_epub_native(
            &ufxp(&[(1, 812.4, 600.6, 0, crate::util::fixtures::PNG_1X1.to_vec())]),
            "{}",
        )
        .unwrap();
        let page = entry(&epub, "EPUB/text/page-001.xhtml");
        // Math.round semantics for positive values.
        assert!(page.contains("content=\"width=812, height=601\""));
    }

    #[test]
    fn test_fixed_css_included() {
        let epub = fixed_pages_to_epub_native(
            &ufxp(&[(1, 800.0, 600.0, 0, crate::util::fixtures::PNG_1X1.to_vec())]),
            "{}",
        )
        .unwrap();
        assert!(has(&epub, "EPUB/styles/fixed.css"));
        assert!(entry(&epub, "EPUB/styles/fixed.css").contains("object-fit: contain"));
    }

    #[test]
    fn test_fixed_image_alt_text() {
        let epub = fixed_pages_to_epub_native(
            &ufxp(&[(9, 800.0, 600.0, 0, crate::util::fixtures::PNG_1X1.to_vec())]),
            "{}",
        )
        .unwrap();
        assert!(entry(&epub, "EPUB/text/page-001.xhtml").contains("alt=\"Page 9\""));
    }

    #[test]
    fn test_fixed_opf_image_items() {
        let epub = fixed_pages_to_epub_native(
            &ufxp(&[(1, 800.0, 600.0, 0, crate::util::fixtures::PNG_1X1.to_vec())]),
            "{}",
        )
        .unwrap();
        assert!(entry(&epub, "EPUB/package.opf").contains(
            "<item id=\"image-001\" href=\"images/page-001.png\" media-type=\"image/png\"/>"
        ));
    }

    #[test]
    fn test_fixed_round_trip_pack() {
        let pack = ufxp(&[(2, 100.0, 200.0, 1, vec![9, 8, 7])]);
        let pages = parse_ufxp(&pack).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_number, 2);
        assert_eq!(pages[0].width, 100.0);
        assert_eq!(pages[0].kind, 1);
        assert_eq!(pages[0].bytes, vec![9, 8, 7]);
    }

    #[test]
    fn test_fixed_empty_pack() {
        let epub = fixed_pages_to_epub_native(&ufxp(&[]), "{}").unwrap();
        assert!(has(&epub, "EPUB/text/page-001.xhtml"));
        assert!(has(&epub, "EPUB/styles/fixed.css"));
    }

    #[test]
    fn test_fixed_page_number_padding() {
        let pages: Vec<(u32, f32, f32, u8, Vec<u8>)> = (1..=10)
            .map(|i| (i, 100.0, 100.0, 0, crate::util::fixtures::PNG_1X1.to_vec()))
            .collect();
        let epub = fixed_pages_to_epub_native(&ufxp(&pages), "{}").unwrap();
        assert!(has(&epub, "EPUB/images/page-001.png"));
        assert!(has(&epub, "EPUB/images/page-010.png"));
    }

    #[test]
    fn test_fixed_language_validation() {
        let epub = fixed_pages_to_epub_native(
            &ufxp(&[(1, 800.0, 600.0, 0, crate::util::fixtures::PNG_1X1.to_vec())]),
            r#"{"language":"not a tag!!"}"#,
        )
        .unwrap();
        assert!(entry(&epub, "EPUB/text/page-001.xhtml").contains("xml:lang=\"en\""));
    }

    #[test]
    fn test_ufxp_invalid_kind_defaults_jpeg() {
        let epub =
            fixed_pages_to_epub_native(&ufxp(&[(1, 800.0, 600.0, 7, TINY_JPEG.to_vec())]), "{}")
                .unwrap();
        assert!(has(&epub, "EPUB/images/page-001.jpg"));
    }

    #[test]
    fn test_ufxp_image_bytes_not_validated() {
        // Image bytes are embedded verbatim — no decoding, so garbage is fine.
        let garbage = vec![1u8, 2, 3, 4, 5];
        let epub =
            fixed_pages_to_epub_native(&ufxp(&[(1, 800.0, 600.0, 0, garbage.clone())]), "{}")
                .unwrap();
        assert_eq!(entry_bytes(&epub, "EPUB/images/page-001.png"), garbage);
    }

    // --- error handling ----------------------------------------------------

    #[test]
    fn test_malformed_uftp_wrong_magic() {
        let bad = PackWriter::new(b"XXXX").u32(0).finish();
        assert!(text_pages_to_epub_native(&bad, "{}").is_err());
    }

    #[test]
    fn test_malformed_uftp_truncated_header() {
        // Declares 2 pages but provides none.
        let bad = PackWriter::new(b"UFTP").u32(2).finish();
        assert!(text_pages_to_epub_native(&bad, "{}").is_err());
    }

    #[test]
    fn test_malformed_uftp_truncated_item() {
        // 1 page, 1 item declared, but item bytes missing.
        let bad = PackWriter::new(b"UFTP")
            .u32(1)
            .u32(1)
            .f32(600.0)
            .f32(800.0)
            .u32(1)
            .finish();
        assert!(text_pages_to_epub_native(&bad, "{}").is_err());
    }

    #[test]
    fn test_malformed_uftp_trailing_data() {
        let mut pack = uftp(&[]);
        pack.push(0xAB);
        assert!(text_pages_to_epub_native(&pack, "{}").is_err());
    }

    #[test]
    fn test_malformed_opts_json_syntax() {
        let err = text_pages_to_epub_native(&uftp(&[]), "{not valid json").unwrap_err();
        assert!(err.contains("Invalid EPUB options JSON"));
    }

    #[test]
    fn test_empty_opts_json_is_defaults() {
        // Empty / whitespace options behave like "{}".
        assert!(text_pages_to_epub_native(&uftp(&[]), "").is_ok());
        assert!(text_pages_to_epub_native(&uftp(&[]), "   ").is_ok());
    }

    #[test]
    fn test_malformed_ufxp_wrong_magic() {
        let bad = PackWriter::new(b"UFTP").u32(0).finish();
        assert!(fixed_pages_to_epub_native(&bad, "{}").is_err());
    }

    #[test]
    fn test_ufxp_truncated_image() {
        let bad = PackWriter::new(b"UFXP")
            .u32(1)
            .u32(1)
            .f32(100.0)
            .f32(100.0)
            .u8(0)
            .u32(50) // claims 50 image bytes
            .finish();
        assert!(fixed_pages_to_epub_native(&bad, "{}").is_err());
    }

    // --- scale / stress ----------------------------------------------------

    #[test]
    fn test_large_page_count() {
        let pages: Vec<PageSpec> = (1..=120)
            .map(|i| (i, 600.0, 800.0, vec![(50.0, 700.0, 100.0, 12.0, "content")]))
            .collect();
        let epub = text_pages_to_epub_native(&uftp(&pages), "{}").unwrap();
        assert!(has(&epub, "EPUB/text/page-120.xhtml"));
    }

    #[test]
    fn test_very_long_text_items() {
        let long = "word ".repeat(2000);
        let epub = text_pages_to_epub_native(
            &uftp(&[(1, 600.0, 800.0, vec![(50.0, 700.0, 100.0, 12.0, &long)])]),
            "{}",
        )
        .unwrap();
        assert!(entry(&epub, "EPUB/text/page-001.xhtml").contains("word word"));
    }

    #[test]
    fn test_special_chars_in_metadata() {
        let epub = text_pages_to_epub_native(
            &uftp(&[]),
            r#"{"title":"Café & Co <résumé>","author":"O'Brien"}"#,
        )
        .unwrap();
        let opf = entry(&epub, "EPUB/package.opf");
        assert!(opf.contains("Café &amp; Co &lt;résumé&gt;"));
        assert!(opf.contains("<dc:creator>O&apos;Brien</dc:creator>"));
    }

    // --- proptests ---------------------------------------------------------

    use proptest::prelude::*;

    fn arb_item() -> impl Strategy<Value = (f32, f32, f32, f32, String)> {
        (
            -500.0f32..1500.0,
            -500.0f32..1500.0,
            0.0f32..400.0,
            0.0f32..40.0,
            "[a-zA-Z0-9 .,-]{0,30}",
        )
    }

    #[allow(clippy::type_complexity)]
    fn arb_page() -> impl Strategy<Value = (u32, f32, f32, Vec<(f32, f32, f32, f32, String)>)> {
        (
            1u32..1000,
            prop::option::of(100.0f32..1200.0).prop_map(|o| o.unwrap_or(0.0)),
            prop::option::of(100.0f32..1600.0).prop_map(|o| o.unwrap_or(0.0)),
            prop::collection::vec(arb_item(), 0..6),
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn test_prop_round_trip_uftp_small_packs(pages in prop::collection::vec(arb_page(), 0..4)) {
            let specs: Vec<PageSpec> = pages
                .iter()
                .map(|(pn, w, h, items)| {
                    let items: Vec<ItemSpec> = items
                        .iter()
                        .map(|(x, y, iw, ih, t)| (*x, *y, *iw, *ih, t.as_str()))
                        .collect();
                    (*pn, *w, *h, items)
                })
                .collect();
            let pack = uftp(&specs);

            // parse round-trips structurally
            let parsed = parse_uftp(&pack).unwrap();
            prop_assert_eq!(parsed.len(), pages.len());

            // generation always yields a valid zip with the right doc count
            let epub = text_pages_to_epub_native(&pack, "{}").unwrap();
            let expected_docs = pages.len().max(1);
            let mut ar = ZipArchive::new(Cursor::new(epub.clone())).unwrap();
            prop_assert!(ar.by_name("EPUB/package.opf").is_ok());
            drop(ar);
            for i in 1..=expected_docs {
                let href = format!("EPUB/text/page-{}.xhtml", pad_page(i));
                prop_assert!(has(&epub, &href));
            }
        }

        #[test]
        fn test_prop_round_trip_ufxp_small_packs(
            pages in prop::collection::vec(
                (1u32..1000, 1.0f32..2000.0, 1.0f32..2000.0, any::<u8>(), prop::collection::vec(any::<u8>(), 0..16)),
                0..4,
            )
        ) {
            let pack = ufxp(&pages);
            let parsed = parse_ufxp(&pack).unwrap();
            prop_assert_eq!(parsed.len(), pages.len());
            let epub = fixed_pages_to_epub_native(&pack, "{}").unwrap();
            prop_assert!(ZipArchive::new(Cursor::new(epub)).is_ok());
        }

        #[test]
        fn test_prop_xml_escape_safe(s in ".{0,80}") {
            let escaped = xml_escape(&s);
            // No raw markup characters survive escaping.
            prop_assert!(!escaped.contains('<'));
            prop_assert!(!escaped.contains('>'));
            // clean_xml_text is idempotent.
            let cleaned = clean_xml_text(&s);
            prop_assert_eq!(clean_xml_text(&cleaned), cleaned);
        }
    }
}
