//! Office document generation (DOCX / XLSX / PPTX) from extracted PDF text.
//!
//! This is the Rust port of `src/lib/tools/office.ts`. The browser extracts a
//! page's text *items* (one per PDF.js text run, each with a position + size)
//! and ships them across the WASM boundary as a single [`crate::pack`] blob with
//! the `UFTP` magic. Rust reconstructs the per-page reading-order text from those
//! items and emits an OOXML container (a ZIP of XML parts) for each format.
//!
//! ## Pack format (`UFTP`, little-endian)
//! ```text
//! b"UFTP"                         magic
//! u32   page_count
//! repeated page_count times:
//!   u32   page_number             1-based identifier (preserved verbatim)
//!   f32   width                   page width  in PDF points (currently unused)
//!   f32   height                  page height in PDF points (currently unused)
//!   u32   item_count
//!   repeated item_count times:
//!     f32 x                       left edge   in PDF points
//!     f32 y                       top edge    in PDF points
//!     f32 width                   item width  in PDF points
//!     f32 height                  item height in PDF points
//!     u32 text_len + bytes        UTF-8 text run
//! ```
//!
//! ## Parity note (read before wiring TS)
//! The original `office.ts` consumed an `ExtractedTextPage.text` string produced
//! directly by PDF.js. The `UFTP` pack intentionally carries **only items**, so
//! this module reconstructs `page.text` from the items (group into lines by `y`,
//! join runs within a line by a single space, join lines by `\n`). DOCX and PPTX
//! use that reconstructed text exclusively; XLSX uses the items directly (with the
//! reconstructed text only as the empty-items fallback). For inputs where PDF.js's
//! native `page.text` differs from this reconstruction the DOCX/PPTX bodies will
//! differ in line/word boundaries — see the integration notes if exact fidelity to
//! the old output is required (extend the pack with a per-page text string).

use crate::pack::PackReader;
use std::cmp::Ordering;
use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const OFFICE_PACK_MAGIC: &[u8; 4] = b"UFTP";

const NO_TEXT_MSG: &str =
    "No selectable text found. This PDF may be scanned; OCR is not available in this tool yet.";

const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const XLSX_MIME: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
const PPTX_MIME: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";

// ---------------------------------------------------------------------------
// Parsed model
// ---------------------------------------------------------------------------

/// One positioned text run extracted from a PDF page.
#[derive(Debug, Clone)]
struct ExtractedTextItem {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    text: String,
}

/// One PDF page's worth of extracted text plus its reconstructed reading order.
#[derive(Debug, Clone)]
struct ExtractedTextPage {
    page_number: u32,
    /// Page width in points (carried from the pack for completeness; unused by all
    /// three formats, which use fixed page/EMU dimensions like the TS original).
    #[allow(dead_code)]
    width: f32,
    /// Page height in points (carried from the pack for completeness; unused by all
    /// three formats, which use fixed page/EMU dimensions like the TS original).
    #[allow(dead_code)]
    height: f32,
    items: Vec<ExtractedTextItem>,
    /// Reading-order text reconstructed from `items` (newline-separated lines).
    text: String,
}

/// Decode a `UFTP` pack into pages, reconstructing each page's text from items.
fn parse_pack(pack: &[u8]) -> Result<Vec<ExtractedTextPage>, String> {
    let mut reader = PackReader::new(pack);
    reader.expect_magic(OFFICE_PACK_MAGIC)?;
    let page_count = reader.read_count(16)?;

    let mut pages = Vec::new();
    for _ in 0..page_count {
        let page_number = reader.read_u32()?;
        let width = reader.read_f32()?;
        let height = reader.read_f32()?;
        let item_count = reader.read_count(20)?;

        let mut items = Vec::new();
        for _ in 0..item_count {
            let x = reader.read_f32()?;
            let y = reader.read_f32()?;
            let w = reader.read_f32()?;
            let h = reader.read_f32()?;
            let text = reader.read_str()?.to_string();
            items.push(ExtractedTextItem {
                x,
                y,
                width: w,
                height: h,
                text,
            });
        }

        let text = reconstruct_page_text(&items);
        pages.push(ExtractedTextPage {
            page_number,
            width,
            height,
            items,
            text,
        });
    }

    reader.expect_done()?;
    Ok(pages)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Escape the five XML metacharacters, in the same order as `office.ts`.
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Split text into lines the way JavaScript's `/\r?\n/` does (a lone `\r` is kept).
fn split_lines(text: &str) -> Vec<&str> {
    text.split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect()
}

/// Split a string on runs of **2 or more** whitespace characters, mirroring
/// JavaScript's `String.split(/\s{2,}/)`. Single whitespace stays inside a token.
/// Pieces are returned untrimmed; callers trim and filter.
fn split_ws_runs(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut current = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            let start = i;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            if i - start >= 2 {
                out.push(std::mem::take(&mut current));
            } else {
                current.push(chars[start]);
            }
        } else {
            current.push(chars[i]);
            i += 1;
        }
    }
    out.push(current);
    out
}

/// Reproduce `splitLine`: prefer tab-delimited cells, then 2+-space-delimited
/// cells, otherwise the whole (trimmed) line as a single cell.
fn split_line(line: &str) -> Vec<String> {
    let tabbed: Vec<String> = line
        .split('\t')
        .map(|cell| cell.trim().to_string())
        .filter(|cell| !cell.is_empty())
        .collect();
    if tabbed.len() > 1 {
        return tabbed;
    }

    let spaced: Vec<String> = split_ws_runs(line)
        .into_iter()
        .map(|cell| cell.trim().to_string())
        .filter(|cell| !cell.is_empty())
        .collect();
    if spaced.len() > 1 {
        return spaced;
    }

    vec![line.trim().to_string()]
}

/// Items with non-blank text, sorted into reading order using the exact `office.ts`
/// comparator: descending `y` when the gap exceeds 3pt, otherwise ascending `x`.
fn sorted_items(items: &[ExtractedTextItem]) -> Vec<&ExtractedTextItem> {
    let mut sorted: Vec<&ExtractedTextItem> = items
        .iter()
        .filter(|it| !it.text.trim().is_empty())
        .collect();
    sorted.sort_by(|a, b| {
        if (b.y - a.y).abs() > 3.0 {
            // JS `b.y - a.y` (positive => a after b) == descending y.
            b.y.partial_cmp(&a.y).unwrap_or(Ordering::Equal)
        } else {
            a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal)
        }
    });
    sorted
}

/// Group sorted items into visual lines using the per-item y-tolerance
/// `max(3, item.height * 0.6)` (compared against the first item of the group).
fn group_lines(items: &[ExtractedTextItem]) -> Vec<Vec<&ExtractedTextItem>> {
    let sorted = sorted_items(items);
    let mut groups: Vec<Vec<&ExtractedTextItem>> = Vec::new();
    for item in sorted {
        let start_new = match groups.last() {
            None => true,
            Some(last) => (last[0].y - item.y).abs() > 3.0_f32.max(item.height * 0.6),
        };
        if start_new {
            groups.push(vec![item]);
        } else {
            groups.last_mut().unwrap().push(item);
        }
    }
    groups
}

/// Reconstruct a page's reading-order text from its items. Within a line, items
/// are sorted by `x` and grouped into cells: a horizontal gap wider than
/// `max(18, prev.height * 1.8)` starts a new cell (parity with office.ts). Words
/// inside a cell join with one space; cells join with two spaces (the column
/// separator that `split_line` later recognizes); lines join with `\n`.
fn reconstruct_page_text(items: &[ExtractedTextItem]) -> String {
    group_lines(items)
        .iter()
        .map(|line| {
            let mut line = line.clone();
            line.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal));
            let mut cells: Vec<String> = Vec::new();
            let mut current = String::new();
            let mut prev: Option<&ExtractedTextItem> = None;
            for it in &line {
                let gap = prev.map(|p| it.x - (p.x + p.width)).unwrap_or(0.0);
                let threshold = prev.map(|p| (p.height * 1.8).max(18.0)).unwrap_or(0.0);
                if prev.is_some() && gap > threshold {
                    if !current.trim().is_empty() {
                        cells.push(current.trim().to_string());
                    }
                    current = it.text.clone();
                } else if current.is_empty() {
                    current = it.text.clone();
                } else {
                    current.push(' ');
                    current.push_str(&it.text);
                }
                prev = Some(it);
            }
            if !current.trim().is_empty() {
                cells.push(current.trim().to_string());
            }
            cells.join("  ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Write the named XML parts into a DEFLATE ZIP with a fixed 1980-01-01 timestamp
/// (so identical input yields byte-identical output — no wall clock needed).
fn write_zip(files: &[(String, String)]) -> Result<Vec<u8>, String> {
    let fixed = zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)
        .map_err(|_| "Invalid fixed ZIP timestamp".to_string())?;
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(fixed);

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in files {
        zip.start_file(name.as_str(), options)
            .map_err(|e| e.to_string())?;
        zip.write_all(content.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let cursor = zip.finish().map_err(|e| e.to_string())?;
    Ok(cursor.into_inner())
}

// ---------------------------------------------------------------------------
// Shared OOXML fragments
// ---------------------------------------------------------------------------

fn content_types(overrides: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
{overrides}
</Types>"#
    )
}

fn package_rels(rel_type: &str, target: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="{rel_type}" Target="{target}"/>
</Relationships>"#
    )
}

const OFFICE_DOCUMENT_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

// ---------------------------------------------------------------------------
// DOCX
// ---------------------------------------------------------------------------

/// Render a `<w:t>` element, adding `xml:space="preserve"` when the (escaped)
/// text has leading/trailing whitespace.
fn docx_text(text: &str) -> String {
    let escaped = xml_escape(text);
    if escaped.trim() == escaped.as_str() {
        format!("<w:t>{escaped}</w:t>")
    } else {
        format!(r#"<w:t xml:space="preserve">{escaped}</w:t>"#)
    }
}

/// Render a `<w:p>` paragraph, optionally carrying a paragraph style id.
fn docx_paragraph(text: &str, style: Option<&str>) -> String {
    let p_pr = match style {
        Some(style) => format!(r#"<w:pPr><w:pStyle w:val="{style}"/></w:pPr>"#),
        None => String::new(),
    };
    format!("<w:p>{p_pr}<w:r>{}</w:r></w:p>", docx_text(text))
}

fn docx_document_rels() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#
        .to_string()
}

fn docx_styles_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/><w:pPr><w:spacing w:after="120"/></w:pPr><w:rPr><w:sz w:val="22"/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="ExtractedHeading"><w:name w:val="Extracted page heading"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:spacing w:before="160" w:after="120"/></w:pPr><w:rPr><w:b/><w:color w:val="0E1320"/><w:sz w:val="28"/></w:rPr></w:style>
</w:styles>"#
        .to_string()
}

/// Generate a DOCX from selectable PDF text, one paragraph per extracted line.
pub fn text_pages_to_docx_native(pack: &[u8]) -> Result<Vec<u8>, String> {
    let pages = parse_pack(pack)?;

    let mut body: Vec<String> = Vec::new();
    let non_empty = pages.iter().any(|page| !page.text.trim().is_empty());

    if !non_empty {
        body.push(docx_paragraph(NO_TEXT_MSG, None));
    } else {
        let last = pages.len() - 1;
        for (index, page) in pages.iter().enumerate() {
            let lines: Vec<&str> = split_lines(&page.text)
                .into_iter()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .collect();
            body.push(docx_paragraph(
                &format!("Page {}", page.page_number),
                Some("ExtractedHeading"),
            ));
            for line in lines {
                body.push(docx_paragraph(line, None));
            }
            if index < last {
                body.push(r#"<w:p><w:r><w:br w:type="page"/></w:r></w:p>"#.to_string());
            }
        }
    }

    let body_joined = body.join("\n    ");
    let document_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    {body_joined}
    <w:sectPr>
      <w:pgSz w:w="12240" w:h="15840"/>
      <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/>
    </w:sectPr>
  </w:body>
</w:document>"#
    );

    let ct = content_types(&format!(
        r#"  <Override PartName="/word/document.xml" ContentType="{DOCX_MIME}.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>"#
    ));

    let files = vec![
        ("[Content_Types].xml".to_string(), ct),
        (
            "_rels/.rels".to_string(),
            package_rels(OFFICE_DOCUMENT_REL, "word/document.xml"),
        ),
        ("word/document.xml".to_string(), document_xml),
        (
            "word/_rels/document.xml.rels".to_string(),
            docx_document_rels(),
        ),
        ("word/styles.xml".to_string(), docx_styles_xml()),
    ];
    write_zip(&files)
}

// ---------------------------------------------------------------------------
// XLSX
// ---------------------------------------------------------------------------

/// Reconstruct table rows for one page: from items when present (line grouping +
/// x-gap column splitting), otherwise from the page's (reconstructed) text.
fn rows_from_items(page: &ExtractedTextPage) -> Vec<Vec<String>> {
    if page.items.is_empty() {
        return split_lines(&page.text)
            .into_iter()
            .map(split_line)
            .filter(|row| row.iter().any(|cell| !cell.is_empty()))
            .collect();
    }

    group_lines(&page.items)
        .into_iter()
        .map(|line| {
            let mut line = line;
            line.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal));

            let mut cells: Vec<String> = Vec::new();
            let mut current = String::new();
            let mut previous: Option<&ExtractedTextItem> = None;
            for item in &line {
                let split = match previous {
                    Some(p) => (item.x - (p.x + p.width)) > 18.0_f32.max(p.height * 1.8),
                    None => false,
                };
                if split {
                    if !current.trim().is_empty() {
                        cells.push(current.trim().to_string());
                    }
                    current = item.text.clone();
                } else if current.is_empty() {
                    current = item.text.clone();
                } else {
                    current.push(' ');
                    current.push_str(&item.text);
                }
                previous = Some(item);
            }
            if !current.trim().is_empty() {
                cells.push(current.trim().to_string());
            }

            if cells.len() > 1 {
                cells
            } else {
                split_line(cells.first().map(String::as_str).unwrap_or(""))
            }
        })
        .filter(|row| row.iter().any(|cell| !cell.is_empty()))
        .collect()
}

/// Assemble the full sheet matrix: header row + one row per reconstructed line,
/// padded to the widest row. Falls back to a single "Note" row when no text.
fn sheet_rows(pages: &[ExtractedTextPage]) -> Vec<Vec<String>> {
    struct Row {
        page_number: u32,
        row_number: usize,
        cells: Vec<String>,
    }

    let mut rows: Vec<Row> = Vec::new();
    for page in pages {
        for (index, cells) in rows_from_items(page).into_iter().enumerate() {
            rows.push(Row {
                page_number: page.page_number,
                row_number: index + 1,
                cells,
            });
        }
    }

    if rows.is_empty() {
        return vec![vec!["Note".to_string()], vec![NO_TEXT_MSG.to_string()]];
    }

    let max_cells = rows.iter().map(|row| row.cells.len()).max().unwrap_or(0);
    let mut header = vec!["Page".to_string(), "Row".to_string()];
    for i in 1..=max_cells {
        header.push(format!("Column {i}"));
    }

    let mut result = vec![header];
    for row in rows {
        let cells_len = row.cells.len();
        let mut out_row = vec![row.page_number.to_string(), row.row_number.to_string()];
        out_row.extend(row.cells);
        for _ in 0..(max_cells - cells_len) {
            out_row.push(String::new());
        }
        result.push(out_row);
    }
    result
}

/// Convert a 0-based column index to A1-style letters (A, B, ..., Z, AA, AB, ...).
fn column_name(index: usize) -> String {
    let mut n = index + 1;
    let mut out = String::new();
    while n > 0 {
        let r = (n - 1) % 26;
        out.insert(0, (b'A' + r as u8) as char);
        n = (n - 1) / 26;
    }
    out
}

fn worksheet_xml(rows: &[Vec<String>]) -> String {
    let mut xml_rows = String::new();
    for (r_index, row) in rows.iter().enumerate() {
        let mut cells = String::new();
        for (c_index, value) in row.iter().enumerate() {
            let cell_ref = format!("{}{}", column_name(c_index), r_index + 1);
            let style = if r_index == 0 { r#" s="1""# } else { "" };
            cells.push_str(&format!(
                r#"<c r="{cell_ref}"{style} t="inlineStr"><is><t>{}</t></is></c>"#,
                xml_escape(value)
            ));
        }
        xml_rows.push_str(&format!(r#"<row r="{}">{cells}</row>"#, r_index + 1));
    }

    let max_cols = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let mut cols = String::new();
    for index in 0..max_cols {
        let width = if index < 2 { 10 } else { 24 };
        cols.push_str(&format!(
            r#"<col min="{}" max="{}" width="{width}" customWidth="1"/>"#,
            index + 1,
            index + 1
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <cols>{cols}</cols>
  <sheetData>{xml_rows}</sheetData>
</worksheet>"#
    )
}

fn xlsx_styles_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="2"><font><sz val="11"/><name val="Aptos"/></font><font><b/><sz val="11"/><color rgb="FF0E1320"/><name val="Aptos"/></font></fonts>
  <fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FFE8FFF5"/><bgColor indexed="64"/></patternFill></fill></fills>
  <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
  <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
  <cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1"/></cellXfs>
  <cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
</styleSheet>"#
        .to_string()
}

/// Generate a best-effort XLSX workbook from selectable PDF text rows.
pub fn text_pages_to_xlsx_native(pack: &[u8]) -> Result<Vec<u8>, String> {
    let pages = parse_pack(pack)?;
    let rows = sheet_rows(&pages);

    let workbook_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets><sheet name="Extracted text" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#
        .to_string();

    let workbook_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>"#
        .to_string();

    let ct = content_types(&format!(
        r#"  <Override PartName="/xl/workbook.xml" ContentType="{XLSX_MIME}.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>"#
    ));

    let files = vec![
        ("[Content_Types].xml".to_string(), ct),
        (
            "_rels/.rels".to_string(),
            package_rels(OFFICE_DOCUMENT_REL, "xl/workbook.xml"),
        ),
        ("xl/workbook.xml".to_string(), workbook_xml),
        ("xl/_rels/workbook.xml.rels".to_string(), workbook_rels),
        ("xl/worksheets/sheet1.xml".to_string(), worksheet_xml(&rows)),
        ("xl/styles.xml".to_string(), xlsx_styles_xml()),
    ];
    write_zip(&files)
}

// ---------------------------------------------------------------------------
// PPTX
// ---------------------------------------------------------------------------

fn ppt_paragraph(text: &str, size: u32, color: &str, bold: bool) -> String {
    let b = if bold { r#" b="1""# } else { "" };
    format!(
        r#"<a:p><a:r><a:rPr lang="en-US" sz="{size}"{b}><a:solidFill><a:srgbClr val="{color}"/></a:solidFill></a:rPr><a:t>{}</a:t></a:r></a:p>"#,
        xml_escape(text)
    )
}

fn ppt_shape(id: u32, name: &str, x: i64, y: i64, cx: i64, cy: i64, paragraphs: &str) -> String {
    format!(
        r#"<p:sp>
  <p:nvSpPr><p:cNvPr id="{id}" name="{}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr>
  <p:txBody><a:bodyPr wrap="square"/><a:lstStyle/>{paragraphs}</p:txBody>
</p:sp>"#,
        xml_escape(name)
    )
}

fn slide_xml(page: &ExtractedTextPage) -> String {
    let title = format!("Page {}", page.page_number);
    let lines: Vec<&str> = split_lines(&page.text)
        .into_iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    let body_lines: Vec<String> = if !lines.is_empty() {
        lines
            .iter()
            .take(16)
            .map(|line| ppt_paragraph(line, 2000, "263244", false))
            .collect()
    } else {
        vec![ppt_paragraph(NO_TEXT_MSG, 2000, "263244", false)]
    };

    let title_shape = ppt_shape(
        2,
        "Title",
        457200,
        274320,
        8229600,
        685800,
        &ppt_paragraph(&title, 3200, "0E1320", true),
    );
    let body_shape = ppt_shape(
        3,
        "Extracted text",
        685800,
        1188720,
        7772400,
        3474720,
        &body_lines.join(""),
    );

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
      {title_shape}
      {body_shape}
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sld>"#
    )
}

fn slide_layout_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank" preserve="1">
  <p:cSld name="Blank"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sldLayout>"#
        .to_string()
}

fn slide_master_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst>
  <p:txStyles><p:titleStyle/><p:bodyStyle/><p:otherStyle/></p:txStyles>
</p:sldMaster>"#
        .to_string()
}

fn theme_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Unfleece">
  <a:themeElements>
    <a:clrScheme name="Unfleece"><a:dk1><a:srgbClr val="0E1320"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="263244"/></a:dk2><a:lt2><a:srgbClr val="F5F7FA"/></a:lt2><a:accent1><a:srgbClr val="2ECC8F"/></a:accent1><a:accent2><a:srgbClr val="4F8CFF"/></a:accent2><a:accent3><a:srgbClr val="F5B84B"/></a:accent3><a:accent4><a:srgbClr val="FB6F84"/></a:accent4><a:accent5><a:srgbClr val="7C6CFF"/></a:accent5><a:accent6><a:srgbClr val="20B8A5"/></a:accent6><a:hlink><a:srgbClr val="4F8CFF"/></a:hlink><a:folHlink><a:srgbClr val="7C6CFF"/></a:folHlink></a:clrScheme>
    <a:fontScheme name="Unfleece"><a:majorFont><a:latin typeface="Aptos Display"/></a:majorFont><a:minorFont><a:latin typeface="Aptos"/></a:minorFont></a:fontScheme>
    <a:fmtScheme name="Unfleece"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="9525"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme>
  </a:themeElements>
</a:theme>"#
        .to_string()
}

/// Generate a PPTX from selectable PDF text, one slide per PDF page.
pub fn text_pages_to_pptx_native(pack: &[u8]) -> Result<Vec<u8>, String> {
    let pages = parse_pack(pack)?;
    // Match office.ts: an empty deck still produces a single blank slide.
    let deck: Vec<ExtractedTextPage> = if pages.is_empty() {
        vec![ExtractedTextPage {
            page_number: 1,
            width: 0.0,
            height: 0.0,
            items: Vec::new(),
            text: String::new(),
        }]
    } else {
        pages
    };
    let n = deck.len();

    let slide_overrides = (0..n)
        .map(|i| {
            format!(
                r#"  <Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#,
                i + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let slide_ids = (0..n)
        .map(|i| format!(r#"<p:sldId id="{}" r:id="rId{}"/>"#, 256 + i, i + 1))
        .collect::<Vec<_>>()
        .join("");

    let mut rels: Vec<String> = (0..n)
        .map(|i| {
            format!(
                r#"  <Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
                i + 1,
                i + 1
            )
        })
        .collect();
    rels.push(format!(
        r#"  <Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>"#,
        n + 1
    ));
    rels.push(format!(
        r#"  <Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>"#,
        n + 2
    ));
    let presentation_rels = rels.join("\n");

    let presentation_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId{}"/></p:sldMasterIdLst>
  <p:sldIdLst>{slide_ids}</p:sldIdLst>
  <p:sldSz cx="9144000" cy="5143500" type="screen16x9"/>
  <p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#,
        n + 1
    );

    let ct = content_types(&format!(
        r#"  <Override PartName="/ppt/presentation.xml" ContentType="{PPTX_MIME}.main+xml"/>
  <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
  <Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
{slide_overrides}"#
    ));

    let presentation_rels_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
{presentation_rels}
</Relationships>"#
    );

    let slide_master_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#
        .to_string();

    let slide_layout_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#
        .to_string();

    let mut files: Vec<(String, String)> = vec![
        ("[Content_Types].xml".to_string(), ct),
        (
            "_rels/.rels".to_string(),
            package_rels(OFFICE_DOCUMENT_REL, "ppt/presentation.xml"),
        ),
        ("ppt/presentation.xml".to_string(), presentation_xml),
        (
            "ppt/_rels/presentation.xml.rels".to_string(),
            presentation_rels_xml,
        ),
        (
            "ppt/slideMasters/slideMaster1.xml".to_string(),
            slide_master_xml(),
        ),
        (
            "ppt/slideMasters/_rels/slideMaster1.xml.rels".to_string(),
            slide_master_rels,
        ),
        (
            "ppt/slideLayouts/slideLayout1.xml".to_string(),
            slide_layout_xml(),
        ),
        (
            "ppt/slideLayouts/_rels/slideLayout1.xml.rels".to_string(),
            slide_layout_rels,
        ),
        ("ppt/theme/theme1.xml".to_string(), theme_xml()),
    ];

    for (index, page) in deck.iter().enumerate() {
        files.push((
            format!("ppt/slides/slide{}.xml", index + 1),
            slide_xml(page),
        ));
        files.push((
            format!("ppt/slides/_rels/slide{}.xml.rels", index + 1),
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#
                .to_string(),
        ));
    }

    write_zip(&files)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackWriter;
    use std::collections::HashMap;
    use std::io::Read;

    type ItemSpec<'a> = (f32, f32, f32, f32, &'a str);
    type PageSpec<'a> = (u32, f32, f32, Vec<ItemSpec<'a>>);

    /// Build a `UFTP` pack from `(page_number, width, height, [(x, y, w, h, text)])`.
    fn build_pack(pages: &[PageSpec]) -> Vec<u8> {
        let mut w = PackWriter::new(OFFICE_PACK_MAGIC).u32(pages.len() as u32);
        for (num, width, height, items) in pages {
            w = w.u32(*num).f32(*width).f32(*height).u32(items.len() as u32);
            for (x, y, iw, ih, text) in items {
                w = w.f32(*x).f32(*y).f32(*iw).f32(*ih).str(text);
            }
        }
        w.finish()
    }

    /// Unzip into a name -> contents map (every part is UTF-8 XML).
    fn unzip(bytes: &[u8]) -> HashMap<String, String> {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).unwrap();
        let mut map = HashMap::new();
        for i in 0..archive.len() {
            let mut f = archive.by_index(i).unwrap();
            // Every part we emit is DEFLATE-compressed.
            assert_eq!(f.compression(), CompressionMethod::Deflated);
            let name = f.name().to_string();
            let mut content = String::new();
            f.read_to_string(&mut content).unwrap();
            map.insert(name, content);
        }
        map
    }

    fn page(num: u32, items: Vec<ItemSpec>) -> PageSpec {
        (num, 612.0, 792.0, items)
    }

    // ----- pack parsing -----------------------------------------------------

    #[test]
    fn pack_roundtrip_single_page() {
        let pack = build_pack(&[page(1, vec![(50.0, 700.0, 200.0, 24.0, "Invoice")])]);
        let pages = parse_pack(&pack).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page_number, 1);
        assert_eq!(pages[0].width, 612.0);
        assert_eq!(pages[0].height, 792.0);
        assert_eq!(pages[0].items.len(), 1);
        assert_eq!(pages[0].items[0].text, "Invoice");
        assert_eq!(pages[0].items[0].x, 50.0);
        assert_eq!(pages[0].text, "Invoice");
    }

    #[test]
    fn pack_roundtrip_multiple_pages() {
        let specs: Vec<PageSpec> = (1..=5)
            .map(|n| {
                page(
                    n,
                    vec![
                        (10.0, 700.0, 40.0, 12.0, "a"),
                        (10.0, 680.0, 40.0, 12.0, "b"),
                        (10.0, 660.0, 40.0, 12.0, "c"),
                    ],
                )
            })
            .collect();
        let pack = build_pack(&specs);
        let pages = parse_pack(&pack).unwrap();
        assert_eq!(pages.len(), 5);
        for (i, p) in pages.iter().enumerate() {
            assert_eq!(p.page_number, (i + 1) as u32);
            assert_eq!(p.items.len(), 3);
            assert_eq!(p.text, "a\nb\nc");
        }
    }

    // ----- DOCX -------------------------------------------------------------

    #[test]
    fn docx_empty_pages() {
        let out = text_pages_to_docx_native(&build_pack(&[])).unwrap();
        let files = unzip(&out);
        assert!(files.contains_key("[Content_Types].xml"));
        assert!(files.contains_key("word/document.xml"));
        assert!(files.contains_key("word/styles.xml"));
        // No pages => "no selectable text" fallback paragraph.
        assert!(files["word/document.xml"].contains("No selectable text found"));
    }

    #[test]
    fn docx_no_selectable_text() {
        // A page whose only item is blank => reconstructed text empty => fallback.
        let out =
            text_pages_to_docx_native(&build_pack(&[page(1, vec![(0.0, 0.0, 0.0, 0.0, "   ")])]))
                .unwrap();
        let doc = unzip(&out).remove("word/document.xml").unwrap();
        assert!(doc.contains("No selectable text found"));
        assert!(!doc.contains("Page 1"));
    }

    #[test]
    fn docx_single_line_per_page() {
        let specs: Vec<PageSpec> = (1..=3)
            .map(|n| page(n, vec![(10.0, 700.0, 50.0, 12.0, "Line")]))
            .collect();
        let out = text_pages_to_docx_native(&build_pack(&specs)).unwrap();
        let doc = unzip(&out).remove("word/document.xml").unwrap();
        // 3 headings + 3 lines.
        assert_eq!(doc.matches("ExtractedHeading").count(), 3);
        assert_eq!(doc.matches("Page 1").count(), 1);
        assert_eq!(doc.matches("Page 3").count(), 1);
        assert_eq!(doc.matches("<w:t>Line</w:t>").count(), 3);
    }

    #[test]
    fn docx_multiline_text() {
        // Three items stacked vertically => three lines in one page.
        let out = text_pages_to_docx_native(&build_pack(&[page(
            1,
            vec![
                (10.0, 700.0, 50.0, 12.0, "line1"),
                (10.0, 680.0, 50.0, 12.0, "line2"),
                (10.0, 660.0, 50.0, 12.0, "line3"),
            ],
        )]))
        .unwrap();
        let doc = unzip(&out).remove("word/document.xml").unwrap();
        assert!(doc.contains("<w:t>line1</w:t>"));
        assert!(doc.contains("<w:t>line2</w:t>"));
        assert!(doc.contains("<w:t>line3</w:t>"));
    }

    #[test]
    fn docx_xml_escaping() {
        let out = text_pages_to_docx_native(&build_pack(&[page(
            1,
            vec![(10.0, 700.0, 200.0, 12.0, "<a> & \"b\" 'c'")],
        )]))
        .unwrap();
        let doc = unzip(&out).remove("word/document.xml").unwrap();
        assert!(doc.contains("&lt;a&gt; &amp; &quot;b&quot; &apos;c&apos;"));
        assert!(!doc.contains("<a>"));
    }

    #[test]
    fn docx_page_break_insertion() {
        let specs: Vec<PageSpec> = (1..=2)
            .map(|n| page(n, vec![(10.0, 700.0, 50.0, 12.0, "x")]))
            .collect();
        let out = text_pages_to_docx_native(&build_pack(&specs)).unwrap();
        let doc = unzip(&out).remove("word/document.xml").unwrap();
        // Exactly one page break between the two pages.
        assert_eq!(doc.matches(r#"<w:br w:type="page"/>"#).count(), 1);
    }

    #[test]
    fn docx_heading_style_present() {
        let out = text_pages_to_docx_native(&build_pack(&[page(
            7,
            vec![(10.0, 700.0, 50.0, 12.0, "hi")],
        )]))
        .unwrap();
        let files = unzip(&out);
        assert!(files["word/styles.xml"].contains(r#"w:styleId="ExtractedHeading""#));
        assert!(files["word/document.xml"].contains(r#"<w:pStyle w:val="ExtractedHeading"/>"#));
        assert!(files["word/document.xml"].contains("Page 7"));
    }

    #[test]
    fn docx_preserve_whitespace_helper() {
        // The full pipeline trims lines, so exercise docx_text directly.
        assert_eq!(docx_text("hello"), "<w:t>hello</w:t>");
        assert_eq!(
            docx_text("  hello  "),
            r#"<w:t xml:space="preserve">  hello  </w:t>"#
        );
    }

    #[test]
    fn docx_zip_structure() {
        let out = text_pages_to_docx_native(&build_pack(&[page(
            1,
            vec![(10.0, 700.0, 50.0, 12.0, "x")],
        )]))
        .unwrap();
        let files = unzip(&out);
        for name in [
            "[Content_Types].xml",
            "_rels/.rels",
            "word/document.xml",
            "word/_rels/document.xml.rels",
            "word/styles.xml",
        ] {
            assert!(files.contains_key(name), "missing {name}");
        }
        assert!(files["[Content_Types].xml"].contains("/word/document.xml"));
        assert!(files["_rels/.rels"].contains("word/document.xml"));
        for content in files.values() {
            assert!(content.starts_with("<?xml version=\"1.0\""));
        }
    }

    // ----- XLSX -------------------------------------------------------------

    #[test]
    fn xlsx_row_grouping_by_y() {
        // y=[700,700,650,650], small heights => 2 line groups => 2 data rows.
        let out = text_pages_to_xlsx_native(&build_pack(&[page(
            1,
            vec![
                (50.0, 700.0, 40.0, 10.0, "A"),
                (100.0, 700.0, 40.0, 10.0, "B"),
                (50.0, 650.0, 40.0, 10.0, "C"),
                (100.0, 650.0, 40.0, 10.0, "D"),
            ],
        )]))
        .unwrap();
        let sheet = unzip(&out).remove("xl/worksheets/sheet1.xml").unwrap();
        // header + 2 data rows.
        assert_eq!(sheet.matches("<row ").count(), 3);
        assert!(sheet.contains("A B"));
        assert!(sheet.contains("C D"));
    }

    #[test]
    fn xlsx_column_splitting_by_x() {
        // Two items on one line separated by a big x-gap => two columns.
        let out = text_pages_to_xlsx_native(&build_pack(&[page(
            1,
            vec![
                (50.0, 700.0, 30.0, 10.0, "Name"),
                (200.0, 700.0, 30.0, 10.0, "Alice"),
            ],
        )]))
        .unwrap();
        let sheet = unzip(&out).remove("xl/worksheets/sheet1.xml").unwrap();
        assert!(sheet.contains("Column 1"));
        assert!(sheet.contains("Column 2"));
        assert!(sheet.contains("<is><t>Name</t></is>"));
        assert!(sheet.contains("<is><t>Alice</t></is>"));
    }

    #[test]
    fn xlsx_fallback_text_splitting_direct() {
        // The UFTP pack carries no page text, so exercise the empty-items text
        // branch by constructing the page model directly.
        let page = ExtractedTextPage {
            page_number: 1,
            width: 0.0,
            height: 0.0,
            items: Vec::new(),
            text: "Header\tValue\nRow1\tValue1".to_string(),
        };
        let rows = sheet_rows(std::slice::from_ref(&page));
        assert_eq!(rows[0], vec!["Page", "Row", "Column 1", "Column 2"]);
        assert_eq!(rows[1], vec!["1", "1", "Header", "Value"]);
        assert_eq!(rows[2], vec!["1", "2", "Row1", "Value1"]);
    }

    #[test]
    fn xlsx_two_or_more_spaces_split_direct() {
        assert_eq!(split_line("a  b   c"), vec!["a", "b", "c"]);
        // single spaces are preserved inside a single cell
        assert_eq!(split_line("a b c"), vec!["a b c"]);
        // tabs take priority
        assert_eq!(split_line("a\tb\tc"), vec!["a", "b", "c"]);
    }

    #[test]
    fn xlsx_header_row() {
        let out = text_pages_to_xlsx_native(&build_pack(&[page(
            9,
            vec![
                (50.0, 700.0, 30.0, 10.0, "K"),
                (200.0, 700.0, 30.0, 10.0, "V"),
            ],
        )]))
        .unwrap();
        let sheet = unzip(&out).remove("xl/worksheets/sheet1.xml").unwrap();
        assert!(sheet.contains("<is><t>Page</t></is>"));
        assert!(sheet.contains("<is><t>Row</t></is>"));
        assert!(sheet.contains("<is><t>Column 1</t></is>"));
        // first data row prefixed with page number 9 and row number 1
        assert!(sheet.contains(r#"<row r="2">"#));
        assert!(sheet.contains("<is><t>9</t></is>"));
    }

    #[test]
    fn xlsx_column_naming() {
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(1), "B");
        assert_eq!(column_name(25), "Z");
        assert_eq!(column_name(26), "AA");
        assert_eq!(column_name(27), "AB");
        assert_eq!(column_name(51), "AZ");
        assert_eq!(column_name(52), "BA");
        assert_eq!(column_name(701), "ZZ");
        assert_eq!(column_name(702), "AAA");
    }

    #[test]
    fn xlsx_cell_references() {
        let out = text_pages_to_xlsx_native(&build_pack(&[page(
            1,
            vec![
                (50.0, 700.0, 30.0, 10.0, "K"),
                (200.0, 700.0, 30.0, 10.0, "V"),
            ],
        )]))
        .unwrap();
        let sheet = unzip(&out).remove("xl/worksheets/sheet1.xml").unwrap();
        assert!(sheet.contains(r#"<c r="A1""#));
        assert!(sheet.contains(r#"<c r="B1""#));
        assert!(sheet.contains(r#"<c r="C1""#));
        assert!(sheet.contains(r#"<c r="A2""#));
    }

    #[test]
    fn xlsx_style_header_only() {
        let out = text_pages_to_xlsx_native(&build_pack(&[page(
            1,
            vec![(50.0, 700.0, 30.0, 10.0, "K")],
        )]))
        .unwrap();
        let sheet = unzip(&out).remove("xl/worksheets/sheet1.xml").unwrap();
        // header cells carry s="1"; data cells carry no style.
        assert!(sheet.contains(r#"<c r="A1" s="1" t="inlineStr">"#));
        assert!(sheet.contains(r#"<c r="A2" t="inlineStr">"#));
        assert!(!sheet.contains(r#"<c r="A2" s="1""#));
    }

    #[test]
    fn xlsx_no_selectable_text() {
        let out = text_pages_to_xlsx_native(&build_pack(&[])).unwrap();
        let sheet = unzip(&out).remove("xl/worksheets/sheet1.xml").unwrap();
        assert!(sheet.contains("<is><t>Note</t></is>"));
        assert!(sheet.contains("No selectable text found"));
    }

    #[test]
    fn xlsx_xml_escaping() {
        let out = text_pages_to_xlsx_native(&build_pack(&[page(
            1,
            vec![(50.0, 700.0, 200.0, 10.0, "<x> & 'y'")],
        )]))
        .unwrap();
        let sheet = unzip(&out).remove("xl/worksheets/sheet1.xml").unwrap();
        assert!(sheet.contains("&lt;x&gt; &amp; &apos;y&apos;"));
    }

    #[test]
    fn xlsx_zip_structure() {
        let out = text_pages_to_xlsx_native(&build_pack(&[page(
            1,
            vec![(50.0, 700.0, 30.0, 10.0, "K")],
        )]))
        .unwrap();
        let files = unzip(&out);
        for name in [
            "[Content_Types].xml",
            "_rels/.rels",
            "xl/workbook.xml",
            "xl/_rels/workbook.xml.rels",
            "xl/worksheets/sheet1.xml",
            "xl/styles.xml",
        ] {
            assert!(files.contains_key(name), "missing {name}");
        }
        assert!(files["xl/styles.xml"].contains("FFE8FFF5"));
    }

    // ----- PPTX -------------------------------------------------------------

    #[test]
    fn pptx_one_slide_per_page() {
        let specs: Vec<PageSpec> = (1..=3)
            .map(|n| page(n, vec![(10.0, 700.0, 50.0, 12.0, "x")]))
            .collect();
        let out = text_pages_to_pptx_native(&build_pack(&specs)).unwrap();
        let files = unzip(&out);
        for i in 1..=3 {
            assert!(files.contains_key(&format!("ppt/slides/slide{i}.xml")));
            assert!(files.contains_key(&format!("ppt/slides/_rels/slide{i}.xml.rels")));
        }
        assert!(!files.contains_key("ppt/slides/slide4.xml"));
    }

    #[test]
    fn pptx_slide_title_format() {
        let out = text_pages_to_pptx_native(&build_pack(&[page(
            5,
            vec![(10.0, 700.0, 50.0, 12.0, "body")],
        )]))
        .unwrap();
        let slide = unzip(&out).remove("ppt/slides/slide1.xml").unwrap();
        assert!(slide.contains(r#"sz="3200" b="1""#));
        assert!(slide.contains("0E1320"));
        assert!(slide.contains("<a:t>Page 5</a:t>"));
        assert!(slide.contains(r#"<a:off x="457200" y="274320"/>"#));
    }

    #[test]
    fn pptx_slide_body_lines_capped_at_16() {
        let items: Vec<ItemSpec> = (0..20).map(|_| (10.0, 0.0, 50.0, 12.0, "ln")).collect();
        // Stack 20 lines vertically by decreasing y.
        let items: Vec<ItemSpec> = items
            .into_iter()
            .enumerate()
            .map(|(i, (x, _, w, h, t))| (x, 700.0 - i as f32 * 20.0, w, h, t))
            .collect();
        let out = text_pages_to_pptx_native(&build_pack(&[page(1, items)])).unwrap();
        let slide = unzip(&out).remove("ppt/slides/slide1.xml").unwrap();
        // 1 title paragraph + 16 body paragraphs = 17 total <a:p>.
        assert_eq!(slide.matches("<a:p>").count(), 17);
    }

    #[test]
    fn pptx_no_selectable_text() {
        let out =
            text_pages_to_pptx_native(&build_pack(&[page(1, vec![(0.0, 0.0, 0.0, 0.0, "   ")])]))
                .unwrap();
        let slide = unzip(&out).remove("ppt/slides/slide1.xml").unwrap();
        assert!(slide.contains("No selectable text found"));
    }

    #[test]
    fn pptx_empty_pages_single_blank_slide() {
        let out = text_pages_to_pptx_native(&build_pack(&[])).unwrap();
        let files = unzip(&out);
        assert!(files.contains_key("ppt/slides/slide1.xml"));
        assert!(!files.contains_key("ppt/slides/slide2.xml"));
        assert!(files["ppt/slides/slide1.xml"].contains("Page 1"));
        assert!(files["ppt/slides/slide1.xml"].contains("No selectable text found"));
    }

    #[test]
    fn pptx_xml_escaping() {
        let out = text_pages_to_pptx_native(&build_pack(&[page(
            1,
            vec![(10.0, 700.0, 200.0, 12.0, "<b> & 'c'")],
        )]))
        .unwrap();
        let slide = unzip(&out).remove("ppt/slides/slide1.xml").unwrap();
        assert!(slide.contains("&lt;b&gt; &amp; &apos;c&apos;"));
    }

    #[test]
    fn pptx_theme_presence() {
        let out = text_pages_to_pptx_native(&build_pack(&[page(
            1,
            vec![(10.0, 700.0, 50.0, 12.0, "x")],
        )]))
        .unwrap();
        let theme = unzip(&out).remove("ppt/theme/theme1.xml").unwrap();
        for hex in ["0E1320", "FFFFFF", "2ECC8F", "4F8CFF"] {
            assert!(theme.contains(hex), "theme missing {hex}");
        }
    }

    #[test]
    fn pptx_presentation_structure() {
        let specs: Vec<PageSpec> = (1..=4)
            .map(|n| page(n, vec![(10.0, 700.0, 50.0, 12.0, "x")]))
            .collect();
        let out = text_pages_to_pptx_native(&build_pack(&specs)).unwrap();
        let files = unzip(&out);
        let pres = &files["ppt/presentation.xml"];
        assert_eq!(pres.matches("<p:sldId ").count(), 4);
        // master rel id == slide count + 1
        assert!(pres.contains(r#"<p:sldMasterId id="2147483648" r:id="rId5"/>"#));
        let rels = &files["ppt/_rels/presentation.xml.rels"];
        assert!(rels.contains(r#"Id="rId5""#)); // slideMaster
        assert!(rels.contains(r#"Id="rId6""#)); // theme
    }

    #[test]
    fn pptx_layout_and_master() {
        let out = text_pages_to_pptx_native(&build_pack(&[page(
            1,
            vec![(10.0, 700.0, 50.0, 12.0, "x")],
        )]))
        .unwrap();
        let files = unzip(&out);
        for name in [
            "ppt/slideMasters/slideMaster1.xml",
            "ppt/slideMasters/_rels/slideMaster1.xml.rels",
            "ppt/slideLayouts/slideLayout1.xml",
            "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        ] {
            assert!(files.contains_key(name), "missing {name}");
        }
        assert!(files["ppt/slideMasters/_rels/slideMaster1.xml.rels"]
            .contains("../slideLayouts/slideLayout1.xml"));
    }

    #[test]
    fn pptx_zip_structure_lists_all_slides() {
        let specs: Vec<PageSpec> = (1..=2)
            .map(|n| page(n, vec![(10.0, 700.0, 50.0, 12.0, "x")]))
            .collect();
        let out = text_pages_to_pptx_native(&build_pack(&specs)).unwrap();
        let ct = unzip(&out).remove("[Content_Types].xml").unwrap();
        assert!(ct.contains("/ppt/slides/slide1.xml"));
        assert!(ct.contains("/ppt/slides/slide2.xml"));
        assert!(ct.contains("/ppt/theme/theme1.xml"));
        assert!(ct.contains("/ppt/slideLayouts/slideLayout1.xml"));
    }

    // ----- determinism ------------------------------------------------------

    #[test]
    fn output_is_deterministic() {
        let pack = build_pack(&[page(1, vec![(10.0, 700.0, 50.0, 12.0, "x")])]);
        assert_eq!(
            text_pages_to_docx_native(&pack).unwrap(),
            text_pages_to_docx_native(&pack).unwrap()
        );
        assert_eq!(
            text_pages_to_xlsx_native(&pack).unwrap(),
            text_pages_to_xlsx_native(&pack).unwrap()
        );
        assert_eq!(
            text_pages_to_pptx_native(&pack).unwrap(),
            text_pages_to_pptx_native(&pack).unwrap()
        );
    }

    // ----- errors -----------------------------------------------------------

    #[test]
    fn error_invalid_pack_magic() {
        let bad = PackWriter::new(b"XXXX").u32(0).finish();
        for f in [
            text_pages_to_docx_native,
            text_pages_to_xlsx_native,
            text_pages_to_pptx_native,
        ] {
            let err = f(&bad).unwrap_err();
            assert_eq!(err, "Invalid input pack");
        }
    }

    #[test]
    fn error_truncated_pack() {
        // Declares 1 page but provides no page bytes.
        let truncated = PackWriter::new(OFFICE_PACK_MAGIC).u32(1).finish();
        assert!(text_pages_to_docx_native(&truncated).is_err());

        // Declares 1 item but cuts off mid-item.
        let mut t2 = PackWriter::new(OFFICE_PACK_MAGIC)
            .u32(1)
            .u32(1)
            .f32(612.0)
            .f32(792.0)
            .u32(1)
            .finish();
        t2.extend_from_slice(&[0u8, 0, 0]); // partial f32
        assert!(text_pages_to_xlsx_native(&t2).is_err());
    }

    #[test]
    fn error_trailing_data() {
        let mut pack = build_pack(&[page(1, vec![(10.0, 700.0, 50.0, 12.0, "x")])]);
        pack.push(0);
        assert!(text_pages_to_pptx_native(&pack).is_err());
    }

    #[test]
    fn error_invalid_utf8_text() {
        // Hand-build a pack with a 1-byte text whose byte is invalid UTF-8 (0xFF).
        let mut pack = PackWriter::new(OFFICE_PACK_MAGIC)
            .u32(1)
            .u32(1)
            .f32(612.0)
            .f32(792.0)
            .u32(1)
            .f32(10.0)
            .f32(700.0)
            .f32(50.0)
            .f32(12.0)
            .u32(1) // text_len = 1
            .finish();
        pack.push(0xFF);
        let err = text_pages_to_docx_native(&pack).unwrap_err();
        assert!(err.contains("Invalid UTF-8"), "got: {err}");
    }

    // ----- proptest ---------------------------------------------------------

    use proptest::prelude::*;

    fn arb_item() -> impl Strategy<Value = (f32, f32, f32, f32, String)> {
        (
            -5000.0f32..5000.0,
            -5000.0f32..5000.0,
            0.0f32..500.0,
            0.0f32..80.0,
            "[ -~\\t]{0,12}", // printable ASCII incl. space + tab
        )
    }

    fn arb_pack() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec((1u32..50, prop::collection::vec(arb_item(), 0..8)), 0..6).prop_map(
            |pages| {
                let mut w = PackWriter::new(OFFICE_PACK_MAGIC).u32(pages.len() as u32);
                for (num, items) in &pages {
                    w = w.u32(*num).f32(612.0).f32(792.0).u32(items.len() as u32);
                    for (x, y, iw, ih, text) in items {
                        w = w.f32(*x).f32(*y).f32(*iw).f32(*ih).str(text);
                    }
                }
                w.finish()
            },
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(120))]

        #[test]
        fn proptest_valid_packs_never_panic(pack in arb_pack()) {
            // All three generators must complete and produce a parseable ZIP.
            for out in [
                text_pages_to_docx_native(&pack).unwrap(),
                text_pages_to_xlsx_native(&pack).unwrap(),
                text_pages_to_pptx_native(&pack).unwrap(),
            ] {
                let files = unzip(&out);
                prop_assert!(files.contains_key("[Content_Types].xml"));
                // every emitted part is a well-formed XML declaration
                for content in files.values() {
                    prop_assert!(content.starts_with("<?xml version=\"1.0\""));
                }
            }
        }

        #[test]
        fn proptest_round_trip_pack_structure(pack in arb_pack()) {
            let pages = parse_pack(&pack).unwrap();
            let docx = unzip(&text_pages_to_docx_native(&pack).unwrap());
            // styles + document always present regardless of content
            prop_assert!(docx.contains_key("word/document.xml"));
            prop_assert!(docx.contains_key("word/styles.xml"));

            let pptx = unzip(&text_pages_to_pptx_native(&pack).unwrap());
            // one slide per page, but at least one when the deck is empty
            let expected = pages.len().max(1);
            let slide_count = pptx
                .keys()
                .filter(|k| k.starts_with("ppt/slides/slide") && k.ends_with(".xml") && !k.contains("_rels"))
                .count();
            prop_assert_eq!(slide_count, expected);
        }
    }
}
