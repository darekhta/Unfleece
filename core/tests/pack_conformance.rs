//! Golden byte-layout conformance for every pack the crate consumes.
//!
//! The TypeScript `PackBuilder` (`src/lib/wasm/pack.ts`) must produce byte-for-byte
//! identical packs by hand, so these tests pin each pack's *exact* little-endian
//! layout. Any accidental change to [`PackWriter`] — field order, primitive
//! encoding, or the magic itself — fails loudly here, flagging that `pack.ts`
//! needs the matching change before the JS↔WASM contract drifts.
//!
//! A property test additionally proves `PackWriter` → `PackReader` round-trips all
//! primitive types for random inputs.

use proptest::prelude::*;
use unfleece_core::pack::{PackReader, PackWriter, PACK_VERSION};

// ---------------------------------------------------------------------------
// Spelled-out little-endian field encodings (the "golden" source of truth —
// deliberately *not* produced via PackWriter, so the test is independent).
// ---------------------------------------------------------------------------

const U0: [u8; 4] = [0x00, 0x00, 0x00, 0x00]; // u32 0
const U1: [u8; 4] = [0x01, 0x00, 0x00, 0x00]; // u32 1
const U2: [u8; 4] = [0x02, 0x00, 0x00, 0x00]; // u32 2
const F0_0: [u8; 4] = [0x00, 0x00, 0x00, 0x00]; // f32 0.0
const F0_5: [u8; 4] = [0x00, 0x00, 0x00, 0x3F]; // f32 0.5
const F1_0: [u8; 4] = [0x00, 0x00, 0x80, 0x3F]; // f32 1.0
const F2_0: [u8; 4] = [0x00, 0x00, 0x00, 0x40]; // f32 2.0
const F3_0: [u8; 4] = [0x00, 0x00, 0x40, 0x40]; // f32 3.0
const F4_0: [u8; 4] = [0x00, 0x00, 0x80, 0x40]; // f32 4.0
const IMG: [u8; 2] = [0xDE, 0xAD]; // stand-in image payload
const HI: [u8; 2] = [0x48, 0x69]; // "Hi" UTF-8

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[track_caller]
fn assert_layout(name: &str, actual: &[u8], expected: &[u8]) {
    assert_eq!(
        actual,
        expected,
        "\n{name} pack layout drifted — update src/lib/wasm/pack.ts to match!\n  actual:   {}\n  expected: {}",
        hex(actual),
        hex(expected),
    );
}

// ---------------------------------------------------------------------------
// One golden test per pack magic used in the crate.
// ---------------------------------------------------------------------------

#[test]
fn ufmg_merge_layout() {
    // magic | u32 count | per doc: bytes(u32 len + bytes)
    let pack = PackWriter::new(b"UFMG").u32(1).bytes(&IMG).finish();
    let expected = [b"UFMG".as_slice(), &U1, &U2, &IMG].concat();
    assert_layout("UFMG", &pack, &expected);
}

#[test]
fn ufst_stamp_layout() {
    // magic | u32 count | per stamp: u8 kind, u32 page, f32 x,y,w,h,opacity, bytes
    let pack = PackWriter::new(b"UFST")
        .u32(1)
        .u8(0)
        .u32(1)
        .f32(1.0)
        .f32(2.0)
        .f32(3.0)
        .f32(0.0)
        .f32(0.5)
        .bytes(&IMG)
        .finish();
    let expected = [
        b"UFST".as_slice(),
        &U1,
        &[0x00],
        &U1,
        &F1_0,
        &F2_0,
        &F3_0,
        &F0_0,
        &F0_5,
        &U2,
        &IMG,
    ]
    .concat();
    assert_layout("UFST", &pack, &expected);
}

#[test]
fn ufip_images_layout() {
    // magic | u32 count | per image: u8 kind, bytes
    let pack = PackWriter::new(b"UFIP").u32(1).u8(0).bytes(&IMG).finish();
    let expected = [b"UFIP".as_slice(), &U1, &[0x00], &U2, &IMG].concat();
    assert_layout("UFIP", &pack, &expected);
}

#[test]
fn ufpa_pdfa_layout() {
    // magic | u32 count | u16 year, u8 month/day/hour/min/sec | per page: f32 w,h, bytes
    let pack = PackWriter::new(b"UFPA")
        .u32(1)
        .u16(2026)
        .u8(1)
        .u8(2)
        .u8(3)
        .u8(4)
        .u8(5)
        .f32(1.0)
        .f32(2.0)
        .bytes(&IMG)
        .finish();
    let expected = [
        b"UFPA".as_slice(),
        &U1,
        &[0xEA, 0x07], // u16 2026
        &[0x01, 0x02, 0x03, 0x04, 0x05],
        &F1_0,
        &F2_0,
        &U2,
        &IMG,
    ]
    .concat();
    assert_layout("UFPA", &pack, &expected);
}

#[test]
fn uftp_office_text_layout() {
    // magic | u32 page_count | per page: u32 page_number, f32 w,h, u32 item_count,
    //                          per item: f32 x,y,w,h, str text
    let pack = PackWriter::new(b"UFTP")
        .u32(1)
        .u32(1)
        .f32(1.0)
        .f32(2.0)
        .u32(1)
        .f32(0.0)
        .f32(0.5)
        .f32(3.0)
        .f32(4.0)
        .str("Hi")
        .finish();
    let expected = [
        b"UFTP".as_slice(),
        &U1,
        &U1,
        &F1_0,
        &F2_0,
        &U1,
        &F0_0,
        &F0_5,
        &F3_0,
        &F4_0,
        &U2,
        &HI,
    ]
    .concat();
    assert_layout("UFTP", &pack, &expected);
}

#[test]
fn ufxp_epub_fixed_layout() {
    // magic | u32 page_count | per page: u32 page_number, f32 w,h, u8 kind, bytes
    let pack = PackWriter::new(b"UFXP")
        .u32(1)
        .u32(1)
        .f32(1.0)
        .f32(2.0)
        .u8(0)
        .bytes(&IMG)
        .finish();
    let expected = [
        b"UFXP".as_slice(),
        &U1,
        &U1,
        &F1_0,
        &F2_0,
        &[0x00],
        &U2,
        &IMG,
    ]
    .concat();
    assert_layout("UFXP", &pack, &expected);
}

#[test]
fn ufap_image_pages_layout() {
    // magic | u32 count | per page: f32 w,h, u8 kind, bytes
    let pack = PackWriter::new(b"UFAP")
        .u32(1)
        .f32(1.0)
        .f32(2.0)
        .u8(0)
        .bytes(&IMG)
        .finish();
    let expected = [b"UFAP".as_slice(), &U1, &F1_0, &F2_0, &[0x00], &U2, &IMG].concat();
    assert_layout("UFAP", &pack, &expected);
}

#[test]
fn uftl_text_layer_layout() {
    // magic | u32 page_count | per page: u32 page_index, u32 span_count,
    //                          per span: f32 x,y,font_size, str text
    let pack = PackWriter::new(b"UFTL")
        .u32(1)
        .u32(0)
        .u32(1)
        .f32(1.0)
        .f32(2.0)
        .f32(3.0)
        .str("Hi")
        .finish();
    let expected = [
        b"UFTL".as_slice(),
        &U1,
        &U0,
        &U1,
        &F1_0,
        &F2_0,
        &F3_0,
        &U2,
        &HI,
    ]
    .concat();
    assert_layout("UFTL", &pack, &expected);
}

#[test]
fn ufcb_crop_layout() {
    // magic | u32 count | per entry: u32 page_index, f32 x,y,w,h
    let pack = PackWriter::new(b"UFCB")
        .u32(1)
        .u32(0)
        .f32(1.0)
        .f32(2.0)
        .f32(3.0)
        .f32(4.0)
        .finish();
    let expected = [b"UFCB".as_slice(), &U1, &U0, &F1_0, &F2_0, &F3_0, &F4_0].concat();
    assert_layout("UFCB", &pack, &expected);
}

#[test]
fn versioned_header_is_magic_then_version_byte() {
    // The (not-yet-adopted) versioned layout: magic immediately followed by the
    // one-byte PACK_VERSION. Pins it so the JS side can mirror it exactly later.
    let pack = PackWriter::new(b"UFXX")
        .write_version(PACK_VERSION)
        .finish();
    let expected = [b"UFXX".as_slice(), &[PACK_VERSION]].concat();
    assert_layout("versioned-header", &pack, &expected);
    assert_eq!(
        PACK_VERSION, 1,
        "PACK_VERSION changed — coordinate with pack.ts"
    );
}

// ---------------------------------------------------------------------------
// Property test: every primitive survives a PackWriter -> PackReader round trip.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn writer_reader_round_trips_all_primitives(
        a in any::<u8>(),
        b in any::<u16>(),
        c in any::<u32>(),
        d in any::<f32>(),
        bytes in proptest::collection::vec(any::<u8>(), 0..128),
        s in ".*",
    ) {
        let pack = PackWriter::new(b"PROP")
            .u8(a)
            .u16(b)
            .u32(c)
            .f32(d)
            .bytes(&bytes)
            .str(&s)
            .finish();

        let mut r = PackReader::new(&pack);
        r.expect_magic(b"PROP").unwrap();
        prop_assert_eq!(r.read_u8().unwrap(), a);
        prop_assert_eq!(r.read_u16().unwrap(), b);
        prop_assert_eq!(r.read_u32().unwrap(), c);
        // Compare bit patterns so NaN payloads round-trip exactly.
        prop_assert_eq!(r.read_f32().unwrap().to_bits(), d.to_bits());
        prop_assert_eq!(r.read_bytes().unwrap(), bytes.as_slice());
        prop_assert_eq!(r.read_str().unwrap(), s.as_str());
        r.expect_done().unwrap();
    }

    #[test]
    fn versioned_round_trips(version in any::<u8>(), value in any::<u32>()) {
        let pack = PackWriter::new(b"PROP")
            .write_version(version)
            .u32(value)
            .finish();
        let mut r = PackReader::new(&pack);
        r.expect_magic_versioned(b"PROP", version).unwrap();
        prop_assert_eq!(r.read_u32().unwrap(), value);
        r.expect_done().unwrap();
    }
}
