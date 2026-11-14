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
        int16x4_t: [i16; 4], align 8;
        int32x2_t: [i32; 2], align 8;
        uint32x2_t: [u32; 2], align 8;
        float32x2_t: [f32; 2], align 8;
        uint64x1_t: [u64; 1], align 8;
        uint8x16_t: [u8; 16], align 16;
        uint16x8_t: [u16; 8], align 16;
        int16x8_t: [i16; 8], align 16;
        int32x4_t: [i32; 4], align 16;
        uint32x4_t: [u32; 4], align 16;
        float32x4_t: [f32; 4], align 16;
        int64x2_t: [i64; 2], align 16;
        uint64x2_t: [u64; 2], align 16;
    }

    macro_rules! vector_pair {
        ($($name:ident: $elem:ty;)*) => {$(
            #[repr(C)]
            pub struct $name(pub $elem, pub $elem);
            impl Copy for $name {}
            impl Clone for $name { fn clone(&self) -> Self { *self } }
            impl crate::fmt::Debug for $name {
                fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
                    f.debug_tuple(stringify!($name)).field(&self.0).field(&self.1).finish()
                }
            }
        )*};
    }
    vector_pair! {
        float32x2x2_t: float32x2_t;
        float32x4x2_t: float32x4_t;
        int32x2x2_t: int32x2_t;
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

    macro_rules! lanes_f32 {
        ($n:expr, $out:ident, |$i:ident| $body:expr) => {{
            let mut out = [0.0f32; $n];
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
    pub unsafe fn vshrn_n_u16(a: uint16x8_t, n: i32) -> uint8x8_t {
        lanes!(8, uint8x8_t, |i| (a.0[i] >> (n as u16)) as u8)
    }
    /// Called as `vget_lane_u64(v, 0)`.
    pub unsafe fn vget_lane_u64(v: uint64x1_t, lane: i32) -> u64 { v.0[lane as usize] }
    /// Called as `vgetq_lane_u64(v, lane)`.
    pub unsafe fn vgetq_lane_u64(v: uint64x2_t, lane: i32) -> u64 { v.0[lane as usize] }

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
    pub unsafe fn vrshrn_n_u16(a: uint16x8_t, n: i32) -> uint8x8_t {
        lanes!(8, uint8x8_t, |i| (((a.0[i] as u32) + (1u32 << ((n - 1) as u32))) >> (n as u32)) as u8)
    }
    pub unsafe fn vshrq_n_s16(a: int16x8_t, n: i32) -> int16x8_t {
        lanes!(8, int16x8_t, |i| a.0[i] >> (n as i16))
    }

    pub unsafe fn veorq_u8(a: uint8x16_t, b: uint8x16_t) -> uint8x16_t {
        lanes!(16, uint8x16_t, |i| a.0[i] ^ b.0[i])
    }
    pub unsafe fn vmovq_n_u8(value: u8) -> uint8x16_t {
        uint8x16_t([value; 16])
    }
    pub unsafe fn vqsubq_u8(a: uint8x16_t, b: uint8x16_t) -> uint8x16_t {
        lanes!(16, uint8x16_t, |i| if a.0[i] > b.0[i] { a.0[i] - b.0[i] } else { 0 })
    }
    pub unsafe fn vqtbl1q_u8(t: uint8x16_t, idx: uint8x16_t) -> uint8x16_t {
        lanes!(16, uint8x16_t, |i| {
            let j = idx.0[i] as usize;
            if j < 16 { t.0[j] } else { 0 }
        })
    }
    pub unsafe fn vmaxvq_u8(a: uint8x16_t) -> u8 {
        let mut m = 0u8;
        let mut i = 0;
        while i < 16 {
            if a.0[i] > m { m = a.0[i]; }
            i += 1;
        }
        m
    }
    pub unsafe fn vshrq_n_u8(a: uint8x16_t, n: i32) -> uint8x16_t {
        lanes!(16, uint8x16_t, |i| a.0[i] >> (n as u32))
    }
    pub unsafe fn vextq_u8(a: uint8x16_t, b: uint8x16_t, n: i32) -> uint8x16_t {
        lanes!(16, uint8x16_t, |i| {
            let k = (n as usize) + i;
            if k < 16 { a.0[k] } else { b.0[k - 16] }
        })
    }

    // f32 NEON operations
    pub fn vdupq_n_f32(value: f32) -> float32x4_t { float32x4_t([value; 4]) }
    pub fn vdup_n_f32(value: f32) -> float32x2_t { float32x2_t([value; 2]) }
    pub fn vdup_lane_f32(a: float32x2_t, lane: i32) -> float32x2_t {
        float32x2_t([a.0[lane as usize]; 2])
    }
    pub fn vdupq_lane_f32(a: float32x2_t, lane: i32) -> float32x4_t {
        float32x4_t([a.0[lane as usize]; 4])
    }

    pub unsafe fn vld1q_f32(ptr: *const f32) -> float32x4_t {
        float32x4_t(unsafe { crate::ptr::read_unaligned(ptr as *const [f32; 4]) })
    }
    pub unsafe fn vld1_f32(ptr: *const f32) -> float32x2_t {
        float32x2_t(unsafe { crate::ptr::read_unaligned(ptr as *const [f32; 2]) })
    }
    pub unsafe fn vld1_dup_f32(ptr: *const f32) -> float32x2_t {
        let v = unsafe { crate::ptr::read_unaligned(ptr) };
        float32x2_t([v; 2])
    }
    pub unsafe fn vld1q_dup_f32(ptr: *const f32) -> float32x4_t {
        let v = unsafe { crate::ptr::read_unaligned(ptr) };
        float32x4_t([v; 4])
    }
    pub unsafe fn vld2q_f32(ptr: *const f32) -> float32x4x2_t {
        let x = unsafe { crate::ptr::read_unaligned(ptr as *const [f32; 8]) };
        float32x4x2_t(
            float32x4_t([x[0], x[2], x[4], x[6]]),
            float32x4_t([x[1], x[3], x[5], x[7]]),
        )
    }
    pub unsafe fn vst1q_f32(ptr: *mut f32, value: float32x4_t) {
        unsafe { crate::ptr::write_unaligned(ptr as *mut [f32; 4], value.0) }
    }
    pub unsafe fn vst1_f32(ptr: *mut f32, value: float32x2_t) {
        unsafe { crate::ptr::write_unaligned(ptr as *mut [f32; 2], value.0) }
    }

    pub fn vaddq_f32(a: float32x4_t, b: float32x4_t) -> float32x4_t {
        lanes_f32!(4, float32x4_t, |i| a.0[i] + b.0[i])
    }
    pub fn vadd_f32(a: float32x2_t, b: float32x2_t) -> float32x2_t {
        lanes_f32!(2, float32x2_t, |i| a.0[i] + b.0[i])
    }
    pub fn vsubq_f32(a: float32x4_t, b: float32x4_t) -> float32x4_t {
        lanes_f32!(4, float32x4_t, |i| a.0[i] - b.0[i])
    }
    pub fn vsub_f32(a: float32x2_t, b: float32x2_t) -> float32x2_t {
        lanes_f32!(2, float32x2_t, |i| a.0[i] - b.0[i])
    }
    pub fn vmulq_f32(a: float32x4_t, b: float32x4_t) -> float32x4_t {
        lanes_f32!(4, float32x4_t, |i| a.0[i] * b.0[i])
    }
    pub fn vmul_f32(a: float32x2_t, b: float32x2_t) -> float32x2_t {
        lanes_f32!(2, float32x2_t, |i| a.0[i] * b.0[i])
    }
    pub fn vmulq_n_f32(a: float32x4_t, b: f32) -> float32x4_t {
        lanes_f32!(4, float32x4_t, |i| a.0[i] * b)
    }
    pub fn vmul_n_f32(a: float32x2_t, b: f32) -> float32x2_t {
        lanes_f32!(2, float32x2_t, |i| a.0[i] * b)
    }
    pub fn vfmaq_f32(a: float32x4_t, b: float32x4_t, c: float32x4_t) -> float32x4_t {
        lanes_f32!(4, float32x4_t, |i| a.0[i] + b.0[i] * c.0[i])
    }
    pub fn vfmsq_f32(a: float32x4_t, b: float32x4_t, c: float32x4_t) -> float32x4_t {
        lanes_f32!(4, float32x4_t, |i| a.0[i] - b.0[i] * c.0[i])
    }
    pub fn vfmaq_lane_f32(a: float32x4_t, b: float32x4_t, v: float32x2_t, lane: i32) -> float32x4_t {
        let s = v.0[lane as usize];
        lanes_f32!(4, float32x4_t, |i| a.0[i] + b.0[i] * s)
    }
    pub fn vfmaq_n_f32(a: float32x4_t, b: float32x4_t, c: f32) -> float32x4_t {
        lanes_f32!(4, float32x4_t, |i| a.0[i] + b.0[i] * c)
    }
    pub fn vabsq_f32(a: float32x4_t) -> float32x4_t {
        lanes_f32!(4, float32x4_t, |i| f32::from_bits(a.0[i].to_bits() & 0x7fff_ffff))
    }
    pub fn vnegq_f32(a: float32x4_t) -> float32x4_t {
        lanes_f32!(4, float32x4_t, |i| -a.0[i])
    }
    pub fn vneg_f32(a: float32x2_t) -> float32x2_t {
        lanes_f32!(2, float32x2_t, |i| -a.0[i])
    }
    pub fn vpaddq_f32(a: float32x4_t, b: float32x4_t) -> float32x4_t {
        float32x4_t([a.0[0] + a.0[1], a.0[2] + a.0[3], b.0[0] + b.0[1], b.0[2] + b.0[3]])
    }
    pub fn vpadd_f32(a: float32x2_t, b: float32x2_t) -> float32x2_t {
        float32x2_t([a.0[0] + a.0[1], b.0[0] + b.0[1]])
    }
    pub fn vaddvq_f32(a: float32x4_t) -> f32 {
        (a.0[0] + a.0[1]) + (a.0[2] + a.0[3])
    }
    pub fn vgetq_lane_f32(v: float32x4_t, lane: i32) -> f32 { v.0[lane as usize] }
    pub fn vget_lane_f32(v: float32x2_t, lane: i32) -> f32 { v.0[lane as usize] }
    pub fn vsetq_lane_f32(a: f32, mut v: float32x4_t, lane: i32) -> float32x4_t {
        v.0[lane as usize] = a;
        v
    }
    pub fn vset_lane_f32(a: f32, mut v: float32x2_t, lane: i32) -> float32x2_t {
        v.0[lane as usize] = a;
        v
    }
    pub fn vextq_f32(a: float32x4_t, b: float32x4_t, n: i32) -> float32x4_t {
        lanes_f32!(4, float32x4_t, |i| {
            let k = (n as usize) + i;
            if k < 4 { a.0[k] } else { b.0[k - 4] }
        })
    }
    pub fn vext_f32(a: float32x2_t, b: float32x2_t, n: i32) -> float32x2_t {
        lanes_f32!(2, float32x2_t, |i| {
            let k = (n as usize) + i;
            if k < 2 { a.0[k] } else { b.0[k - 2] }
        })
    }
    pub fn vrev64q_f32(a: float32x4_t) -> float32x4_t {
        float32x4_t([a.0[1], a.0[0], a.0[3], a.0[2]])
    }
    pub fn vget_low_f32(a: float32x4_t) -> float32x2_t { float32x2_t([a.0[0], a.0[1]]) }
    pub fn vget_high_f32(a: float32x4_t) -> float32x2_t { float32x2_t([a.0[2], a.0[3]]) }
    pub fn vcombine_f32(a: float32x2_t, b: float32x2_t) -> float32x4_t {
        float32x4_t([a.0[0], a.0[1], b.0[0], b.0[1]])
    }
    pub fn vtrnq_f32(a: float32x4_t, b: float32x4_t) -> float32x4x2_t {
        float32x4x2_t(
            float32x4_t([a.0[0], b.0[0], a.0[2], b.0[2]]),
            float32x4_t([a.0[1], b.0[1], a.0[3], b.0[3]]),
        )
    }
    pub fn vuzpq_f32(a: float32x4_t, b: float32x4_t) -> float32x4x2_t {
        float32x4x2_t(
            float32x4_t([a.0[0], a.0[2], b.0[0], b.0[2]]),
            float32x4_t([a.0[1], a.0[3], b.0[1], b.0[3]]),
        )
    }
    pub fn vzipq_f32(a: float32x4_t, b: float32x4_t) -> float32x4x2_t {
        float32x4x2_t(
            float32x4_t([a.0[0], b.0[0], a.0[1], b.0[1]]),
            float32x4_t([a.0[2], b.0[2], a.0[3], b.0[3]]),
        )
    }
    pub fn vmaxq_f32(a: float32x4_t, b: float32x4_t) -> float32x4_t {
        lanes_f32!(4, float32x4_t, |i| if a.0[i] >= b.0[i] { a.0[i] } else { b.0[i] })
    }
    pub fn vminq_f32(a: float32x4_t, b: float32x4_t) -> float32x4_t {
        lanes_f32!(4, float32x4_t, |i| if a.0[i] <= b.0[i] { a.0[i] } else { b.0[i] })
    }

    // Integer and conversion NEON operations
    pub fn vcvtq_f32_s32(a: int32x4_t) -> float32x4_t {
        lanes_f32!(4, float32x4_t, |i| a.0[i] as f32)
    }
    pub fn vcvtq_s32_f32(a: float32x4_t) -> int32x4_t {
        lanes!(4, int32x4_t, |i| a.0[i] as i32)
    }

    pub unsafe fn vld1q_s32(ptr: *const i32) -> int32x4_t {
        int32x4_t(unsafe { crate::ptr::read_unaligned(ptr as *const [i32; 4]) })
    }
    pub unsafe fn vld1_s32(ptr: *const i32) -> int32x2_t {
        int32x2_t(unsafe { crate::ptr::read_unaligned(ptr as *const [i32; 2]) })
    }
    pub unsafe fn vst1q_s32(ptr: *mut i32, value: int32x4_t) {
        unsafe { crate::ptr::write_unaligned(ptr as *mut [i32; 4], value.0) }
    }
    pub unsafe fn vst1_s32(ptr: *mut i32, value: int32x2_t) {
        unsafe { crate::ptr::write_unaligned(ptr as *mut [i32; 2], value.0) }
    }
    pub unsafe fn vst1q_lane_s32(ptr: *mut i32, value: int32x4_t, lane: i32) {
        unsafe { crate::ptr::write_unaligned(ptr, value.0[lane as usize]) }
    }
    pub unsafe fn vld1q_s16(ptr: *const i16) -> int16x8_t {
        int16x8_t(unsafe { crate::ptr::read_unaligned(ptr as *const [i16; 8]) })
    }
    pub unsafe fn vld1_s16(ptr: *const i16) -> int16x4_t {
        int16x4_t(unsafe { crate::ptr::read_unaligned(ptr as *const [i16; 4]) })
    }
    pub unsafe fn vld1_dup_s16(ptr: *const i16) -> int16x4_t {
        let v = unsafe { crate::ptr::read_unaligned(ptr) };
        int16x4_t([v; 4])
    }
    pub unsafe fn vst1_s16(ptr: *mut i16, value: int16x4_t) {
        unsafe { crate::ptr::write_unaligned(ptr as *mut [i16; 4], value.0) }
    }

    pub fn vdupq_n_s32(value: i32) -> int32x4_t { int32x4_t([value; 4]) }
    pub fn vdup_n_s32(value: i32) -> int32x2_t { int32x2_t([value; 2]) }
    pub fn vdupq_n_u32(value: u32) -> uint32x4_t { uint32x4_t([value; 4]) }
    pub fn vdup_n_u32(value: u32) -> uint32x2_t { uint32x2_t([value; 2]) }

    pub fn vorrq_s32(a: int32x4_t, b: int32x4_t) -> int32x4_t {
        lanes!(4, int32x4_t, |i| a.0[i] | b.0[i])
    }
    pub fn vandq_s32(a: int32x4_t, b: int32x4_t) -> int32x4_t {
        lanes!(4, int32x4_t, |i| a.0[i] & b.0[i])
    }
    pub fn veorq_s32(a: int32x4_t, b: int32x4_t) -> int32x4_t {
        lanes!(4, int32x4_t, |i| a.0[i] ^ b.0[i])
    }
    pub fn vextq_s32(a: int32x4_t, b: int32x4_t, n: i32) -> int32x4_t {
        lanes!(4, int32x4_t, |i| {
            let k = (n as usize) + i;
            if k < 4 { a.0[k] } else { b.0[k - 4] }
        })
    }
    pub fn vextq_s16(a: int16x8_t, b: int16x8_t, n: i32) -> int16x8_t {
        lanes!(8, int16x8_t, |i| {
            let k = (n as usize) + i;
            if k < 8 { a.0[k] } else { b.0[k - 8] }
        })
    }

    pub fn vgetq_lane_s32(v: int32x4_t, lane: i32) -> i32 { v.0[lane as usize] }
    pub fn vget_lane_s32(v: int32x2_t, lane: i32) -> i32 { v.0[lane as usize] }
    pub fn vrev64q_s32(a: int32x4_t) -> int32x4_t {
        int32x4_t([a.0[1], a.0[0], a.0[3], a.0[2]])
    }
    pub fn vcombine_s32(a: int32x2_t, b: int32x2_t) -> int32x4_t {
        int32x4_t([a.0[0], a.0[1], b.0[0], b.0[1]])
    }
    pub fn vget_high_s32(a: int32x4_t) -> int32x2_t { int32x2_t([a.0[2], a.0[3]]) }
    pub fn vget_low_s32(a: int32x4_t) -> int32x2_t { int32x2_t([a.0[0], a.0[1]]) }
    pub fn vget_low_s16(a: int16x8_t) -> int16x4_t {
        int16x4_t([a.0[0], a.0[1], a.0[2], a.0[3]])
    }
    pub fn vget_high_s16(a: int16x8_t) -> int16x4_t {
        int16x4_t([a.0[4], a.0[5], a.0[6], a.0[7]])
    }

    pub fn vmovl_s16(a: int16x4_t) -> int32x4_t {
        lanes!(4, int32x4_t, |i| a.0[i] as i32)
    }
    pub fn vmull_s16(a: int16x4_t, b: int16x4_t) -> int32x4_t {
        lanes!(4, int32x4_t, |i| (a.0[i] as i32) * (b.0[i] as i32))
    }
    pub fn vmull_s32(a: int32x2_t, b: int32x2_t) -> int64x2_t {
        lanes!(2, int64x2_t, |i| (a.0[i] as i64) * (b.0[i] as i64))
    }
    pub fn vmlal_s16(a: int32x4_t, b: int16x4_t, c: int16x4_t) -> int32x4_t {
        lanes!(4, int32x4_t, |i| a.0[i].wrapping_add((b.0[i] as i32) * (c.0[i] as i32)))
    }
    pub fn vmlal_high_s16(a: int32x4_t, b: int16x8_t, c: int16x8_t) -> int32x4_t {
        lanes!(4, int32x4_t, |i| a.0[i].wrapping_add((b.0[i + 4] as i32) * (c.0[i + 4] as i32)))
    }
    pub fn vshll_n_s16(a: int16x4_t, n: i32) -> int32x4_t {
        lanes!(4, int32x4_t, |i| (a.0[i] as i32) << (n as u32))
    }
    pub fn vshrn_n_s64(a: int64x2_t, n: i32) -> int32x2_t {
        lanes!(2, int32x2_t, |i| (a.0[i] >> (n as u32)) as i32)
    }
    pub fn vqshrn_n_s32(a: int32x4_t, n: i32) -> int16x4_t {
        lanes!(4, int16x4_t, |i| {
            let v = a.0[i] >> (n as u32);
            if v < i16::MIN as i32 {
                i16::MIN
            } else if v > i16::MAX as i32 {
                i16::MAX
            } else {
                v as i16
            }
        })
    }
    pub fn vshr_n_u32(a: uint32x2_t, n: i32) -> uint32x2_t {
        lanes!(2, uint32x2_t, |i| a.0[i] >> (n as u32))
    }
    pub fn vshl_n_s32(a: int32x2_t, n: i32) -> int32x2_t {
        lanes!(2, int32x2_t, |i| a.0[i].wrapping_shl(n as u32))
    }
    pub fn vshr_n_s32(a: int32x2_t, n: i32) -> int32x2_t {
        lanes!(2, int32x2_t, |i| a.0[i] >> (n as u32))
    }
    pub fn vshlq_u32(a: uint32x4_t, b: int32x4_t) -> uint32x4_t {
        lanes!(4, uint32x4_t, |i| {
            let s = b.0[i] as i8 as i32;
            if s >= 32 || s <= -32 {
                0
            } else if s >= 0 {
                a.0[i] << (s as u32)
            } else {
                a.0[i] >> ((-s) as u32)
            }
        })
    }
    pub fn vshrq_n_s32(a: int32x4_t, n: i32) -> int32x4_t {
        lanes!(4, int32x4_t, |i| a.0[i] >> (n as u32))
    }
    pub fn vshlq_n_s32(a: int32x4_t, n: i32) -> int32x4_t {
        lanes!(4, int32x4_t, |i| a.0[i].wrapping_shl(n as u32))
    }
    pub fn vrsraq_n_s32(a: int32x4_t, b: int32x4_t, n: i32) -> int32x4_t {
        lanes!(4, int32x4_t, |i| {
            let r = ((b.0[i] as i64) + (1i64 << ((n - 1) as u32))) >> (n as u32);
            a.0[i].wrapping_add(r as i32)
        })
    }

    pub fn vaddq_s32(a: int32x4_t, b: int32x4_t) -> int32x4_t {
        lanes!(4, int32x4_t, |i| a.0[i].wrapping_add(b.0[i]))
    }
    pub fn vadd_s32(a: int32x2_t, b: int32x2_t) -> int32x2_t {
        lanes!(2, int32x2_t, |i| a.0[i].wrapping_add(b.0[i]))
    }
    pub fn vsubq_s32(a: int32x4_t, b: int32x4_t) -> int32x4_t {
        lanes!(4, int32x4_t, |i| a.0[i].wrapping_sub(b.0[i]))
    }
    pub fn vneg_s32(a: int32x2_t) -> int32x2_t {
        lanes!(2, int32x2_t, |i| a.0[i].wrapping_neg())
    }
    pub fn vnegq_s32(a: int32x4_t) -> int32x4_t {
        lanes!(4, int32x4_t, |i| a.0[i].wrapping_neg())
    }
    pub fn vpaddq_s32(a: int32x4_t, b: int32x4_t) -> int32x4_t {
        int32x4_t([
            a.0[0].wrapping_add(a.0[1]),
            a.0[2].wrapping_add(a.0[3]),
            b.0[0].wrapping_add(b.0[1]),
            b.0[2].wrapping_add(b.0[3]),
        ])
    }
    pub fn vpaddlq_s32(a: int32x4_t) -> int64x2_t {
        int64x2_t([(a.0[0] as i64) + (a.0[1] as i64), (a.0[2] as i64) + (a.0[3] as i64)])
    }
    pub fn vaddvq_s32(a: int32x4_t) -> i32 {
        a.0[0].wrapping_add(a.0[1]).wrapping_add(a.0[2]).wrapping_add(a.0[3])
    }
    pub fn vaddvq_u32(a: uint32x4_t) -> u32 {
        a.0[0].wrapping_add(a.0[1]).wrapping_add(a.0[2]).wrapping_add(a.0[3])
    }
    pub fn vaddvq_s64(a: int64x2_t) -> i64 {
        a.0[0].wrapping_add(a.0[1])
    }
    pub fn vaddq_u32(a: uint32x4_t, b: uint32x4_t) -> uint32x4_t {
        lanes!(4, uint32x4_t, |i| a.0[i].wrapping_add(b.0[i]))
    }

    pub fn vreinterpretq_u32_s32(a: int32x4_t) -> uint32x4_t {
        lanes!(4, uint32x4_t, |i| a.0[i] as u32)
    }
    pub fn vreinterpretq_s32_u32(a: uint32x4_t) -> int32x4_t {
        lanes!(4, int32x4_t, |i| a.0[i] as i32)
    }
    pub fn vreinterpret_s32_u32(a: uint32x2_t) -> int32x2_t {
        lanes!(2, int32x2_t, |i| a.0[i] as i32)
    }
    pub fn vreinterpret_u32_s32(a: int32x2_t) -> uint32x2_t {
        lanes!(2, uint32x2_t, |i| a.0[i] as u32)
    }

    pub fn vzip_s32(a: int32x2_t, b: int32x2_t) -> int32x2x2_t {
        int32x2x2_t(int32x2_t([a.0[0], b.0[0]]), int32x2_t([a.0[1], b.0[1]]))
    }
    pub fn vtrn_s32(a: int32x2_t, b: int32x2_t) -> int32x2x2_t {
        int32x2x2_t(int32x2_t([a.0[0], b.0[0]]), int32x2_t([a.0[1], b.0[1]]))
    }
    pub fn vqdmulhq_s32(a: int32x4_t, b: int32x4_t) -> int32x4_t {
        lanes!(4, int32x4_t, |i| {
            if a.0[i] == i32::MIN && b.0[i] == i32::MIN {
                i32::MAX
            } else {
                (((a.0[i] as i64) * (b.0[i] as i64)) >> 31) as i32
            }
        })
    }
    pub fn vqdmulhq_lane_s32(a: int32x4_t, v: int32x2_t, lane: i32) -> int32x4_t {
        let b = v.0[lane as usize];
        lanes!(4, int32x4_t, |i| {
            if a.0[i] == i32::MIN && b == i32::MIN {
                i32::MAX
            } else {
                (((a.0[i] as i64) * (b as i64)) >> 31) as i32
            }
        })
    }
}
