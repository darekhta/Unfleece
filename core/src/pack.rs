//! Little-endian binary "pack" reader/writer shared by all WASM entry points
//! that need structured multi-part input (lists of files, images, placements…).
//!
//! Packs keep the JS↔WASM boundary to a single `&[u8]` argument, which avoids
//! per-element copies and keeps wasm-bindgen signatures trivial.

pub struct PackReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> PackReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn is_done(&self) -> bool {
        self.pos == self.data.len()
    }

    pub fn expect_magic(&mut self, magic: &[u8]) -> Result<(), String> {
        if self.read_exact(magic.len())? != magic {
            return Err("Invalid input pack".to_string());
        }
        Ok(())
    }

    pub fn read_exact(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| "Input pack is too large".to_string())?;
        if end > self.data.len() {
            return Err("Input pack ended early".to_string());
        }
        let out = &self.data[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    pub fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.read_exact(1)?[0])
    }

    pub fn read_u16(&mut self) -> Result<u16, String> {
        let mut bytes = [0; 2];
        bytes.copy_from_slice(self.read_exact(2)?);
        Ok(u16::from_le_bytes(bytes))
    }

    pub fn read_u32(&mut self) -> Result<u32, String> {
        let mut bytes = [0; 4];
        bytes.copy_from_slice(self.read_exact(4)?);
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn read_f32(&mut self) -> Result<f32, String> {
        let mut bytes = [0; 4];
        bytes.copy_from_slice(self.read_exact(4)?);
        Ok(f32::from_le_bytes(bytes))
    }

    /// Read a u32 length followed by that many bytes.
    pub fn read_bytes(&mut self) -> Result<&'a [u8], String> {
        let len = self.read_u32()? as usize;
        self.read_exact(len)
    }

    /// Read a u32 length followed by that many bytes of UTF-8.
    pub fn read_str(&mut self) -> Result<&'a str, String> {
        std::str::from_utf8(self.read_bytes()?)
            .map_err(|_| "Invalid UTF-8 in input pack".to_string())
    }

    pub fn expect_done(&self) -> Result<(), String> {
        if !self.is_done() {
            return Err("Input pack has trailing data".to_string());
        }
        Ok(())
    }
}

/// Builder mirror of [`PackReader`] — used by tests (and documents the format
/// the TypeScript side must produce).
#[derive(Default)]
pub struct PackWriter {
    out: Vec<u8>,
}

impl PackWriter {
    pub fn new(magic: &[u8]) -> Self {
        Self {
            out: magic.to_vec(),
        }
    }

    pub fn u8(mut self, v: u8) -> Self {
        self.out.push(v);
        self
    }

    pub fn u16(mut self, v: u16) -> Self {
        self.out.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn u32(mut self, v: u32) -> Self {
        self.out.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn f32(mut self, v: f32) -> Self {
        self.out.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn bytes(mut self, v: &[u8]) -> Self {
        self.out.extend_from_slice(&(v.len() as u32).to_le_bytes());
        self.out.extend_from_slice(v);
        self
    }

    pub fn str(self, v: &str) -> Self {
        self.bytes(v.as_bytes())
    }

    pub fn finish(self) -> Vec<u8> {
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_all_primitives() {
        let pack = PackWriter::new(b"TEST")
            .u8(7)
            .u16(513)
            .u32(70_000)
            .f32(1.5)
            .bytes(&[1, 2, 3])
            .str("héllo")
            .finish();
        let mut r = PackReader::new(&pack);
        r.expect_magic(b"TEST").unwrap();
        assert_eq!(r.read_u8().unwrap(), 7);
        assert_eq!(r.read_u16().unwrap(), 513);
        assert_eq!(r.read_u32().unwrap(), 70_000);
        assert_eq!(r.read_f32().unwrap(), 1.5);
        assert_eq!(r.read_bytes().unwrap(), &[1, 2, 3]);
        assert_eq!(r.read_str().unwrap(), "héllo");
        r.expect_done().unwrap();
    }

    #[test]
    fn rejects_truncated_and_trailing_data() {
        let pack = PackWriter::new(b"TEST").u32(10).finish(); // declares 10 bytes, provides none
        let mut r = PackReader::new(&pack);
        r.expect_magic(b"TEST").unwrap();
        assert!(r.read_bytes().is_err());

        let pack = PackWriter::new(b"TEST").u8(1).u8(2).finish();
        let mut r = PackReader::new(&pack);
        r.expect_magic(b"TEST").unwrap();
        r.read_u8().unwrap();
        assert!(r.expect_done().is_err());
    }

    #[test]
    fn rejects_wrong_magic() {
        let mut r = PackReader::new(b"NOPE...");
        assert!(r.expect_magic(b"TEST").is_err());
    }
}
