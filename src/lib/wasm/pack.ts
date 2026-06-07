// TypeScript mirror of core/src/pack.rs — little-endian length-prefixed packs
// that cross the JS↔WASM boundary as a single Uint8Array. Keep the two files
// in sync; the Rust PackReader is the source of truth for the format.

export class PackBuilder {
  private chunks: Uint8Array[] = [];
  private size = 0;

  constructor(magic: string) {
    this.raw(new TextEncoder().encode(magic));
  }

  private raw(bytes: Uint8Array): this {
    this.chunks.push(bytes);
    this.size += bytes.length;
    return this;
  }

  u8(v: number): this {
    return this.raw(new Uint8Array([v & 0xff]));
  }

  u16(v: number): this {
    const b = new Uint8Array(2);
    new DataView(b.buffer).setUint16(0, v, true);
    return this.raw(b);
  }

  u32(v: number): this {
    const b = new Uint8Array(4);
    new DataView(b.buffer).setUint32(0, v >>> 0, true);
    return this.raw(b);
  }

  f32(v: number): this {
    const b = new Uint8Array(4);
    new DataView(b.buffer).setFloat32(0, v, true);
    return this.raw(b);
  }

  /** u32 length followed by the bytes. */
  bytes(v: Uint8Array): this {
    this.u32(v.length);
    return this.raw(v);
  }

  /** u32 length followed by UTF-8 bytes. */
  str(v: string): this {
    return this.bytes(new TextEncoder().encode(v));
  }

  finish(): Uint8Array {
    const out = new Uint8Array(this.size);
    let pos = 0;
    for (const c of this.chunks) {
      out.set(c, pos);
      pos += c.length;
    }
    return out;
  }
}
