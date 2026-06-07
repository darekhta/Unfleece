import { describe, expect, it } from 'vitest';
import { PackBuilder } from '@lib/wasm/pack.js';

// Byte-level contract test: this layout is what core/src/pack.rs PackReader
// parses. If this test needs changing, the Rust side must change in lockstep.
describe('PackBuilder', () => {
  it('produces the exact little-endian layout PackReader expects', () => {
    const pack = new PackBuilder('TEST').u8(7).u16(513).u32(70_000).f32(1.5).bytes(new Uint8Array([1, 2, 3])).str('hé').finish();

    const expected = [
      ...[0x54, 0x45, 0x53, 0x54], // "TEST"
      7,
      ...[0x01, 0x02], // 513 LE
      ...[0x70, 0x11, 0x01, 0x00], // 70000 LE
      ...[0x00, 0x00, 0xc0, 0x3f], // 1.5f LE
      ...[3, 0, 0, 0, 1, 2, 3], // len-prefixed bytes
      ...[3, 0, 0, 0, 0x68, 0xc3, 0xa9], // len-prefixed UTF-8 "hé"
    ];
    expect(Array.from(pack)).toEqual(expected);
  });

  it('handles empty payloads and large values', () => {
    const pack = new PackBuilder('XXXX').u32(0xffffffff).bytes(new Uint8Array(0)).finish();
    expect(pack.length).toBe(4 + 4 + 4);
    expect(Array.from(pack.slice(4, 8))).toEqual([0xff, 0xff, 0xff, 0xff]);
  });
});
