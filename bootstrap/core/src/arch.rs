//! Architecture-specific operations.
//!
//! Only the NEON operations that crates in our own graphs call are here, and they are written as ordinary
//! lane-by-lane Rust over plain arrays. That keeps this `core` free of vector language items, so any
//! compiler that can compile a loop can compile these; turning them back into vector instructions is the
//! optimizer's job.

/// Inline assembly. Expanded by the compiler.
#[rustc_builtin_macro]
pub macro asm("assembly template", $(operands,)* $(options($(option),*))?) {
    /* compiler built-in */
}

/// Module-level assembly. Expanded by the compiler.
#[rustc_builtin_macro]
pub macro global_asm("assembly template", $(operands,)* $(options($(option),*))?) {
    /* compiler built-in */
}

#[cfg(target_arch = "aarch64")]
pub mod aarch64 {
    #![allow(non_camel_case_types)]
    use crate::clone::Clone;
    use crate::marker::Copy;

    macro_rules! vector {
        ($($name:ident: [$lane:ty; $n:expr], align $align:literal;)*) => {$(
            #[repr(C, align($align))]
            pub struct $name(pub(crate) [$lane; $n]);
            impl Copy for $name {}
            impl Clone for $name { fn clone(&self) -> Self { *self } }
            impl crate::fmt::Debug for $name {
                fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
                    crate::fmt::Debug::fmt(&self.0, f)
                }
            }
        )*};
    }
    vector! {
        uint8x8_t: [u8; 8], align 8;
        int8x8_t: [i8; 8], align 8;
        uint64x1_t: [u64; 1], align 8;
        uint8x16_t: [u8; 16], align 16;
        uint16x8_t: [u16; 8], align 16;
        int16x8_t: [i16; 8], align 16;
        uint64x2_t: [u64; 2], align 16;
    }

    /// Lane-wise unary and binary operations: one loop each.
    macro_rules! lanes {
        ($n:expr, $out:ident, |$i:ident| $body:expr) => {{
            let mut out = [0; $n];
            let mut $i = 0;
            while $i < $n { out[$i] = $body; $i += 1; }
            $out(out)
        }};
    }

    pub fn vdup_n_u8(value: u8) -> uint8x8_t { uint8x8_t([value; 8]) }
    pub fn vdupq_n_u8(value: u8) -> uint8x16_t { uint8x16_t([value; 16]) }
    pub fn vdupq_n_s16(value: i16) -> int16x8_t { int16x8_t([value; 8]) }

    /// Reads 8 bytes. `ptr` need not be aligned.
    pub unsafe fn vld1_u8(ptr: *const u8) -> uint8x8_t {
        uint8x8_t(unsafe { crate::ptr::read_unaligned(ptr as *const [u8; 8]) })
    }
    /// Reads 16 bytes. `ptr` need not be aligned.
    pub unsafe fn vld1q_u8(ptr: *const u8) -> uint8x16_t {
        uint8x16_t(unsafe { crate::ptr::read_unaligned(ptr as *const [u8; 16]) })
    }
    /// Writes 8 bytes. `ptr` need not be aligned.
    pub unsafe fn vst1_u8(ptr: *mut u8, value: uint8x8_t) {
        unsafe { crate::ptr::write_unaligned(ptr as *mut [u8; 8], value.0) }
    }

    pub fn vceq_u8(a: uint8x8_t, b: uint8x8_t) -> uint8x8_t {
        lanes!(8, uint8x8_t, |i| if a.0[i] == b.0[i] { 0xff } else { 0 })
    }
    pub fn vceqq_u8(a: uint8x16_t, b: uint8x16_t) -> uint8x16_t {
        lanes!(16, uint8x16_t, |i| if a.0[i] == b.0[i] { 0xff } else { 0 })
    }
    /// All ones in each lane that is negative.
    pub fn vcltz_s8(a: int8x8_t) -> uint8x8_t { lanes!(8, uint8x8_t, |i| if a.0[i] < 0 { 0xff } else { 0 }) }
    /// All ones in each lane that is zero or positive.
    pub fn vcgez_s8(a: int8x8_t) -> uint8x8_t { lanes!(8, uint8x8_t, |i| if a.0[i] >= 0 { 0xff } else { 0 }) }

    pub fn vorr_u8(a: uint8x8_t, b: uint8x8_t) -> uint8x8_t { lanes!(8, uint8x8_t, |i| a.0[i] | b.0[i]) }
    pub fn vorrq_u8(a: uint8x16_t, b: uint8x16_t) -> uint8x16_t { lanes!(16, uint8x16_t, |i| a.0[i] | b.0[i]) }
    pub fn vandq_u8(a: uint8x16_t, b: uint8x16_t) -> uint8x16_t { lanes!(16, uint8x16_t, |i| a.0[i] & b.0[i]) }

    /// Pairwise maximum: the low half takes adjacent pairs of `a`, the high half adjacent pairs of `b`.
    pub fn vpmaxq_u8(a: uint8x16_t, b: uint8x16_t) -> uint8x16_t {
        lanes!(16, uint8x16_t, |i| {
            let from = if i < 8 { &a.0 } else { &b.0 };
            let (x, y) = (from[(i % 8) * 2], from[(i % 8) * 2 + 1]);
            if x < y { y } else { x }
        })
    }

    // Reinterpretations keep the bytes as they sit in memory.
    pub fn vreinterpret_s8_u8(a: uint8x8_t) -> int8x8_t { lanes!(8, int8x8_t, |i| a.0[i] as i8) }
    pub fn vreinterpret_u64_u8(a: uint8x8_t) -> uint64x1_t { uint64x1_t([u64::from_ne_bytes(a.0)]) }
    pub fn vreinterpretq_u16_u8(a: uint8x16_t) -> uint16x8_t {
        lanes!(8, uint16x8_t, |i| u16::from_ne_bytes([a.0[i * 2], a.0[i * 2 + 1]]))
    }
    pub fn vreinterpretq_u64_u8(a: uint8x16_t) -> uint64x2_t {
        lanes!(2, uint64x2_t, |i| {
            let b = &a.0;
            let at = i * 8;
            u64::from_ne_bytes([b[at], b[at + 1], b[at + 2], b[at + 3], b[at + 4], b[at + 5], b[at + 6], b[at + 7]])
        })
    }

    /// Each 16-bit lane shifted right by `N` and narrowed to its low 8 bits. Called as `vshrn_n_u16(a, n)`.
    #[rustc_legacy_const_generics(1)]
    pub fn vshrn_n_u16<const N: i32>(a: uint16x8_t) -> uint8x8_t {
        lanes!(8, uint8x8_t, |i| (a.0[i] >> (N as u16)) as u8)
    }
    /// Called as `vget_lane_u64(v, 0)`.
    #[rustc_legacy_const_generics(1)]
    pub fn vget_lane_u64<const LANE: i32>(v: uint64x1_t) -> u64 { v.0[LANE as usize] }
    /// Called as `vgetq_lane_u64(v, lane)`.
    #[rustc_legacy_const_generics(1)]
    pub fn vgetq_lane_u64<const LANE: i32>(v: uint64x2_t) -> u64 { v.0[LANE as usize] }

    pub fn vaddl_u8(a: uint8x8_t, b: uint8x8_t) -> uint16x8_t {
        lanes!(8, uint16x8_t, |i| (a.0[i] as u16) + (b.0[i] as u16))
    }
    pub fn vaddq_s16(a: int16x8_t, b: int16x8_t) -> int16x8_t {
        lanes!(8, int16x8_t, |i| a.0[i].wrapping_add(b.0[i]))
    }
    pub fn vmull_u8(a: uint8x8_t, b: uint8x8_t) -> uint16x8_t {
        lanes!(8, uint16x8_t, |i| (a.0[i] as u16) * (b.0[i] as u16))
    }
    pub fn vmlal_u8(a: uint16x8_t, b: uint8x8_t, c: uint8x8_t) -> uint16x8_t {
        lanes!(8, uint16x8_t, |i| a.0[i].wrapping_add((b.0[i] as u16) * (c.0[i] as u16)))
    }
    pub fn vmlaq_n_s16(a: int16x8_t, b: int16x8_t, c: i16) -> int16x8_t {
        lanes!(8, int16x8_t, |i| a.0[i].wrapping_add(b.0[i].wrapping_mul(c)))
    }
    pub fn vmlsq_n_s16(a: int16x8_t, b: int16x8_t, c: i16) -> int16x8_t {
        lanes!(8, int16x8_t, |i| a.0[i].wrapping_sub(b.0[i].wrapping_mul(c)))
    }
    pub fn vqmovun_s16(a: int16x8_t) -> uint8x8_t {
        lanes!(8, uint8x8_t, |i| if a.0[i] <= 0 { 0 } else if a.0[i] >= 255 { 255 } else { a.0[i] as u8 })
    }
    pub fn vreinterpretq_s16_u16(a: uint16x8_t) -> int16x8_t {
        lanes!(8, int16x8_t, |i| a.0[i] as i16)
    }
    pub fn vrhadd_u8(a: uint8x8_t, b: uint8x8_t) -> uint8x8_t {
        lanes!(8, uint8x8_t, |i| (((a.0[i] as u16) + (b.0[i] as u16) + 1) >> 1) as u8)
    }
    #[rustc_legacy_const_generics(1)]
    pub fn vrshrn_n_u16<const N: i32>(a: uint16x8_t) -> uint8x8_t {
        lanes!(8, uint8x8_t, |i| (((a.0[i] as u32) + (1u32 << ((N - 1) as u32))) >> (N as u32)) as u8)
    }
    #[rustc_legacy_const_generics(1)]
    pub fn vshrq_n_s16<const N: i32>(a: int16x8_t) -> int16x8_t {
        lanes!(8, int16x8_t, |i| a.0[i] >> (N as i16))
    }
}
