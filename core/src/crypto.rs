//! PDF Standard Security: encryption, decryption and detection.
//!
//! This module owns Unfleece's password tooling (the *Protect* and *Unlock*
//! flows) so that no JavaScript crypto library is needed at runtime. It speaks
//! the Standard Security handler of ISO 32000 (§7.6.3 / PDF 32000-1) directly:
//!
//! - [`is_encrypted_native`] — peek the trailer `/Encrypt` without decrypting.
//! - [`unlock_native`]       — authenticate a *user* or *owner* password and
//!   emit an unencrypted copy (RC4 V1/V2 R2–R3, AESV2 V4 R4, AESV3 V5 R6).
//! - [`protect_native`]      — encrypt with AESV2 V4 R4 (matching pdf-lib's
//!   default), mapping the three Unfleece permission toggles onto the `/P` bits.
//!
//! ### Cryptographic notes / limitations
//! The Standard Security handler is built on MD5 and RC4 for revisions 2–4
//! (PDF 32000-1 §7.6.3); these primitives are weak by modern standards but are
//! mandated by the spec for interoperability and are unavoidable when reading
//! existing documents. Revision 6 (AESV3, PDF 2.0) uses SHA-2 and AES-256.
//! Password verification computes the full key derivation with **no early
//! exit**, so it is not a useful timing oracle. Randomness (the file `/ID`,
//! per-object AES IVs and the R6 file key) comes from [`getrandom`], which is
//! wasm-safe via its `js` feature.

use lopdf::encryption::get_encryption_key;
use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};
use serde::Deserialize;

use aes::cipher::block_padding::{NoPadding, Pkcs7};
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecryptMut, BlockEncrypt, BlockEncryptMut, KeyInit, KeyIvInit};
use md5::Md5;
use sha2::{Digest, Sha256, Sha384, Sha512};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

/// The 32-byte password padding string (PDF 32000-1, Algorithm 2, step a).
const PAD: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

// ---------------------------------------------------------------------------
// Permission (`/P`) bit layout. Bits are 1-indexed in the PDF spec; here the
// constants hold the raw little-endian masks (bit n -> 1 << (n - 1)).
// ---------------------------------------------------------------------------

/// Bits 3 and 12: print (low-res) + print high-resolution.
const P_PRINT: u32 = (1 << 2) | (1 << 11);
/// Bits 5 and 10: copy/extract + extract for accessibility.
const P_COPY: u32 = (1 << 4) | (1 << 9);
/// Bits 4, 6, 9 and 11: modify + annotate + fill forms + assemble.
const P_MODIFY: u32 = (1 << 3) | (1 << 5) | (1 << 8) | (1 << 10);
/// Bits 1 and 2 are reserved and must be 0.
const P_RESERVED_LOW: u32 = 0b11;

// ===========================================================================
// Public API
// ===========================================================================

/// Returns `true` when the document's trailer carries an `/Encrypt` dictionary.
///
/// Errors only when the bytes are not a parseable PDF (so the caller can show
/// the same "not a PDF" message it would for any other operation).
pub fn is_encrypted_native(data: &[u8]) -> Result<bool, String> {
    let doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    Ok(doc.is_encrypted())
}

/// Decrypt `data` using the supplied *user* or *owner* password and return an
/// unencrypted copy (the `/Encrypt` dictionary is removed).
///
/// Mirrors `unlockPdf` in `security.ts`: the password is trimmed, an empty
/// password is rejected up front, and an unencrypted input is returned
/// unchanged.
pub fn unlock_native(data: &[u8], password: &str) -> Result<Vec<u8>, String> {
    let pw = password.trim();
    if pw.is_empty() {
        return Err("Enter the password for this PDF".to_string());
    }

    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    if !doc.is_encrypted() {
        // An unencrypted PDF needs no work; hand the original bytes back.
        return Ok(data.to_vec());
    }

    let (_v, r) = read_v_r(&doc)?;
    let is_aes = encrypt_dict(&doc).map(is_aesv2).unwrap_or(false);

    let key = if r >= 5 {
        authenticate_v5(&doc, pw.as_bytes())?
    } else {
        authenticate_legacy(&doc, pw.as_bytes())?
    };

    decrypt_document(&mut doc, &key, is_aes, r >= 5)?;
    doc.trailer.remove(b"Encrypt");
    crate::util::save_compact(doc).map_err(|e| e.to_string())
}

/// JSON options for [`protect_native`]; matches `ProtectOptions` in
/// `security.ts` (camelCase keys).
#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ProtectOpts {
    user_password: String,
    owner_password: Option<String>,
    allow_printing: bool,
    allow_copying: bool,
    allow_modifying: bool,
}

impl Default for ProtectOpts {
    fn default() -> Self {
        Self {
            user_password: String::new(),
            owner_password: None,
            // Printing is allowed by default; copying and modifying are not.
            allow_printing: true,
            allow_copying: false,
            allow_modifying: false,
        }
    }
}

/// The three permission toggles Unfleece exposes, resolved into `/P` masks.
#[derive(Clone, Copy)]
struct Perms {
    printing: bool,
    copying: bool,
    modifying: bool,
}

/// Encrypt `data` with AESV2 (V4 / R4) Standard Security.
///
/// `opts_json` is a [`ProtectOpts`]. The user password is required (trimmed);
/// an empty owner password falls back to the user password, exactly like
/// `protectPdf` in `security.ts`.
pub fn protect_native(data: &[u8], opts_json: &str) -> Result<Vec<u8>, String> {
    let opts: ProtectOpts =
        serde_json::from_str(opts_json).map_err(|e| format!("Invalid options: {e}"))?;

    let user = opts.user_password.trim().to_string();
    if user.is_empty() {
        return Err("Enter a password to protect this PDF".to_string());
    }
    let owner = match opts.owner_password {
        Some(o) if !o.trim().is_empty() => o.trim().to_string(),
        _ => user.clone(),
    };

    let perms = Perms {
        printing: opts.allow_printing,
        copying: opts.allow_copying,
        modifying: opts.allow_modifying,
    };

    let mut id0 = [0u8; 16];
    let mut id1 = [0u8; 16];
    let mut file_key = [0u8; 32];
    fill_random(&mut id0)?;
    fill_random(&mut id1)?;
    fill_random(&mut file_key)?;

    encrypt_doc(
        data,
        user.as_bytes(),
        owner.as_bytes(),
        perms,
        Scheme::AesV2R4,
        id0,
        id1,
        file_key,
    )
}

// ===========================================================================
// Permission helpers
// ===========================================================================

/// Build the 32-bit signed `/P` value from the permission toggles. Bits 1–2
/// are cleared; every bit outside the three permission groups defaults to 1.
fn compute_p(perms: Perms) -> i32 {
    let mut p: u32 = u32::MAX;
    p &= !P_RESERVED_LOW;
    p &= !(P_PRINT | P_COPY | P_MODIFY);
    if perms.printing {
        p |= P_PRINT;
    }
    if perms.copying {
        p |= P_COPY;
    }
    if perms.modifying {
        p |= P_MODIFY;
    }
    p as i32
}

// ===========================================================================
// Encryption (AESV2 R4 for `protect`, plus RC4 R2 / AESV3 R6 for round-trips)
// ===========================================================================

/// The output schemes this module can *produce*. `protect_native` always uses
/// [`Scheme::AesV2R4`]; the other two exist so the decrypt path can be
/// exercised end-to-end by the test suite (hence `dead_code` off-test).
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(not(test), allow(dead_code))]
enum Scheme {
    /// RC4, V1 / R2, 40-bit key (legacy).
    Rc4R2,
    /// AESV2, V4 / R4, 128-bit key (pdf-lib default; what `protect` emits).
    AesV2R4,
    /// AESV3, V5 / R6, 256-bit key (PDF 2.0).
    AesV3R6,
}

/// Encrypt `data` under `scheme`. `id0`/`id1` seed the file `/ID` (and the key
/// derivation for the legacy schemes); `file_key` is the random 256-bit key
/// used only by AESV3. Splitting the randomness out as parameters keeps the
/// function deterministic for tests.
#[allow(clippy::too_many_arguments)]
fn encrypt_doc(
    data: &[u8],
    user: &[u8],
    owner: &[u8],
    perms: Perms,
    scheme: Scheme,
    id0: [u8; 16],
    id1: [u8; 16],
    file_key_v5: [u8; 32],
) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(data).map_err(|e| e.to_string())?;
    if doc.is_encrypted() {
        return Err("This PDF is already password-protected. Unlock it first.".to_string());
    }

    // Encryption must run *after* compression and once object numbers are
    // final, because per-object keys are derived from the object id and the
    // stored bytes are encrypted in place. Hence the manual save below rather
    // than `util::save_compact` (which would compress *after* we encrypt).
    doc.prune_objects();
    doc.compress();
    doc.renumber_objects();

    let (ida, idb) = resolve_file_id(&doc, &id0, &id1);
    let p = compute_p(perms);

    let (enc_dict, file_key, is_aes, aesv3) = match scheme {
        Scheme::Rc4R2 => {
            let o = compute_o(user, owner, 2, 5);
            let fk = compute_file_key(user, &o, p, &ida, 2, 5);
            let u = compute_u(&fk, &ida, 2);
            (
                legacy_encrypt_dict(1, 2, 40, &o, &u, p, None),
                fk,
                false,
                false,
            )
        }
        Scheme::AesV2R4 => {
            let o = compute_o(user, owner, 4, 16);
            let fk = compute_file_key(user, &o, p, &ida, 4, 16);
            let u = compute_u(&fk, &ida, 4);
            (
                legacy_encrypt_dict(4, 4, 128, &o, &u, p, Some(b"AESV2")),
                fk,
                true,
                false,
            )
        }
        Scheme::AesV3R6 => {
            let dict = v5_encrypt_dict(user, owner, p, &file_key_v5)?;
            (dict, file_key_v5.to_vec(), false, true)
        }
    };

    let enc_id = doc.add_object(Object::Dictionary(enc_dict));

    let ids: Vec<ObjectId> = doc.objects.keys().copied().collect();
    for id in ids {
        if id == enc_id {
            continue;
        }
        let obj = doc.objects.get_mut(&id).expect("id from keys()");
        if should_skip(obj, true) {
            continue;
        }
        crypt_tree(obj, id, &file_key, is_aes, true, aesv3)?;
    }

    doc.trailer.set("Encrypt", Object::Reference(enc_id));
    doc.trailer.set(
        "ID",
        Object::Array(vec![
            Object::String(ida, StringFormat::Hexadecimal),
            Object::String(idb, StringFormat::Hexadecimal),
        ]),
    );

    let mut buf = Vec::new();
    doc.save_to(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

/// Reuse an existing first `/ID` element if present (it participates in legacy
/// key derivation and must stay stable); otherwise adopt the random pair.
fn resolve_file_id(doc: &Document, id0: &[u8; 16], id1: &[u8; 16]) -> (Vec<u8>, Vec<u8>) {
    if let Ok(arr) = doc.trailer.get(b"ID").and_then(Object::as_array) {
        if let Some(Object::String(first, _)) = arr.first() {
            if !first.is_empty() {
                let second = match arr.get(1) {
                    Some(Object::String(s, _)) if !s.is_empty() => s.clone(),
                    _ => id1.to_vec(),
                };
                return (first.clone(), second);
            }
        }
    }
    (id0.to_vec(), id1.to_vec())
}

/// Build the `/Encrypt` dictionary for an RC4 or AESV2 scheme.
fn legacy_encrypt_dict(
    v: i64,
    r: i64,
    length: i64,
    o: &[u8],
    u: &[u8],
    p: i32,
    cfm: Option<&[u8]>,
) -> Dictionary {
    let mut d = Dictionary::new();
    d.set("Filter", Object::Name(b"Standard".to_vec()));
    d.set("V", Object::Integer(v));
    d.set("R", Object::Integer(r));
    d.set("Length", Object::Integer(length));
    if let Some(cfm) = cfm {
        let mut std = Dictionary::new();
        std.set("CFM", Object::Name(cfm.to_vec()));
        std.set("AuthEvent", Object::Name(b"DocOpen".to_vec()));
        std.set("Length", Object::Integer(length / 8));
        let mut cf = Dictionary::new();
        cf.set("StdCF", Object::Dictionary(std));
        d.set("CF", Object::Dictionary(cf));
        d.set("StmF", Object::Name(b"StdCF".to_vec()));
        d.set("StrF", Object::Name(b"StdCF".to_vec()));
    }
    d.set("O", Object::String(o.to_vec(), StringFormat::Hexadecimal));
    d.set("U", Object::String(u.to_vec(), StringFormat::Hexadecimal));
    d.set("P", Object::Integer(p as i64));
    d
}

/// Build the `/Encrypt` dictionary for AESV3 (V5 / R6).
fn v5_encrypt_dict(
    user: &[u8],
    owner: &[u8],
    p: i32,
    file_key: &[u8; 32],
) -> Result<Dictionary, String> {
    let mut salts = [0u8; 32];
    fill_random(&mut salts)?;
    let (uvs, uks) = (&salts[0..8], &salts[8..16]);
    let (ovs, oks) = (&salts[16..24], &salts[24..32]);

    // /U = hash(pw + validation salt) || validation salt || key salt   (Alg. 8)
    let mut u = hash_r6(user, uvs, &[]);
    u.extend_from_slice(uvs);
    u.extend_from_slice(uks);
    let ik_u = hash_r6(user, uks, &[]);
    let ue = aes256_noiv_nopad(&ik_u, file_key, true);

    // /O is bound to /U (the 48-byte value just built).                 (Alg. 9)
    let mut o = hash_r6(owner, ovs, &u);
    o.extend_from_slice(ovs);
    o.extend_from_slice(oks);
    let ik_o = hash_r6(owner, oks, &u);
    let oe = aes256_noiv_nopad(&ik_o, file_key, true);

    let perms_block = perms_block(p, true)?;
    let perms_enc = aes256_ecb_encrypt(file_key, &perms_block);

    let mut std = Dictionary::new();
    std.set("CFM", Object::Name(b"AESV3".to_vec()));
    std.set("AuthEvent", Object::Name(b"DocOpen".to_vec()));
    std.set("Length", Object::Integer(32));
    let mut cf = Dictionary::new();
    cf.set("StdCF", Object::Dictionary(std));

    let mut d = Dictionary::new();
    d.set("Filter", Object::Name(b"Standard".to_vec()));
    d.set("V", Object::Integer(5));
    d.set("R", Object::Integer(6));
    d.set("Length", Object::Integer(256));
    d.set("CF", Object::Dictionary(cf));
    d.set("StmF", Object::Name(b"StdCF".to_vec()));
    d.set("StrF", Object::Name(b"StdCF".to_vec()));
    d.set("O", Object::String(o, StringFormat::Hexadecimal));
    d.set("U", Object::String(u, StringFormat::Hexadecimal));
    d.set("OE", Object::String(oe, StringFormat::Hexadecimal));
    d.set("UE", Object::String(ue, StringFormat::Hexadecimal));
    d.set(
        "Perms",
        Object::String(perms_enc, StringFormat::Hexadecimal),
    );
    d.set("P", Object::Integer(p as i64));
    d.set("EncryptMetadata", Object::Boolean(true));
    Ok(d)
}

/// The 16-byte `/Perms` plaintext block (PDF 32000-2, Algorithm 8 step f).
fn perms_block(p: i32, encrypt_metadata: bool) -> Result<[u8; 16], String> {
    let mut b = [0u8; 16];
    b[0..4].copy_from_slice(&(p as u32).to_le_bytes());
    b[4..8].copy_from_slice(&[0xFF; 4]);
    b[8] = if encrypt_metadata { b'T' } else { b'F' };
    b[9..12].copy_from_slice(b"adb");
    fill_random(&mut b[12..16])?;
    Ok(b)
}

// ===========================================================================
// Authentication
// ===========================================================================

/// Derive the file key for an RC4/AESV2 document, trying the password as a
/// *user* password first and then as an *owner* password (Algorithms 6 & 7).
fn authenticate_legacy(doc: &Document, pw: &[u8]) -> Result<Vec<u8>, String> {
    if let Ok(key) = get_encryption_key(doc, pw, true) {
        return Ok(key);
    }

    let (o, r, key_len) = {
        let enc = encrypt_dict(doc)?;
        let o = enc
            .get(b"O")
            .ok()
            .and_then(|x| x.as_str().ok())
            .ok_or_else(|| "Invalid encryption dictionary".to_string())?
            .to_vec();
        let r = enc
            .get(b"R")
            .ok()
            .and_then(|x| x.as_i64().ok())
            .ok_or_else(|| "Invalid encryption dictionary".to_string())?;
        let key_len = enc
            .get(b"Length")
            .ok()
            .and_then(|x| x.as_i64().ok())
            .unwrap_or(40) as usize
            / 8;
        (o, r, key_len)
    };

    let recovered = recover_user_from_owner(pw, &o, r, key_len);
    if let Ok(key) = get_encryption_key(doc, &recovered, true) {
        return Ok(key);
    }

    Err("Incorrect password".to_string())
}

/// Algorithm 7: recover the padded user password from `/O` given the owner
/// password, so it can be fed back through Algorithm 2.
fn recover_user_from_owner(owner_pw: &[u8], o: &[u8], r: i64, key_len: usize) -> Vec<u8> {
    let key_len = key_len.clamp(1, 16);
    let mut h = md5(&pad_password(owner_pw));
    if r >= 3 {
        for _ in 0..50 {
            h = md5(&h[..key_len]);
        }
    }
    let rc4_key = &h[..key_len];

    if r == 2 {
        rc4(rc4_key, o)
    } else {
        let mut buf = o.to_vec();
        for i in (0..=19u8).rev() {
            let k: Vec<u8> = rc4_key.iter().map(|b| b ^ i).collect();
            buf = rc4(&k, &buf);
        }
        buf
    }
}

/// Authenticate an AESV3 (R6) document and return the 256-bit file key.
fn authenticate_v5(doc: &Document, pw: &[u8]) -> Result<Vec<u8>, String> {
    let enc = encrypt_dict(doc)?;
    let get = |k: &[u8]| -> Result<Vec<u8>, String> {
        enc.get(k)
            .ok()
            .and_then(|x| x.as_str().ok())
            .map(<[u8]>::to_vec)
            .ok_or_else(|| "Invalid encryption dictionary".to_string())
    };
    let u = get(b"U")?;
    let ue = get(b"UE")?;
    let o = get(b"O")?;
    let oe = get(b"OE")?;
    if u.len() < 48 || o.len() < 48 || ue.len() < 32 || oe.len() < 32 {
        return Err("Invalid encryption dictionary".to_string());
    }

    // Try the user password (Algorithm 2.A / 8).
    if hash_r6(pw, &u[32..40], &[]) == u[0..32] {
        let ik = hash_r6(pw, &u[40..48], &[]);
        return Ok(aes256_noiv_nopad(&ik, &ue[..32], false));
    }
    // Then the owner password (bound to /U).
    if hash_r6(pw, &o[32..40], &u[0..48]) == o[0..32] {
        let ik = hash_r6(pw, &o[40..48], &u[0..48]);
        return Ok(aes256_noiv_nopad(&ik, &oe[..32], false));
    }

    Err("Incorrect password".to_string())
}

// ===========================================================================
// Decryption
// ===========================================================================

/// Replace every encrypted string/stream in `doc` with its plaintext, in place.
fn decrypt_document(
    doc: &mut Document,
    key: &[u8],
    is_aes: bool,
    aesv3: bool,
) -> Result<(), String> {
    let enc_id = doc
        .trailer
        .get(b"Encrypt")
        .and_then(Object::as_reference)
        .map_err(|_| "Invalid encryption dictionary".to_string())?;

    let encrypt_metadata = doc
        .get_object(enc_id)
        .ok()
        .and_then(|o| o.as_dict().ok())
        .and_then(|d| d.get(b"EncryptMetadata").ok())
        .and_then(|o| o.as_bool().ok())
        .unwrap_or(true);

    let ids: Vec<ObjectId> = doc.objects.keys().copied().collect();
    for id in ids {
        if id == enc_id {
            continue;
        }
        let obj = doc.objects.get_mut(&id).expect("id from keys()");
        if should_skip(obj, encrypt_metadata) {
            continue;
        }
        crypt_tree(obj, id, key, is_aes, false, aesv3)?;
    }
    Ok(())
}

/// Cross-reference streams are never encrypted, and `/Metadata` streams are
/// exempt when `EncryptMetadata` is false.
fn should_skip(obj: &Object, encrypt_metadata: bool) -> bool {
    if let Object::Stream(s) = obj {
        let ty = s.dict.get(b"Type").ok().and_then(|t| t.as_name().ok());
        if ty == Some(b"XRef") {
            return true;
        }
        if !encrypt_metadata && ty == Some(b"Metadata") {
            return true;
        }
    }
    false
}

/// Recursively encrypt or decrypt every `String`/`Stream` reachable inside the
/// indirect object `id`, keyed by that object's id.
fn crypt_tree(
    obj: &mut Object,
    id: ObjectId,
    key: &[u8],
    is_aes: bool,
    encrypt: bool,
    aesv3: bool,
) -> Result<(), String> {
    match obj {
        Object::String(content, fmt) => {
            *content = crypt_bytes(content, id, key, is_aes, encrypt, aesv3)?;
            if encrypt {
                // Force hex so arbitrary ciphertext bytes serialize safely.
                *fmt = StringFormat::Hexadecimal;
            }
        }
        Object::Stream(stream) => {
            for (_, v) in stream.dict.iter_mut() {
                crypt_tree(v, id, key, is_aes, encrypt, aesv3)?;
            }
            let new = crypt_bytes(&stream.content, id, key, is_aes, encrypt, aesv3)?;
            stream.set_content(new);
        }
        Object::Array(arr) => {
            for v in arr.iter_mut() {
                crypt_tree(v, id, key, is_aes, encrypt, aesv3)?;
            }
        }
        Object::Dictionary(dict) => {
            for (_, v) in dict.iter_mut() {
                crypt_tree(v, id, key, is_aes, encrypt, aesv3)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Encrypt/decrypt one string-or-stream payload with the right per-object key.
fn crypt_bytes(
    content: &[u8],
    id: ObjectId,
    key: &[u8],
    is_aes: bool,
    encrypt: bool,
    aesv3: bool,
) -> Result<Vec<u8>, String> {
    if content.is_empty() {
        return Ok(Vec::new());
    }

    if aesv3 {
        // AESV3 uses the file key directly (no per-object salt).
        return if encrypt {
            aes_cbc_encrypt(key, content)
        } else {
            Ok(aes_cbc_decrypt(key, content).unwrap_or_else(|| content.to_vec()))
        };
    }

    let okey = per_object_key(key, id, is_aes);
    if is_aes {
        if encrypt {
            aes_cbc_encrypt(&okey, content)
        } else {
            Ok(aes_cbc_decrypt(&okey, content).unwrap_or_else(|| content.to_vec()))
        }
    } else {
        // RC4 is symmetric.
        Ok(rc4(&okey, content))
    }
}

/// Algorithm 1: per-object key = MD5(file key || obj# (3 LE) || gen (2 LE)
/// [|| "sAlT"]) truncated to `min(len + 5, 16)`.
fn per_object_key(file_key: &[u8], id: ObjectId, aes: bool) -> Vec<u8> {
    let mut buf = Vec::with_capacity(file_key.len() + 9);
    buf.extend_from_slice(file_key);
    buf.extend_from_slice(&id.0.to_le_bytes()[..3]);
    buf.extend_from_slice(&id.1.to_le_bytes()[..2]);
    if aes {
        buf.extend_from_slice(&[0x73, 0x41, 0x6C, 0x54]); // "sAlT"
    }
    let n = (file_key.len() + 5).min(16);
    md5(&buf)[..n].to_vec()
}

// ===========================================================================
// Standard Security key/value derivation (Algorithms 2–5)
// ===========================================================================

/// Algorithm 3: compute `/O` from the owner and user passwords.
fn compute_o(user: &[u8], owner: &[u8], r: i64, key_len: usize) -> Vec<u8> {
    let mut h = md5(&pad_password(owner));
    if r >= 3 {
        for _ in 0..50 {
            h = md5(&h[..key_len]);
        }
    }
    let rc4_key = &h[..key_len];

    let mut o = rc4(rc4_key, &pad_password(user));
    if r >= 3 {
        for i in 1..=19u8 {
            let k: Vec<u8> = rc4_key.iter().map(|b| b ^ i).collect();
            o = rc4(&k, &o);
        }
    }
    o
}

/// Algorithm 2: derive the file encryption key from the user password.
/// (`EncryptMetadata` is always true for documents we produce.)
fn compute_file_key(user: &[u8], o: &[u8], p: i32, id0: &[u8], r: i64, key_len: usize) -> Vec<u8> {
    let mut buf = pad_password(user).to_vec();
    buf.extend_from_slice(o);
    buf.extend_from_slice(&(p as u32).to_le_bytes());
    buf.extend_from_slice(id0);

    let mut h = md5(&buf);
    if r >= 3 {
        for _ in 0..50 {
            h = md5(&h[..key_len]);
        }
    }
    h[..key_len].to_vec()
}

/// Algorithms 4 (R2) and 5 (R3+): compute the 32-byte `/U` verifier.
fn compute_u(file_key: &[u8], id0: &[u8], r: i64) -> Vec<u8> {
    if r == 2 {
        return rc4(file_key, &PAD);
    }

    let mut ctx = Md5::new();
    ctx.update(PAD);
    ctx.update(id0);
    let hash = ctx.finalize();

    let mut enc = rc4(file_key, &hash);
    for i in 1..=19u8 {
        let k: Vec<u8> = file_key.iter().map(|b| b ^ i).collect();
        enc = rc4(&k, &enc);
    }
    enc.extend_from_slice(&[0u8; 16]); // pad to 32 (the tail is arbitrary)
    enc
}

/// Pad/truncate a password to exactly 32 bytes (Algorithm 2, step a).
fn pad_password(pw: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let n = pw.len().min(32);
    out[..n].copy_from_slice(&pw[..n]);
    out[n..].copy_from_slice(&PAD[..32 - n]);
    out
}

// ===========================================================================
// R6 (AESV3) hardened hash — PDF 32000-2, Algorithm 2.B
// ===========================================================================

/// The Algorithm 2.B hash used by revision 6 (`udata` is the 48-byte `/U` when
/// hashing an owner password, otherwise empty).
fn hash_r6(password: &[u8], salt: &[u8], udata: &[u8]) -> Vec<u8> {
    let mut input = Vec::with_capacity(password.len() + salt.len() + udata.len());
    input.extend_from_slice(password);
    input.extend_from_slice(salt);
    input.extend_from_slice(udata);
    let mut k = Sha256::digest(&input).to_vec();

    let mut round = 0usize;
    loop {
        // K1 = (password || K || udata) repeated 64 times.
        let mut block = Vec::with_capacity(password.len() + k.len() + udata.len());
        block.extend_from_slice(password);
        block.extend_from_slice(&k);
        block.extend_from_slice(udata);
        let mut k1 = Vec::with_capacity(block.len() * 64);
        for _ in 0..64 {
            k1.extend_from_slice(&block);
        }

        // E = AES-128-CBC(K[0..16], iv=K[16..32], K1), no padding.
        let e = aes128_cbc_nopad_encrypt(&k[0..16], &k[16..32], &k1);

        let modulo = e[0..16].iter().map(|&b| b as u32).sum::<u32>() % 3;
        k = match modulo {
            0 => Sha256::digest(&e).to_vec(),
            1 => Sha384::digest(&e).to_vec(),
            _ => Sha512::digest(&e).to_vec(),
        };

        round += 1;
        if round >= 64 && (*e.last().unwrap() as usize) <= round - 32 {
            break;
        }
    }

    k.truncate(32);
    k
}

// ===========================================================================
// Primitives
// ===========================================================================

/// MD5 digest as a fixed 16-byte array.
fn md5(data: &[u8]) -> [u8; 16] {
    let d = Md5::digest(data);
    let mut out = [0u8; 16];
    out.copy_from_slice(&d);
    out
}

/// RC4 keystream cipher (encryption and decryption are the same operation).
/// Implemented inline to accept runtime-variable key lengths (the `rc4` crate
/// fixes the key size at the type level, which does not fit PDF's 5–16 byte
/// per-object keys).
fn rc4(key: &[u8], data: &[u8]) -> Vec<u8> {
    debug_assert!(!key.is_empty());
    let mut s: [u8; 256] = core::array::from_fn(|i| i as u8);
    let mut j: u8 = 0;
    for i in 0..256 {
        j = j.wrapping_add(s[i]).wrapping_add(key[i % key.len()]);
        s.swap(i, j as usize);
    }

    let mut out = Vec::with_capacity(data.len());
    let (mut a, mut b) = (0u8, 0u8);
    for &byte in data {
        a = a.wrapping_add(1);
        b = b.wrapping_add(s[a as usize]);
        s.swap(a as usize, b as usize);
        let k = s[(s[a as usize].wrapping_add(s[b as usize])) as usize];
        out.push(byte ^ k);
    }
    out
}

/// AES-CBC encrypt with PKCS#7 padding and a fresh random IV (prepended).
/// `key` length selects AES-128 (16) or AES-256 (32).
fn aes_cbc_encrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    let mut iv = [0u8; 16];
    fill_random(&mut iv)?;
    let ct = match key.len() {
        16 => Aes128CbcEnc::new_from_slices(key, &iv)
            .map_err(|e| e.to_string())?
            .encrypt_padded_vec_mut::<Pkcs7>(data),
        32 => Aes256CbcEnc::new_from_slices(key, &iv)
            .map_err(|e| e.to_string())?
            .encrypt_padded_vec_mut::<Pkcs7>(data),
        _ => return Err("Invalid AES key length".to_string()),
    };
    let mut out = iv.to_vec();
    out.extend_from_slice(&ct);
    Ok(out)
}

/// AES-CBC decrypt; expects `IV(16) || ciphertext`. Returns `None` when the
/// content is too short or the PKCS#7 padding is invalid (so non-encrypted
/// payloads are left untouched).
fn aes_cbc_decrypt(key: &[u8], content: &[u8]) -> Option<Vec<u8>> {
    if content.len() < 32 || !content.len().is_multiple_of(16) {
        return None;
    }
    let (iv, ct) = content.split_at(16);
    match key.len() {
        16 => Aes128CbcDec::new_from_slices(key, iv)
            .ok()?
            .decrypt_padded_vec_mut::<Pkcs7>(ct)
            .ok(),
        32 => Aes256CbcDec::new_from_slices(key, iv)
            .ok()?
            .decrypt_padded_vec_mut::<Pkcs7>(ct)
            .ok(),
        _ => None,
    }
}

/// AES-128-CBC, no padding, explicit IV — used only by [`hash_r6`].
fn aes128_cbc_nopad_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
    Aes128CbcEnc::new_from_slices(key, iv)
        .expect("16-byte key/iv")
        .encrypt_padded_vec_mut::<NoPadding>(data)
}

/// AES-256-CBC with a zero IV and no padding (the `/UE` and `/OE` wrap).
fn aes256_noiv_nopad(key: &[u8], data: &[u8], encrypt: bool) -> Vec<u8> {
    let iv = [0u8; 16];
    if encrypt {
        Aes256CbcEnc::new_from_slices(key, &iv)
            .expect("32-byte key")
            .encrypt_padded_vec_mut::<NoPadding>(data)
    } else {
        Aes256CbcDec::new_from_slices(key, &iv)
            .expect("32-byte key")
            .decrypt_padded_vec_mut::<NoPadding>(data)
            .expect("aligned ciphertext")
            .to_vec()
    }
}

/// AES-256-ECB single-block encrypt (the `/Perms` wrap).
fn aes256_ecb_encrypt(key: &[u8], block: &[u8; 16]) -> Vec<u8> {
    let cipher = aes::Aes256::new_from_slice(key).expect("32-byte key");
    let mut b = GenericArray::clone_from_slice(block);
    cipher.encrypt_block(&mut b);
    b.to_vec()
}

/// Fill `buf` with cryptographically secure random bytes (wasm-safe).
fn fill_random(buf: &mut [u8]) -> Result<(), String> {
    getrandom::getrandom(buf).map_err(|e| format!("Random source unavailable: {e}"))
}

// ===========================================================================
// Small dictionary readers
// ===========================================================================

fn encrypt_dict(doc: &Document) -> Result<&Dictionary, String> {
    doc.get_encrypted()
        .map_err(|_| "Invalid encryption dictionary".to_string())
}

fn read_v_r(doc: &Document) -> Result<(i64, i64), String> {
    let enc = encrypt_dict(doc)?;
    let v = enc
        .get(b"V")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(0);
    let r = enc
        .get(b"R")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .ok_or_else(|| "Invalid encryption dictionary".to_string())?;
    Ok((v, r))
}

fn is_aesv2(enc: &Dictionary) -> bool {
    cfm_name(enc).as_deref() == Some(b"AESV2")
}

fn cfm_name(enc: &Dictionary) -> Option<Vec<u8>> {
    enc.get(b"CF")
        .ok()
        .and_then(|c| c.as_dict().ok())
        .and_then(|c| c.get(b"StdCF").ok())
        .and_then(|c| c.as_dict().ok())
        .and_then(|c| c.get(b"CFM").ok())
        .and_then(|o| o.as_name().ok())
        .map(<[u8]>::to_vec)
}

// ===========================================================================
// Tests
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fixtures::{page_content_text, sample, sample_with_text};
    use lopdf::Document;
    use proptest::prelude::*;
    use std::path::PathBuf;

    // --- helpers ----------------------------------------------------------

    fn fixture(name: &str) -> Vec<u8> {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests/fixtures");
        p.push(name);
        std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
    }

    fn opts(user: &str, owner: Option<&str>, print: bool, copy: bool, modify: bool) -> String {
        let owner = match owner {
            Some(o) => format!(r#","ownerPassword":"{o}""#),
            None => String::new(),
        };
        format!(
            r#"{{"userPassword":"{user}"{owner},"allowPrinting":{print},"allowCopying":{copy},"allowModifying":{modify}}}"#
        )
    }

    fn protect(data: &[u8], user: &str) -> Vec<u8> {
        protect_native(data, &opts(user, None, true, false, false)).unwrap()
    }

    fn p_bits(data: &[u8]) -> u32 {
        let doc = Document::load_mem(data).unwrap();
        doc.get_encrypted()
            .unwrap()
            .get(b"P")
            .unwrap()
            .as_i64()
            .unwrap() as u32
    }

    fn page_count(data: &[u8]) -> usize {
        Document::load_mem(data).unwrap().page_iter().count()
    }

    fn has(bits: u32, mask: u32) -> bool {
        bits & mask == mask
    }

    // --- is_encrypted -----------------------------------------------------

    #[test]
    fn is_encrypted_plain_is_false() {
        assert!(!is_encrypted_native(&sample(1)).unwrap());
    }

    #[test]
    fn is_encrypted_aesv2_fixture_is_true() {
        assert!(is_encrypted_native(&fixture("encrypted_user.pdf")).unwrap());
    }

    #[test]
    fn is_encrypted_malformed_is_err() {
        assert!(is_encrypted_native(b"%PDF-1.7 not really a pdf").is_err());
    }

    #[test]
    fn is_encrypted_empty_is_err() {
        assert!(is_encrypted_native(&[]).is_err());
    }

    #[test]
    fn is_encrypted_after_protect_is_true() {
        let out = protect(&sample(1), "pw");
        assert!(is_encrypted_native(&out).unwrap());
    }

    // --- unlock: fixtures -------------------------------------------------

    #[test]
    fn unlock_user_password_three_pages() {
        let out = unlock_native(&fixture("encrypted_user.pdf"), "secret123").unwrap();
        assert!(!is_encrypted_native(&out).unwrap());
        assert_eq!(page_count(&out), 3);
    }

    #[test]
    fn unlock_owner_owner_password() {
        let out = unlock_native(&fixture("encrypted_user_owner.pdf"), "owner1").unwrap();
        assert!(!is_encrypted_native(&out).unwrap());
        assert_eq!(page_count(&out), 3);
    }

    #[test]
    fn unlock_owner_user_password() {
        let out = unlock_native(&fixture("encrypted_user_owner.pdf"), "user1").unwrap();
        assert_eq!(page_count(&out), 3);
    }

    #[test]
    fn unlock_wrong_password_is_incorrect() {
        let err = unlock_native(&fixture("encrypted_user.pdf"), "nope").unwrap_err();
        assert_eq!(err, "Incorrect password");
    }

    #[test]
    fn unlock_empty_password_message() {
        let err = unlock_native(&fixture("encrypted_user.pdf"), "   ").unwrap_err();
        assert_eq!(err, "Enter the password for this PDF");
    }

    #[test]
    fn unlock_unencrypted_returns_unchanged() {
        let plain = sample(2);
        let out = unlock_native(&plain, "whatever").unwrap();
        assert_eq!(out, plain);
    }

    #[test]
    fn unlock_corrupt_is_err() {
        assert!(unlock_native(b"not a pdf at all", "pw").is_err());
    }

    #[test]
    fn unlock_fixture_content_decrypts_to_text() {
        // The fixture pages draw text via a (mangled) Helvetica; after unlocking
        // the content stream must decode to a `BT ... ET` text block.
        let out = unlock_native(&fixture("encrypted_user.pdf"), "secret123").unwrap();
        let txt = page_content_text(&out, 0);
        assert!(txt.contains("BT"), "expected a text block, got: {txt:?}");
    }

    // --- protect: validation & permissions --------------------------------

    #[test]
    fn protect_empty_user_password_is_err() {
        let err = protect_native(&sample(1), &opts("", None, true, false, false)).unwrap_err();
        assert_eq!(err, "Enter a password to protect this PDF");
    }

    #[test]
    fn protect_whitespace_user_password_is_err() {
        let err = protect_native(&sample(1), r#"{"userPassword":"   "}"#).unwrap_err();
        assert_eq!(err, "Enter a password to protect this PDF");
    }

    #[test]
    fn protect_distinct_owner_gives_o_ne_u() {
        let out =
            protect_native(&sample(1), &opts("user", Some("owner"), true, false, false)).unwrap();
        let doc = Document::load_mem(&out).unwrap();
        let enc = doc.get_encrypted().unwrap();
        let o = enc.get(b"O").unwrap().as_str().unwrap();
        let u = enc.get(b"U").unwrap().as_str().unwrap();
        assert_ne!(o, u);
    }

    #[test]
    fn protect_printing_true_sets_bits() {
        let out = protect_native(&sample(1), &opts("p", None, true, false, false)).unwrap();
        assert!(has(p_bits(&out), P_PRINT));
    }

    #[test]
    fn protect_printing_false_clears_bits() {
        let out = protect_native(&sample(1), &opts("p", None, false, false, false)).unwrap();
        assert_eq!(p_bits(&out) & P_PRINT, 0);
    }

    #[test]
    fn protect_copying_toggle() {
        let on = protect_native(&sample(1), &opts("p", None, false, true, false)).unwrap();
        let off = protect_native(&sample(1), &opts("p", None, false, false, false)).unwrap();
        assert!(has(p_bits(&on), P_COPY));
        assert_eq!(p_bits(&off) & P_COPY, 0);
    }

    #[test]
    fn protect_modifying_toggle() {
        let on = protect_native(&sample(1), &opts("p", None, false, false, true)).unwrap();
        let off = protect_native(&sample(1), &opts("p", None, false, false, false)).unwrap();
        assert!(has(p_bits(&on), P_MODIFY));
        assert_eq!(p_bits(&off) & P_MODIFY, 0);
    }

    #[test]
    fn protect_all_permissions_set() {
        let out = protect_native(&sample(1), &opts("p", None, true, true, true)).unwrap();
        let bits = p_bits(&out);
        assert!(has(bits, P_PRINT | P_COPY | P_MODIFY));
        // reserved low bits stay clear
        assert_eq!(bits & P_RESERVED_LOW, 0);
    }

    #[test]
    fn protect_no_permissions_reading_only() {
        let out = protect_native(&sample(1), &opts("p", None, false, false, false)).unwrap();
        let bits = p_bits(&out);
        assert_eq!(bits & (P_PRINT | P_COPY | P_MODIFY), 0);
        // a non-permission high bit (bit 13) remains set
        assert!(has(bits, 1 << 12));
    }

    #[test]
    fn protect_defaults_printing_only() {
        // Only the user password supplied: printing on, copy/modify off.
        let out = protect_native(&sample(1), r#"{"userPassword":"pw"}"#).unwrap();
        let bits = p_bits(&out);
        assert!(has(bits, P_PRINT));
        assert_eq!(bits & (P_COPY | P_MODIFY), 0);
    }

    #[test]
    fn protect_invalid_json_is_err() {
        assert!(protect_native(&sample(1), "{not json").is_err());
    }

    // --- protect: structure preserved -------------------------------------

    #[test]
    fn protect_three_pages_decrypts_to_three() {
        let out = protect(&sample(3), "pw");
        let back = unlock_native(&out, "pw").unwrap();
        assert_eq!(page_count(&back), 3);
    }

    #[test]
    fn protect_zero_page_pdf_succeeds() {
        let out = protect(&sample(0), "pw");
        assert!(is_encrypted_native(&out).unwrap());
        let back = unlock_native(&out, "pw").unwrap();
        assert_eq!(page_count(&back), 0);
    }

    #[test]
    fn protect_already_encrypted_is_err() {
        let out = protect(&sample(1), "pw");
        assert!(protect_native(&out, &opts("pw2", None, true, false, false)).is_err());
    }

    // --- round-trips ------------------------------------------------------

    #[test]
    fn roundtrip_p_is_deterministic() {
        let a = protect_native(&sample(1), &opts("u", Some("o"), true, true, false)).unwrap();
        let b = protect_native(&sample(1), &opts("u", Some("o"), true, true, false)).unwrap();
        assert_eq!(p_bits(&a), p_bits(&b));
    }

    #[test]
    fn roundtrip_text_matches() {
        let plain = sample_with_text(1, Some("Hello"));
        let enc = protect(&plain, "pw");
        let back = unlock_native(&enc, "pw").unwrap();
        assert!(page_content_text(&back, 0).contains("Hello"));
    }

    #[test]
    fn roundtrip_multipage_text_intact() {
        let plain = sample_with_text(3, Some("Page"));
        let enc = protect(&plain, "secret");
        let back = unlock_native(&enc, "secret").unwrap();
        assert_eq!(page_count(&back), 3);
        for i in 0..3 {
            assert!(page_content_text(&back, i).contains("Page"));
        }
    }

    #[test]
    fn roundtrip_resources_and_mediabox_preserved() {
        let plain = sample_with_text(1, Some("X"));
        let enc = protect(&plain, "pw");
        let back = unlock_native(&enc, "pw").unwrap();
        let doc = Document::load_mem(&back).unwrap();
        let page = doc.page_iter().next().unwrap();
        let dict = doc.get_dictionary(page).unwrap();
        assert!(dict.has(b"Resources"));
        assert!(dict.has(b"MediaBox"));
    }

    #[test]
    fn roundtrip_rc4_r2_self() {
        let plain = sample_with_text(2, Some("RC4"));
        let enc = encrypt_doc(
            &plain,
            b"pw",
            b"pw",
            Perms {
                printing: true,
                copying: false,
                modifying: false,
            },
            Scheme::Rc4R2,
            [9u8; 16],
            [8u8; 16],
            [0u8; 32],
        )
        .unwrap();
        assert!(is_encrypted_native(&enc).unwrap());
        let back = unlock_native(&enc, "pw").unwrap();
        assert_eq!(page_count(&back), 2);
        assert!(page_content_text(&back, 0).contains("RC4"));
    }

    #[test]
    fn roundtrip_aesv3_r6_self() {
        let plain = sample_with_text(2, Some("AESV3"));
        let mut fk = [0u8; 32];
        for (i, b) in fk.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(7).wrapping_add(1);
        }
        let enc = encrypt_doc(
            &plain,
            b"user-pw",
            b"owner-pw",
            Perms {
                printing: true,
                copying: true,
                modifying: true,
            },
            Scheme::AesV3R6,
            [3u8; 16],
            [4u8; 16],
            fk,
        )
        .unwrap();
        assert!(is_encrypted_native(&enc).unwrap());
        // user password
        let back_u = unlock_native(&enc, "user-pw").unwrap();
        assert!(page_content_text(&back_u, 0).contains("AESV3"));
        // owner password
        let back_o = unlock_native(&enc, "owner-pw").unwrap();
        assert_eq!(page_count(&back_o), 2);
    }

    #[test]
    fn aesv3_wrong_password_is_incorrect() {
        let plain = sample(1);
        let enc = encrypt_doc(
            &plain,
            b"right",
            b"right",
            Perms {
                printing: true,
                copying: false,
                modifying: false,
            },
            Scheme::AesV3R6,
            [1u8; 16],
            [2u8; 16],
            [5u8; 32],
        )
        .unwrap();
        assert_eq!(
            unlock_native(&enc, "wrong").unwrap_err(),
            "Incorrect password"
        );
    }

    // --- unit: permission bit math ----------------------------------------

    #[test]
    fn compute_p_layout() {
        let none = compute_p(Perms {
            printing: false,
            copying: false,
            modifying: false,
        }) as u32;
        assert_eq!(none & P_RESERVED_LOW, 0);
        assert_eq!(none & (P_PRINT | P_COPY | P_MODIFY), 0);

        let all = compute_p(Perms {
            printing: true,
            copying: true,
            modifying: true,
        }) as u32;
        assert!(has(all, P_PRINT) && has(all, P_COPY) && has(all, P_MODIFY));
        // exactly bits 1-2 are clear in the all-permissions value
        assert_eq!(all, !P_RESERVED_LOW);
    }

    // --- unit: per-object key uniqueness ----------------------------------

    #[test]
    fn per_object_keys_do_not_collide() {
        let file_key = [42u8; 16];
        let mut seen = std::collections::HashSet::new();
        for n in 1u32..=200 {
            let k = per_object_key(&file_key, (n, 0), true);
            assert!(seen.insert(k), "collision at object {n}");
        }
        assert_eq!(seen.len(), 200);
    }

    #[test]
    fn rc4_known_answer() {
        // RFC-style test vector: key "Key", plaintext "Plaintext".
        let ct = rc4(b"Key", b"Plaintext");
        assert_eq!(ct, [0xBB, 0xF3, 0x16, 0xE8, 0xD9, 0x40, 0xAF, 0x0A, 0xD3]);
        // symmetric
        assert_eq!(rc4(b"Key", &ct), b"Plaintext");
    }

    #[test]
    fn pad_password_lengths() {
        assert_eq!(pad_password(b"").to_vec(), PAD.to_vec());
        let long = vec![b'a'; 40];
        assert_eq!(&pad_password(&long)[..], &[b'a'; 32][..]);
    }

    // --- property tests ---------------------------------------------------

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(24))]

        /// Any printable, non-space-bordered ASCII password round-trips.
        /// (Options are JSON-encoded with serde so quotes/backslashes survive.)
        #[test]
        fn prop_password_roundtrip(pw in "[!-~]{1,28}") {
            let plain = sample_with_text(1, Some("Z"));
            let json = serde_json::json!({ "userPassword": pw, "allowPrinting": true }).to_string();
            let enc = protect_native(&plain, &json).unwrap();
            prop_assert!(is_encrypted_native(&enc).unwrap());
            let back = unlock_native(&enc, &pw).unwrap();
            prop_assert!(page_content_text(&back, 0).contains("Z"));
        }

        /// All eight permission combinations encode the expected `/P` bits.
        #[test]
        fn prop_all_permission_combos(p in any::<bool>(), c in any::<bool>(), m in any::<bool>()) {
            let out = protect_native(&sample(1), &opts("pw", None, p, c, m)).unwrap();
            let bits = p_bits(&out);
            prop_assert_eq!(has(bits, P_PRINT), p);
            prop_assert_eq!(has(bits, P_COPY), c);
            prop_assert_eq!(has(bits, P_MODIFY), m);
            prop_assert_eq!(bits & P_RESERVED_LOW, 0);
        }

        /// Many objects, no per-object key collisions, for arbitrary file keys.
        #[test]
        fn prop_no_key_collisions(seed in any::<u8>()) {
            let file_key = [seed; 16];
            let mut seen = std::collections::HashSet::new();
            for n in 1u32..=150 {
                prop_assert!(seen.insert(per_object_key(&file_key, (n, 0), true)));
            }
        }
    }
}
