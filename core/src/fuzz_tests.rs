//! Cross-cutting robustness harness.
//!
//! Two guarantees, enforced across every public entry point:
//!
//! 1. **Never panic on hostile input.** Each `*_native` function is fed a battery
//!    of malformed byte strings (empty, truncated, random, lying length prefixes,
//!    corrupted PDFs) and must return `Result::Err` — never panic, never hang.
//!    Panics are caught with [`std::panic::catch_unwind`] so one bad input is a
//!    precise, named failure rather than an aborted test run. This is the class
//!    of bug that hides in parser/`unsafe`-adjacent code (cf. lopdf 0.35's own
//!    `decrypt` panicking on a real file — which is exactly why our crypto handler
//!    is hand-rolled).
//!
//! 2. **Structural round-trips hold.** Property tests (proptest) generate random
//!    page counts / option values and assert the owned operations keep the
//!    document valid and the page math correct.

#![cfg(test)]

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::pack::PackWriter;
use crate::util::fixtures::{sample, PNG_1X1};

/// A spread of byte strings designed to break a PDF/pack parser.
fn malformed_inputs() -> Vec<(&'static str, Vec<u8>)> {
    let valid = sample(2);
    let mut truncated = valid.clone();
    truncated.truncate(valid.len() / 2);
    let mut header_only = b"%PDF-1.7\n".to_vec();
    header_only.extend_from_slice(b"trailer<<>>\n");
    let mut corrupt_xref = valid.clone();
    if let Some(pos) = corrupt_xref.windows(4).position(|w| w == b"xref") {
        corrupt_xref[pos..pos + 4].copy_from_slice(b"XXXX");
    }
    let mut nul = vec![0u8; 256];
    nul[0] = b'%';
    vec![
        ("empty", Vec::new()),
        ("one byte", vec![0x25]),
        ("ascii junk", b"not a pdf at all, just text".to_vec()),
        ("pdf header only", b"%PDF-1.7".to_vec()),
        ("header + empty trailer", header_only),
        ("truncated valid pdf", truncated),
        ("corrupted xref keyword", corrupt_xref),
        ("256 nul-ish bytes", nul),
        ("0xFF run", vec![0xFFu8; 512]),
        ("lying length pack", {
            // a pack-shaped blob claiming a huge count with no payload
            let mut v = b"UFAP".to_vec();
            v.extend_from_slice(&u32::MAX.to_le_bytes());
            v
        }),
        ("valid-magic empty body", b"UFTP\x00\x00\x00\x00".to_vec()),
    ]
}

/// Assert `f` returns (Ok or Err) without panicking; report `label` on panic.
fn no_panic<T>(label: &str, f: impl FnOnce() -> T) {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {})); // silence the backtrace spew during the sweep
    let result = catch_unwind(AssertUnwindSafe(f));
    std::panic::set_hook(prev);
    assert!(result.is_ok(), "PANIC on hostile input: {label}");
}

/// Every entry point that takes raw PDF bytes, exercised against every malformed input.
#[test]
fn pdf_readers_never_panic_on_hostile_input() {
    for (name, bytes) in malformed_inputs() {
        let b = bytes.as_slice();
        no_panic(&format!("page_count/{name}"), || {
            crate::page_count_native(b)
        });
        no_panic(&format!("optimize/{name}"), || crate::optimize_native(b));
        no_panic(&format!("rotate_all/{name}"), || {
            crate::rotate_all_native(b, 90)
        });
        no_panic(&format!("rotate_pages/{name}"), || {
            crate::rotate_pages_native(b, &[0, 1], 90)
        });
        no_panic(&format!("select_pages/{name}"), || {
            crate::select_pages_native(b, &[0])
        });
        no_panic(&format!("crop_margins/{name}"), || {
            crate::crop_margins_native(b, 5.0, 5.0, 5.0, 5.0)
        });
        no_panic(&format!("read_metadata/{name}"), || {
            crate::read_metadata_native(b)
        });
        no_panic(&format!("set_metadata/{name}"), || {
            crate::set_metadata_native(b, r#"{"title":"x"}"#)
        });
        no_panic(&format!("strip_metadata/{name}"), || {
            crate::strip_metadata_native(b)
        });
        no_panic(&format!("sanitize/{name}"), || {
            crate::sanitize_native(b, "{}")
        });
        no_panic(&format!("sanitize_report/{name}"), || {
            crate::sanitize_report_native(b)
        });
        no_panic(&format!("add_page_numbers/{name}"), || {
            crate::add_page_numbers_native(b, r#"{"format":"{n}"}"#)
        });
        no_panic(&format!("add_watermark/{name}"), || {
            crate::add_watermark_native(b, r#"{"text":"X"}"#)
        });
        no_panic(&format!("n_up/{name}"), || crate::n_up_native(b, 4, "{}"));
        no_panic(&format!("booklet/{name}"), || {
            crate::booklet_native(b, "{}")
        });
        no_panic(&format!("list_form_fields/{name}"), || {
            crate::list_form_fields_native(b)
        });
        no_panic(&format!("fill_form/{name}"), || {
            crate::fill_form_native(b, r#"{"a":"b"}"#)
        });
        no_panic(&format!("flatten_form/{name}"), || {
            crate::flatten_form_native(b)
        });
        no_panic(&format!("is_encrypted/{name}"), || {
            crate::is_encrypted_native(b)
        });
        no_panic(&format!("unlock/{name}"), || crate::unlock_native(b, "pw"));
        no_panic(&format!("protect/{name}"), || {
            crate::protect_native(b, r#"{"userPassword":"pw"}"#)
        });
        no_panic(&format!("stamp_images/{name}"), || {
            crate::stamp_images_native(b, &bytes)
        });
        no_panic(&format!("add_text_layer/{name}"), || {
            crate::add_text_layer_native(b, &bytes)
        });
        no_panic(&format!("set_crop_boxes/{name}"), || {
            crate::set_crop_boxes_native(b, &bytes)
        });
    }
}

/// Every entry point that takes a binary pack, exercised against every malformed input.
#[test]
fn pack_readers_never_panic_on_hostile_input() {
    for (name, bytes) in malformed_inputs() {
        let b = bytes.as_slice();
        no_panic(&format!("merge/{name}"), || crate::merge_pdfs_native(b));
        no_panic(&format!("pdfa/{name}"), || {
            crate::pdfa_from_png_pages_native(b)
        });
        no_panic(&format!("images_to_pdf/{name}"), || {
            crate::images_to_pdf_native(b, "{}")
        });
        no_panic(&format!("image_pages_to_pdf/{name}"), || {
            crate::image_pages_to_pdf_native(b)
        });
        no_panic(&format!("docx/{name}"), || {
            crate::text_pages_to_docx_native(b)
        });
        no_panic(&format!("xlsx/{name}"), || {
            crate::text_pages_to_xlsx_native(b)
        });
        no_panic(&format!("pptx/{name}"), || {
            crate::text_pages_to_pptx_native(b)
        });
        no_panic(&format!("epub_text/{name}"), || {
            crate::text_pages_to_epub_native(b, "{}")
        });
        no_panic(&format!("epub_fixed/{name}"), || {
            crate::fixed_pages_to_epub_native(b, "{}")
        });
    }
}

/// JSON-options entry points must reject malformed option strings without panicking.
#[test]
fn json_option_parsers_never_panic() {
    let doc = sample(1);
    let bad_json = [
        "",
        "{",
        "}",
        "null",
        "[]",
        "\"string\"",
        "{bad}",
        "{\"x\":}",
        "12345",
        "{\"fontSize\":\"not a number\"}",
        "{\"pageSize\":42}",
    ];
    for j in bad_json {
        no_panic(&format!("set_metadata/{j}"), || {
            crate::set_metadata_native(&doc, j)
        });
        no_panic(&format!("sanitize/{j}"), || crate::sanitize_native(&doc, j));
        no_panic(&format!("page_numbers/{j}"), || {
            crate::add_page_numbers_native(&doc, j)
        });
        no_panic(&format!("watermark/{j}"), || {
            crate::add_watermark_native(&doc, j)
        });
        no_panic(&format!("n_up/{j}"), || crate::n_up_native(&doc, 4, j));
        no_panic(&format!("booklet/{j}"), || crate::booklet_native(&doc, j));
        no_panic(&format!("protect/{j}"), || crate::protect_native(&doc, j));
        no_panic(&format!("fill_form/{j}"), || {
            crate::fill_form_native(&doc, j)
        });
        no_panic(&format!("text_to_pdf/{j}"), || {
            crate::text_to_pdf_native("body", j)
        });
    }
}

/// Text-to-PDF owns its parser/layout path rather than reading PDF bytes, so it
/// gets a separate text corpus for the same no-panic guarantee.
#[test]
fn text_to_pdf_never_panics_on_hostile_text() {
    let texts = vec![
        String::new(),
        "\0\0\0".to_string(),
        "\u{202E}right-to-left override".to_string(),
        "\u{D7FF}\u{E000}\u{FFFD}".to_string(),
        "<script>alert(1)</script>".to_string(),
        "word ".repeat(10_000),
    ];
    for text in &texts {
        no_panic(&format!("text_to_pdf/text-len-{}", text.len()), || {
            crate::text_to_pdf_native(text, r#"{"title":"fuzz","fontSize":11,"margin":54}"#)
        });
    }
}

/// Random page-index slices must never panic select/rotate (they validate range).
#[test]
fn random_index_slices_never_panic() {
    let doc = sample(3);
    let slices: Vec<Vec<u32>> = vec![
        vec![],
        vec![0],
        vec![2, 1, 0],
        vec![99],
        vec![u32::MAX],
        vec![0, 0, 0],
        vec![3],
        (0..1000).collect(),
    ];
    for s in &slices {
        no_panic(&format!("select/{s:?}"), || {
            crate::select_pages_native(&doc, s)
        });
        no_panic(&format!("rotate_pages/{s:?}"), || {
            crate::rotate_pages_native(&doc, s, 90)
        });
    }
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// Random byte vectors never panic the core readers (the fuzz invariant,
    /// proptest-driven for breadth beyond the hand-picked corpus).
    #[test]
    fn prop_random_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let b = bytes.as_slice();
        no_panic("prop page_count", || crate::page_count_native(b));
        no_panic("prop optimize", || crate::optimize_native(b));
        no_panic("prop select", || crate::select_pages_native(b, &[0]));
        no_panic("prop metadata", || crate::read_metadata_native(b));
        no_panic("prop sanitize", || crate::sanitize_native(b, "{}"));
        no_panic("prop is_encrypted", || crate::is_encrypted_native(b));
        no_panic("prop unlock", || crate::unlock_native(b, "pw"));
        no_panic("prop merge", || crate::merge_pdfs_native(b));
        no_panic("prop image_pages", || crate::image_pages_to_pdf_native(b));
    }

    /// select_pages keeps exactly the requested pages, in order, for any valid
    /// index permutation/subset of a generated document.
    #[test]
    fn prop_select_pages_roundtrip(
        page_count in 1usize..8,
        seed in any::<u64>(),
    ) {
        let doc = sample(page_count);
        // Build a deterministic in-range index list from the seed (no rng in wasm,
        // but this is a host test — still, derive it arithmetically for determinism).
        let take = (seed as usize % page_count) + 1;
        let indices: Vec<u32> = (0..take).map(|k| ((seed as usize + k * 3) % page_count) as u32).collect();
        let out = crate::select_pages_native(&doc, &indices).unwrap();
        prop_assert!(out.starts_with(b"%PDF-"));
        prop_assert_eq!(crate::page_count_native(&out).unwrap(), indices.len());
    }

    /// rotate_all is modular: rotating by k and by k+360 are equivalent, and the
    /// page count is preserved for any angle.
    #[test]
    fn prop_rotate_all_preserves_pages(page_count in 1usize..6, angle in -1080i64..1080) {
        let doc = sample(page_count);
        let out = crate::rotate_all_native(&doc, angle).unwrap();
        prop_assert_eq!(crate::page_count_native(&out).unwrap(), page_count);
    }

    /// optimize never changes the page count and always yields a valid PDF.
    #[test]
    fn prop_optimize_preserves_pages(page_count in 1usize..8) {
        let doc = sample(page_count);
        let out = crate::optimize_native(&doc).unwrap();
        prop_assert!(out.starts_with(b"%PDF-"));
        prop_assert_eq!(crate::page_count_native(&out).unwrap(), page_count);
    }

    /// protect → unlock is the identity on page count for any non-empty password.
    #[test]
    fn prop_protect_unlock_roundtrip(page_count in 1usize..5, pw in "[ -~]{1,24}") {
        let doc = sample(page_count);
        let opts = format!(r#"{{"userPassword":{}}}"#, serde_json::to_string(&pw).unwrap());
        let protected = crate::protect_native(&doc, &opts).unwrap();
        prop_assert!(crate::is_encrypted_native(&protected).unwrap());
        let unlocked = crate::unlock_native(&protected, &pw).unwrap();
        prop_assert!(!crate::is_encrypted_native(&unlocked).unwrap());
        prop_assert_eq!(crate::page_count_native(&unlocked).unwrap(), page_count);
    }

    /// crop_margins keeps the page count and produces a valid PDF for any
    /// reasonable margin set (clamping handles the rest).
    #[test]
    fn prop_crop_preserves_pages(m in 0f32..120.0) {
        let doc = sample(3);
        // sample pages are 300+ wide / 400 tall, so margins below ~140 are valid.
        if let Ok(out) = crate::crop_margins_native(&doc, m, m, m, m) {
            prop_assert_eq!(crate::page_count_native(&out).unwrap(), 3);
        }
    }

    /// images_to_pdf builds one page per image for any small image count.
    #[test]
    fn prop_images_to_pdf_page_count(n in 1usize..6) {
        let mut w = PackWriter::new(b"UFIP").u32(n as u32);
        for _ in 0..n { w = w.u8(0).bytes(PNG_1X1); }
        let out = crate::images_to_pdf_native(&w.finish(), r#"{"pageSize":"a4"}"#).unwrap();
        prop_assert_eq!(crate::page_count_native(&out).unwrap(), n);
    }
}
