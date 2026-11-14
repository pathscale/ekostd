use crate::option::Option::{self, None, Some};

#[rustc_intrinsic]
#[rustc_nounwind]
const fn add_with_overflow<T>(a: T, b: T) -> (T, bool);
#[rustc_intrinsic]
#[rustc_nounwind]
const fn sub_with_overflow<T>(a: T, b: T) -> (T, bool);
#[rustc_intrinsic]
#[rustc_nounwind]
const fn mul_with_overflow<T>(a: T, b: T) -> (T, bool);
#[rustc_intrinsic]
#[rustc_nounwind]
const fn ctlz<T>(x: T) -> u32;
#[rustc_intrinsic]
#[rustc_nounwind]
const fn cttz<T>(x: T) -> u32;
#[rustc_intrinsic]
#[rustc_nounwind]
const fn ctpop<T>(x: T) -> u32;

macro_rules! integer {
    ($t:ty, $bits:expr, $signed:expr) => {
        impl $t {
            pub const MIN: Self = if $signed { (!0 as $t) << (($bits - 1) as $t) } else { 0 };
            pub const MAX: Self = if $signed { !(Self::MIN) } else { !0 };
            pub const BITS: u32 = $bits;

            pub const fn wrapping_add(self, rhs: Self) -> Self { add_with_overflow(self, rhs).0 }
            pub const fn wrapping_sub(self, rhs: Self) -> Self { sub_with_overflow(self, rhs).0 }
            pub const fn wrapping_mul(self, rhs: Self) -> Self { mul_with_overflow(self, rhs).0 }
            /// Shift with the amount taken modulo the width, so it cannot overflow.
            pub const fn wrapping_shl(self, rhs: u32) -> Self { self << ((rhs & ($bits - 1)) as $t) }
            pub const fn wrapping_shr(self, rhs: u32) -> Self { self >> ((rhs & ($bits - 1)) as $t) }
            pub const fn wrapping_neg(self) -> Self { (0 as Self).wrapping_sub(self) }
            pub const fn checked_add(self, rhs: Self) -> Option<Self> {
                let (value, overflow) = add_with_overflow(self, rhs);
                if overflow { None } else { Some(value) }
            }
            pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
                let (value, overflow) = sub_with_overflow(self, rhs);
                if overflow { None } else { Some(value) }
            }
            pub const fn checked_mul(self, rhs: Self) -> Option<Self> {
                let (value, overflow) = mul_with_overflow(self, rhs);
                if overflow { None } else { Some(value) }
            }
            pub const fn saturating_add(self, rhs: Self) -> Self {
                let (value, overflow) = add_with_overflow(self, rhs);
                if !overflow { value } else if $signed && self < 0 { Self::MIN } else { Self::MAX }
            }
            pub const fn saturating_sub(self, rhs: Self) -> Self {
                let (value, overflow) = sub_with_overflow(self, rhs);
                if !overflow { value } else if !$signed { 0 } else if self < 0 { Self::MIN } else { Self::MAX }
            }
            pub const fn saturating_mul(self, rhs: Self) -> Self {
                let (value, overflow) = mul_with_overflow(self, rhs);
                if !overflow { value } else if $signed && ((self < 0) != (rhs < 0)) { Self::MIN } else { Self::MAX }
            }
            /// Parses digits in `radix` (2 to 36), with an optional `+`, and `-` for signed types.
            pub fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseIntError> {
                let bytes = src.as_bytes();
                if bytes.len() == 0 { return Err(ParseIntError { kind: IntErrorKind::Empty }); }
                let (negative, mut i) = match bytes[0] {
                    b'+' => (false, 1),
                    b'-' if $signed => (true, 1),
                    _ => (false, 0),
                };
                if i == bytes.len() { return Err(ParseIntError { kind: IntErrorKind::InvalidDigit }); }
                let mut value: Self = 0;
                while i < bytes.len() {
                    let digit = match bytes[i] {
                        b @ b'0'..=b'9' => (b - b'0') as u32,
                        b @ b'a'..=b'z' => (b - b'a') as u32 + 10,
                        b @ b'A'..=b'Z' => (b - b'A') as u32 + 10,
                        _ => radix,
                    };
                    if digit >= radix { return Err(ParseIntError { kind: IntErrorKind::InvalidDigit }); }
                    // Negative values are built downwards so that MIN, which has no positive twin, parses.
                    let step = match value.checked_mul(radix as Self) {
                        Some(scaled) if negative => scaled.checked_sub(digit as Self),
                        Some(scaled) => scaled.checked_add(digit as Self),
                        None => None,
                    };
                    value = match step {
                        Some(next) => next,
                        None if negative => return Err(ParseIntError { kind: IntErrorKind::NegOverflow }),
                        None => return Err(ParseIntError { kind: IntErrorKind::PosOverflow }),
                    };
                    i += 1;
                }
                Ok(value)
            }
            pub const fn checked_shl(self, rhs: u32) -> Option<Self> {
                if rhs < $bits { Some(self << (rhs as $t)) } else { None }
            }
            pub const fn checked_shr(self, rhs: u32) -> Option<Self> {
                if rhs < $bits { Some(self >> (rhs as $t)) } else { None }
            }
            pub const fn count_ones(self) -> u32 { ctpop(self) }
            pub const fn count_zeros(self) -> u32 { (!self).count_ones() }
            pub const fn leading_zeros(self) -> u32 { ctlz(self) }
            pub const fn trailing_zeros(self) -> u32 { cttz(self) }
            pub const fn leading_ones(self) -> u32 { (!self).leading_zeros() }
            pub const fn trailing_ones(self) -> u32 { (!self).trailing_zeros() }
            pub const fn to_be(self) -> Self {
                #[cfg(target_endian = "big")] { self }
                #[cfg(not(target_endian = "big"))] { self.swap_bytes() }
            }
            pub const fn to_le(self) -> Self {
                #[cfg(target_endian = "little")] { self }
                #[cfg(not(target_endian = "little"))] { self.swap_bytes() }
            }
            pub const fn from_be(x: Self) -> Self {
                #[cfg(target_endian = "big")] { x }
                #[cfg(not(target_endian = "big"))] { x.swap_bytes() }
            }
            pub const fn from_le(x: Self) -> Self {
                #[cfg(target_endian = "little")] { x }
                #[cfg(not(target_endian = "little"))] { x.swap_bytes() }
            }
            pub const fn checked_ilog2(self) -> Option<u32> {
                if self <= 0 { None } else { Some(Self::BITS - 1 - self.leading_zeros()) }
            }
            pub const fn ilog2(self) -> u32 {
                match self.checked_ilog2() { Some(v) => v, None => crate::intrinsics::abort() }
            }
            pub const fn checked_ilog(self, base: Self) -> Option<u32> {
                if self <= 0 || base <= 1 { return None; }
                let mut n = self;
                let mut log = 0u32;
                while n >= base {
                    n /= base;
                    log += 1;
                }
                Some(log)
            }
            pub const fn ilog(self, base: Self) -> u32 {
                match self.checked_ilog(base) { Some(v) => v, None => crate::intrinsics::abort() }
            }
            pub const fn checked_ilog10(self) -> Option<u32> {
                if self <= 0 { None } else { self.checked_ilog(10) }
            }
            pub const fn ilog10(self) -> u32 {
                match self.checked_ilog10() { Some(v) => v, None => crate::intrinsics::abort() }
            }
            pub const fn overflowing_add(self, rhs: Self) -> (Self, bool) { add_with_overflow(self, rhs) }
            pub const fn overflowing_sub(self, rhs: Self) -> (Self, bool) { sub_with_overflow(self, rhs) }
            pub const fn overflowing_mul(self, rhs: Self) -> (Self, bool) { mul_with_overflow(self, rhs) }
            pub const fn overflowing_shl(self, rhs: u32) -> (Self, bool) { (self.wrapping_shl(rhs), rhs >= $bits) }
            pub const fn overflowing_shr(self, rhs: u32) -> (Self, bool) { (self.wrapping_shr(rhs), rhs >= $bits) }
            pub const fn clamp(self, min: Self, max: Self) -> Self {
                if self < min { min } else if self > max { max } else { self }
            }
            pub const fn max(self, other: Self) -> Self { if self < other { other } else { self } }
            pub const fn min(self, other: Self) -> Self { if self < other { self } else { other } }
            pub const fn pow(self, mut exp: u32) -> Self {
                let mut base = self;
                let mut acc: Self = 1;
                while exp > 0 {
                    if (exp & 1) == 1 { acc = acc * base; }
                    exp >>= 1;
                    if exp > 0 { base = base * base; }
                }
                acc
            }
            pub const fn checked_pow(self, mut exp: u32) -> Option<Self> {
                let mut base = self;
                let mut acc: Self = 1;
                while exp > 0 {
                    if (exp & 1) == 1 {
                        let Some(next) = acc.checked_mul(base) else { return None; };
                        acc = next;
                    }
                    exp >>= 1;
                    if exp > 0 {
                        let Some(next) = base.checked_mul(base) else { return None; };
                        base = next;
                    }
                }
                Some(acc)
            }
            pub const fn wrapping_pow(self, mut exp: u32) -> Self {
                let mut base = self;
                let mut acc: Self = 1;
                while exp > 0 {
                    if (exp & 1) == 1 { acc = acc.wrapping_mul(base); }
                    exp >>= 1;
                    if exp > 0 { base = base.wrapping_mul(base); }
                }
                acc
            }
            pub const fn saturating_pow(self, exp: u32) -> Self {
                match self.checked_pow(exp) {
                    Some(v) => v,
                    None if $signed && self < 0 && (exp & 1) == 1 => Self::MIN,
                    None => Self::MAX,
                }
            }
        }
    };
}

macro_rules! unsigned {
    ($t:ty, $bits:expr) => {
        integer!($t, $bits, false);
        impl $t {
            pub const fn is_power_of_two(self) -> bool { self != 0 && (self & self.wrapping_sub(1)) == 0 }
            pub const fn checked_next_power_of_two(self) -> Option<Self> {
                if self <= 1 { return Some(1); }
                let mut n: Self = 1;
                while n < self {
                    if n > Self::MAX / 2 { return None; }
                    n *= 2;
                }
                Some(n)
            }
            pub const fn wrapping_next_power_of_two(self) -> Self {
                match self.checked_next_power_of_two() { Some(v) => v, None => 0 }
            }
            pub const fn next_power_of_two(self) -> Self {
                if self <= 1 { return 1; }
                let mut n: Self = 1;
                while n < self {
                    if n > Self::MAX / 2 { crate::intrinsics::abort(); }
                    n = n * 2;
                }
                n
            }
            pub const fn abs_diff(self, other: Self) -> Self {
                if self >= other { self - other } else { other - self }
            }
            /// Division rounded towards positive infinity.
            pub const fn div_ceil(self, rhs: Self) -> Self {
                let quotient = self / rhs;
                if self % rhs != 0 { quotient + 1 } else { quotient }
            }
            /// Whether `self` is a whole multiple of `rhs`. Only zero is a multiple of zero.
            pub const fn is_multiple_of(self, rhs: Self) -> bool { if rhs == 0 { self == 0 } else { self % rhs == 0 } }
            /// The smallest multiple of `rhs` that is not below `self`. Overflow is a panic.
            pub const fn next_multiple_of(self, rhs: Self) -> Self {
                let rest = self % rhs;
                if rest == 0 { self } else { self + (rhs - rest) }
            }
            pub const fn checked_next_multiple_of(self, rhs: Self) -> Option<Self> {
                if rhs == 0 { return None; }
                let rest = self % rhs;
                if rest == 0 { Some(self) } else { self.checked_add(rhs - rest) }
            }
            pub const fn overflowing_neg(self) -> (Self, bool) { (self.wrapping_neg(), self != 0) }
            pub const fn checked_neg(self) -> Option<Self> { if self == 0 { Some(0) } else { None } }
            pub const fn checked_div(self, rhs: Self) -> Option<Self> { if rhs == 0 { None } else { Some(self / rhs) } }
            pub const fn checked_rem(self, rhs: Self) -> Option<Self> { if rhs == 0 { None } else { Some(self % rhs) } }
            pub const fn div_euclid(self, rhs: Self) -> Self { self / rhs }
            pub const fn rem_euclid(self, rhs: Self) -> Self { self % rhs }
            pub const fn checked_div_euclid(self, rhs: Self) -> Option<Self> { self.checked_div(rhs) }
            pub const fn checked_rem_euclid(self, rhs: Self) -> Option<Self> { self.checked_rem(rhs) }
            pub const fn wrapping_div_euclid(self, rhs: Self) -> Self { self / rhs }
            pub const fn wrapping_rem_euclid(self, rhs: Self) -> Self { self % rhs }
            pub const fn wrapping_div(self, rhs: Self) -> Self { self / rhs }
            pub const fn wrapping_rem(self, rhs: Self) -> Self { self % rhs }
        }
    };
}

unsigned!(u8, 8); unsigned!(u16, 16); unsigned!(u32, 32);
unsigned!(u64, 64); unsigned!(u128, 128); unsigned!(usize, crate::mem::size_of::<usize>() as u32 * 8);
integer!(i8, 8, true); integer!(i16, 16, true); integer!(i32, 32, true);
integer!(i64, 64, true); integer!(i128, 128, true); integer!(isize, crate::mem::size_of::<isize>() as u32 * 8, true);

macro_rules! bytes {
    ($t:ty, $u:ty, $n:expr) => {
        impl $t {
            pub const fn to_le_bytes(self) -> [u8; $n] {
                let mut out = [0; $n];
                let v = self as $u;
                let mut i = 0;
                while i < $n { out[i] = (v >> ((i * 8) as $u)) as u8; i += 1; }
                out
            }
            pub const fn to_ne_bytes(self) -> [u8; $n] {
                self.to_le_bytes()
            }
            pub const fn from_le_bytes(bytes: [u8; $n]) -> Self {
                let mut value: $u = 0;
                let mut i = 0;
                while i < $n { value |= (bytes[i] as $u) << ((i * 8) as $u); i += 1; }
                value as Self
            }
            /// Reverses the byte order.
            pub const fn swap_bytes(self) -> Self {
                let bytes = self.to_le_bytes();
                let mut out = [0; $n];
                let mut i = 0;
                while i < $n { out[i] = bytes[$n - 1 - i]; i += 1; }
                Self::from_le_bytes(out)
            }
            pub const fn to_be_bytes(self) -> [u8; $n] { self.swap_bytes().to_le_bytes() }
            pub const fn from_be_bytes(bytes: [u8; $n]) -> Self { Self::from_le_bytes(bytes).swap_bytes() }
            pub const fn from_ne_bytes(bytes: [u8; $n]) -> Self {
                Self::from_le_bytes(bytes)
            }
            pub const fn rotate_left(self, n: u32) -> Self {
                let u = self as $u;
                let bits = Self::BITS;
                let n = n & (bits - 1);
                (if n == 0 { u } else { (u << (n as $u)) | (u >> ((bits - n) as $u)) }) as Self
            }
            pub const fn rotate_right(self, n: u32) -> Self {
                let u = self as $u;
                let bits = Self::BITS;
                let n = n & (bits - 1);
                (if n == 0 { u } else { (u >> (n as $u)) | (u << ((bits - n) as $u)) }) as Self
            }
            pub const fn reverse_bits(self) -> Self {
                let mut x = self.swap_bytes() as $u;
                let mut mask1: $u = 0;
                let mut mask2: $u = 0;
                let mut mask4: $u = 0;
                let mut i = 0;
                while i < $n {
                    let shift = (i * 8) as $u;
                    mask1 |= (0x55 as $u) << shift;
                    mask2 |= (0x33 as $u) << shift;
                    mask4 |= (0x0f as $u) << shift;
                    i += 1;
                }
                x = ((x >> 1) & mask1) | ((x & mask1) << 1);
                x = ((x >> 2) & mask2) | ((x & mask2) << 2);
                x = ((x >> 4) & mask4) | ((x & mask4) << 4);
                x as Self
            }
        }
    };
}
bytes!(u8, u8, 1); bytes!(u16, u16, 2); bytes!(u32, u32, 4); bytes!(u64, u64, 8); bytes!(u128, u128, 16);
bytes!(i8, u8, 1); bytes!(i16, u16, 2); bytes!(i32, u32, 4); bytes!(i64, u64, 8); bytes!(i128, u128, 16);
bytes!(usize, usize, crate::mem::size_of::<usize>());
bytes!(isize, usize, crate::mem::size_of::<isize>());

macro_rules! nonzero {
    ($name:ident, $t:ty) => {
        #[repr(transparent)]
        pub struct $name($t);
        impl $name {
            pub const fn new(n: $t) -> Option<Self> { if n == 0 { None } else { Some(Self(n)) } }
            pub const unsafe fn new_unchecked(n: $t) -> Self { Self(n) }
            pub const fn get(self) -> $t { self.0 }
            pub const fn leading_zeros(self) -> u32 { self.0.leading_zeros() }
            pub const fn trailing_zeros(self) -> u32 { self.0.trailing_zeros() }
        }
    };
}
nonzero!(NonZeroU8, u8); nonzero!(NonZeroU16, u16); nonzero!(NonZeroU32, u32);
nonzero!(NonZeroU64, u64); nonzero!(NonZeroU128, u128); nonzero!(NonZeroUsize, usize);

macro_rules! signed_abs {
    ($($t:ty, $u:ty);*) => {$(
        impl $t {
            /// Absolute value as the unsigned type, where `MIN` fits.
            pub const fn unsigned_abs(self) -> $u { if self < 0 { (0 as $u).wrapping_sub(self as $u) } else { self as $u } }
            pub const fn abs_diff(self, other: Self) -> $u {
                if self >= other { (self as $u).wrapping_sub(other as $u) } else { (other as $u).wrapping_sub(self as $u) }
            }
            /// Absolute value. `MIN` has none and wraps to itself.
            pub const fn abs(self) -> Self { if self < 0 { (0 as $t).wrapping_sub(self) } else { self } }
            pub const fn wrapping_abs(self) -> Self { if self < 0 { self.wrapping_neg() } else { self } }
            pub const fn checked_abs(self) -> Option<Self> {
                if self == Self::MIN { None } else if self < 0 { Some(-self) } else { Some(self) }
            }
            pub const fn overflowing_neg(self) -> (Self, bool) {
                if self == Self::MIN { (Self::MIN, true) } else { (-self, false) }
            }
            pub const fn checked_neg(self) -> Option<Self> {
                if self == Self::MIN { None } else { Some(-self) }
            }
            pub const fn checked_div(self, rhs: Self) -> Option<Self> {
                if rhs == 0 || (self == Self::MIN && rhs == -1) { None } else { Some(self / rhs) }
            }
            pub const fn checked_rem(self, rhs: Self) -> Option<Self> {
                if rhs == 0 || (self == Self::MIN && rhs == -1) { None } else { Some(self % rhs) }
            }
            pub const fn signum(self) -> Self {
                if self < 0 { -1 } else if self > 0 { 1 } else { 0 }
            }
            pub const fn is_positive(self) -> bool { self > 0 }
            pub const fn is_negative(self) -> bool { self < 0 }
            pub const fn wrapping_div(self, rhs: Self) -> Self {
                if self == Self::MIN && rhs == -1 { Self::MIN } else { self / rhs }
            }
            pub const fn wrapping_rem(self, rhs: Self) -> Self {
                if self == Self::MIN && rhs == -1 { 0 } else { self % rhs }
            }
            pub const fn wrapping_div_euclid(self, rhs: Self) -> Self {
                let q = self.wrapping_div(rhs);
                if self.wrapping_rem(rhs) < 0 {
                    if rhs > 0 { q.wrapping_sub(1) } else { q.wrapping_add(1) }
                } else {
                    q
                }
            }
            pub const fn wrapping_rem_euclid(self, rhs: Self) -> Self {
                let r = self.wrapping_rem(rhs);
                if r < 0 {
                    if rhs < 0 { r.wrapping_sub(rhs) } else { r.wrapping_add(rhs) }
                } else {
                    r
                }
            }
            pub const fn checked_div_euclid(self, rhs: Self) -> Option<Self> {
                if rhs == 0 || (self == Self::MIN && rhs == -1) {
                    None
                } else {
                    Some(self.wrapping_div_euclid(rhs))
                }
            }
            pub const fn checked_rem_euclid(self, rhs: Self) -> Option<Self> {
                if rhs == 0 || (self == Self::MIN && rhs == -1) {
                    None
                } else {
                    Some(self.wrapping_rem_euclid(rhs))
                }
            }
            pub const fn div_euclid(self, rhs: Self) -> Self {
                let q = self / rhs;
                if self % rhs < 0 {
                    if rhs > 0 { q - 1 } else { q + 1 }
                } else {
                    q
                }
            }
            pub const fn rem_euclid(self, rhs: Self) -> Self {
                let r = self % rhs;
                if r < 0 {
                    if rhs < 0 { r - rhs } else { r + rhs }
                } else {
                    r
                }
            }
        }
    )*};
}
signed_abs!(i8, u8; i16, u16; i32, u32; i64, u64; i128, u128; isize, usize);

/// Why `from_str_radix` refused its input.
pub enum IntErrorKind { Empty, InvalidDigit, PosOverflow, NegOverflow, Zero }
impl crate::marker::Copy for IntErrorKind {}
impl crate::clone::Clone for IntErrorKind { fn clone(&self) -> Self { *self } }

#[derive(Copy, Clone, PartialEq, Eq)]
enum FloatErrorKind {
    Empty,
    Invalid,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ParseFloatError {
    kind: FloatErrorKind,
}
impl ParseFloatError {
    const fn text(&self) -> &'static str {
        match self.kind {
            FloatErrorKind::Empty => "cannot parse float from empty string",
            FloatErrorKind::Invalid => "invalid float literal",
        }
    }
}
impl crate::fmt::Display for ParseFloatError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { f.write_str(self.text()) }
}
impl crate::fmt::Debug for ParseFloatError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.debug_struct("ParseFloatError").field("kind", &self.text()).finish()
    }
}

pub(crate) fn parse_f64(s: &str) -> crate::result::Result<f64, ParseFloatError> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return crate::result::Result::Err(ParseFloatError { kind: FloatErrorKind::Empty });
    }
    let mut i: usize = 0;
    let mut neg = false;
    if bytes[i] == b'-' {
        neg = true;
        i += 1;
    } else if bytes[i] == b'+' {
        i += 1;
    }
    if i >= bytes.len() {
        return crate::result::Result::Err(ParseFloatError { kind: FloatErrorKind::Invalid });
    }
    let rest = unsafe { crate::str::from_utf8_unchecked(&bytes[i..]) };
    if rest.eq_ignore_ascii_case("inf") || rest.eq_ignore_ascii_case("infinity") {
        return crate::result::Result::Ok(if neg { f64::NEG_INFINITY } else { f64::INFINITY });
    }
    if rest.eq_ignore_ascii_case("nan") {
        return crate::result::Result::Ok(if neg { -f64::NAN } else { f64::NAN });
    }

    let mut mant: u64 = 0;
    let mut mant_digits: usize = 0;
    let mut has_digits = false;
    let mut exp10: i32 = 0;

    while i < bytes.len() {
        let b = bytes[i];
        if b < b'0' {
            break;
        }
        if b > b'9' {
            break;
        }
        has_digits = true;
        let d = (b - b'0') as u64;
        if mant_digits < 19 {
            if mant != 0 || d != 0 {
                mant = mant * 10 + d;
                mant_digits += 1;
            }
        } else {
            exp10 += 1;
        }
        i += 1;
    }

    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() {
            let b = bytes[i];
            if b < b'0' {
                break;
            }
            if b > b'9' {
                break;
            }
            has_digits = true;
            let d = (b - b'0') as u64;
            if mant_digits < 19 {
                if mant != 0 || d != 0 {
                    mant = mant * 10 + d;
                    mant_digits += 1;
                }
                exp10 -= 1;
            }
            i += 1;
        }
    }

    if !has_digits {
        return crate::result::Result::Err(ParseFloatError { kind: FloatErrorKind::Invalid });
    }

    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        let mut exp_neg = false;
        if i < bytes.len() {
            if bytes[i] == b'-' {
                exp_neg = true;
                i += 1;
            } else if bytes[i] == b'+' {
                i += 1;
            }
        }
        if i >= bytes.len() || bytes[i] < b'0' || bytes[i] > b'9' {
            return crate::result::Result::Err(ParseFloatError { kind: FloatErrorKind::Invalid });
        }
        let mut exp_val: i32 = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b < b'0' {
                break;
            }
            if b > b'9' {
                break;
            }
            let d = (b - b'0') as i32;
            if exp_val < 10000 {
                exp_val = exp_val * 10 + d;
            }
            i += 1;
        }
        if exp_neg {
            exp10 -= exp_val;
        } else {
            exp10 += exp_val;
        }
    }

    if i != bytes.len() {
        return crate::result::Result::Err(ParseFloatError { kind: FloatErrorKind::Invalid });
    }

    if mant == 0 {
        return crate::result::Result::Ok(if neg { -0.0 } else { 0.0 });
    }

    let mut val = mant as f64;
    if exp10 > 308 {
        val = f64::INFINITY;
    } else if exp10 < -340 {
        val = 0.0;
    } else {
        let mut e = exp10;
        let mut s1 = 0usize;
        while s1 < 24 {
            if e < 16 {
                break;
            }
            val *= 1e16;
            e -= 16;
            s1 += 1;
        }
        let mut s2 = 0usize;
        while s2 < 16 {
            if e <= 0 {
                break;
            }
            val *= 10.0;
            e -= 1;
            s2 += 1;
        }
        let mut s3 = 0usize;
        while s3 < 24 {
            if e > -16 {
                break;
            }
            val /= 1e16;
            e += 16;
            s3 += 1;
        }
        let mut s4 = 0usize;
        while s4 < 16 {
            if e >= 0 {
                break;
            }
            val /= 10.0;
            e += 1;
            s4 += 1;
        }
    }

    crate::result::Result::Ok(if neg { -val } else { val })
}

pub struct ParseIntError { kind: IntErrorKind }
impl ParseIntError {
    pub const fn kind(&self) -> &IntErrorKind { &self.kind }
    const fn text(&self) -> &'static str {
        match self.kind {
            IntErrorKind::Empty => "cannot parse integer from empty string",
            IntErrorKind::InvalidDigit => "invalid digit found in string",
            IntErrorKind::PosOverflow => "number too large to fit in target type",
            IntErrorKind::NegOverflow => "number too small to fit in target type",
            IntErrorKind::Zero => "number would be zero for non-zero type",
        }
    }
}
impl crate::marker::Copy for ParseIntError {}
impl crate::clone::Clone for ParseIntError { fn clone(&self) -> Self { *self } }
impl crate::fmt::Display for ParseIntError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { f.write_str(self.text()) }
}
impl crate::fmt::Debug for ParseIntError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { f.write_str(self.text()) }
}

impl u8 {
    pub const fn to_ascii_uppercase(&self) -> u8 { if *self >= b'a' && *self <= b'z' { *self - 32 } else { *self } }
    pub const fn to_ascii_lowercase(&self) -> u8 { if *self >= b'A' && *self <= b'Z' { *self + 32 } else { *self } }
    pub const fn eq_ignore_ascii_case(&self, other: &u8) -> bool { self.to_ascii_lowercase() == other.to_ascii_lowercase() }
    pub const fn is_ascii(&self) -> bool { *self <= 0x7f }
    pub const fn is_ascii_alphabetic(&self) -> bool { (*self >= b'A' && *self <= b'Z') || (*self >= b'a' && *self <= b'z') }
    pub const fn is_ascii_uppercase(&self) -> bool { *self >= b'A' && *self <= b'Z' }
    pub const fn is_ascii_lowercase(&self) -> bool { *self >= b'a' && *self <= b'z' }
    pub const fn is_ascii_alphanumeric(&self) -> bool { self.is_ascii_alphabetic() || self.is_ascii_digit() }
    pub const fn is_ascii_hexdigit(&self) -> bool {
        self.is_ascii_digit() || (*self >= b'a' && *self <= b'f') || (*self >= b'A' && *self <= b'F')
    }
    pub const fn is_ascii_whitespace(&self) -> bool {
        *self == b' ' || *self == b'\t' || *self == b'\n' || *self == b'\r' || *self == 0x0c
    }
    pub const fn is_ascii_control(&self) -> bool { *self <= 0x1f || *self == 0x7f }
    pub const fn is_ascii_graphic(&self) -> bool { *self >= 0x21 && *self <= 0x7e }
    pub const fn is_ascii_punctuation(&self) -> bool { self.is_ascii_graphic() && !self.is_ascii_alphanumeric() }
}

impl bool {
    /// `Some(value)` when true.
    pub fn then_some<T>(self, value: T) -> Option<T> { if self { Some(value) } else { None } }
    /// `Some(f())` when true; `f` is not called otherwise.
    pub fn then<T, F: crate::ops::FnOnce() -> T>(self, f: F) -> Option<T> { if self { Some(f()) } else { None } }
}

impl u8 {
    pub const fn is_ascii_digit(&self) -> bool { *self >= b'0' && *self <= b'9' }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FpCategory {
    Nan,
    Infinite,
    Zero,
    Subnormal,
    Normal,
}

macro_rules! nonzero_traits {
    ($($name:ident, $t:ty);*) => {$(
        impl $name {
            /// The smallest value: one.
            pub const MIN: Self = Self(1);
            /// The sum, held at the largest value when it would not fit.
            pub const fn saturating_add(self, other: $t) -> Self { Self(self.0.saturating_add(other)) }
        }
        impl crate::marker::Copy for $name {}
        impl crate::clone::Clone for $name { fn clone(&self) -> Self { *self } }
        impl crate::cmp::PartialEq for $name { fn eq(&self, other: &Self) -> bool { self.0 == other.0 } }
        impl crate::cmp::Eq for $name {}
        impl crate::cmp::PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<crate::cmp::Ordering> { self.0.partial_cmp(&other.0) }
        }
        impl crate::cmp::Ord for $name {
            fn cmp(&self, other: &Self) -> crate::cmp::Ordering { self.0.cmp(&other.0) }
        }
        impl crate::hash::Hash for $name {
            fn hash<H: crate::hash::Hasher>(&self, state: &mut H) { self.0.hash(state) }
        }
        impl crate::fmt::Debug for $name {
            fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { crate::fmt::Debug::fmt(&self.0, f) }
        }
        impl crate::fmt::Display for $name {
            fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { crate::fmt::Display::fmt(&self.0, f) }
        }
        impl crate::fmt::LowerHex for $name {
            fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { crate::fmt::LowerHex::fmt(&self.0, f) }
        }
        impl crate::convert::From<$name> for $t { fn from(value: $name) -> $t { value.0 } }
        impl crate::convert::TryFrom<$t> for $name {
            type Error = crate::convert::TryFromIntError;
            fn try_from(value: $t) -> crate::result::Result<Self, Self::Error> {
                match Self::new(value) {
                    Some(nonzero) => crate::result::Result::Ok(nonzero),
                    None => crate::result::Result::Err(crate::convert::TryFromIntError::new()),
                }
            }
        }
    )*};
}
nonzero_traits!(NonZeroU8, u8; NonZeroU16, u16; NonZeroU32, u32; NonZeroU64, u64; NonZeroU128, u128; NonZeroUsize, usize);

macro_rules! float_impl {
    ($t:ty, $u:ty, $bytes:expr, $sign_mask:expr, $exp_mask:expr, $mant_mask:expr, $pi:expr) => {
        impl $t {
            pub const INFINITY: Self = 1.0 / 0.0;
            pub const NEG_INFINITY: Self = -1.0 / 0.0;
            pub const NAN: Self = 0.0 / 0.0;

            #[inline(always)]
            pub const fn to_bits(self) -> $u {
                unsafe { crate::intrinsics::transmute(self) }
            }
            #[inline(always)]
            pub const fn from_bits(v: $u) -> Self {
                unsafe { crate::intrinsics::transmute(v) }
            }
            #[inline(always)]
            pub const fn is_nan(self) -> bool {
                self != self
            }
            #[inline(always)]
            pub const fn is_infinite(self) -> bool {
                (self.to_bits() & !$sign_mask) == $exp_mask
            }
            #[inline(always)]
            pub const fn is_finite(self) -> bool {
                (self.to_bits() & $exp_mask) != $exp_mask
            }
            #[inline(always)]
            pub const fn is_sign_positive(self) -> bool {
                (self.to_bits() & $sign_mask) == 0
            }
            #[inline(always)]
            pub const fn is_sign_negative(self) -> bool {
                (self.to_bits() & $sign_mask) != 0
            }
            #[inline]
            pub const fn classify(self) -> FpCategory {
                let b = self.to_bits();
                let exp = b & $exp_mask;
                let mant = b & $mant_mask;
                if exp == $exp_mask {
                    if mant == 0 { FpCategory::Infinite } else { FpCategory::Nan }
                } else if exp == 0 {
                    if mant == 0 { FpCategory::Zero } else { FpCategory::Subnormal }
                } else {
                    FpCategory::Normal
                }
            }
            #[inline(always)]
            pub const fn is_normal(self) -> bool {
                let exp = self.to_bits() & $exp_mask;
                exp != 0 && exp != $exp_mask
            }
            #[inline(always)]
            pub const fn is_subnormal(self) -> bool {
                let b = self.to_bits();
                (b & $exp_mask) == 0 && (b & $mant_mask) != 0
            }
            #[inline(always)]
            pub const fn abs(self) -> Self {
                Self::from_bits(self.to_bits() & !$sign_mask)
            }
            #[inline(always)]
            pub const fn copysign(self, sign: Self) -> Self {
                Self::from_bits((self.to_bits() & !$sign_mask) | (sign.to_bits() & $sign_mask))
            }
            #[inline(always)]
            pub const fn signum(self) -> Self {
                if self.is_nan() { Self::NAN } else { (1.0 as Self).copysign(self) }
            }
            #[inline(always)]
            pub const fn recip(self) -> Self {
                1.0 / self
            }
            #[inline]
            pub const fn min(self, other: Self) -> Self {
                if self.is_nan() { other } else if other.is_nan() { self } else if self < other { self } else { other }
            }
            #[inline]
            pub const fn max(self, other: Self) -> Self {
                if self.is_nan() { other } else if other.is_nan() { self } else if self > other { self } else { other }
            }
            #[inline]
            pub const fn clamp(self, min: Self, max: Self) -> Self {
                if min > max { crate::intrinsics::abort(); }
                let mut x = self;
                if x < min { x = min; }
                if x > max { x = max; }
                x
            }
            #[inline(always)]
            pub const fn to_degrees(self) -> Self {
                self * (180.0 / $pi)
            }
            #[inline(always)]
            pub const fn to_radians(self) -> Self {
                self * ($pi / 180.0)
            }
            #[inline(always)]
            pub const fn to_be_bytes(self) -> [u8; $bytes] {
                self.to_bits().to_be_bytes()
            }
            #[inline(always)]
            pub const fn to_le_bytes(self) -> [u8; $bytes] {
                self.to_bits().to_le_bytes()
            }
            #[inline(always)]
            pub const fn to_ne_bytes(self) -> [u8; $bytes] {
                self.to_bits().to_ne_bytes()
            }
            #[inline(always)]
            pub const fn from_be_bytes(bytes: [u8; $bytes]) -> Self {
                Self::from_bits(<$u>::from_be_bytes(bytes))
            }
            #[inline(always)]
            pub const fn from_le_bytes(bytes: [u8; $bytes]) -> Self {
                Self::from_bits(<$u>::from_le_bytes(bytes))
            }
            #[inline(always)]
            pub const fn from_ne_bytes(bytes: [u8; $bytes]) -> Self {
                Self::from_bits(<$u>::from_ne_bytes(bytes))
            }
        }
    };
}

float_impl!(f32, u32, 4, 0x8000_0000_u32, 0x7f80_0000_u32, 0x007f_ffff_u32, 3.14159265358979323846_f32);
float_impl!(f64, u64, 8, 0x8000_0000_0000_0000_u64, 0x7ff0_0000_0000_0000_u64, 0x000f_ffff_ffff_ffff_u64, 3.14159265358979323846_f64);

impl f32 {
    pub const RADIX: u32 = 2;
    pub const MANTISSA_DIGITS: u32 = 24;
    pub const DIGITS: u32 = 6;
    pub const EPSILON: f32 = 1.19209290e-07_f32;
    pub const MIN: f32 = -3.40282347e+38_f32;
    pub const MIN_POSITIVE: f32 = 1.17549435e-38_f32;
    pub const MAX: f32 = 3.40282347e+38_f32;
    pub const MIN_EXP: i32 = -125;
    pub const MAX_EXP: i32 = 128;
    pub const MIN_10_EXP: i32 = -37;
    pub const MAX_10_EXP: i32 = 38;
}

impl f64 {
    pub const RADIX: u32 = 2;
    pub const MANTISSA_DIGITS: u32 = 53;
    pub const DIGITS: u32 = 15;
    pub const EPSILON: f64 = 2.2204460492503131e-16_f64;
    pub const MIN: f64 = -1.7976931348623157e+308_f64;
    pub const MIN_POSITIVE: f64 = 2.2250738585072014e-308_f64;
    pub const MAX: f64 = 1.7976931348623157e+308_f64;
    pub const MIN_EXP: i32 = -1021;
    pub const MAX_EXP: i32 = 1024;
    pub const MIN_10_EXP: i32 = -307;
    pub const MAX_10_EXP: i32 = 308;
}

#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Wrapping<T>(pub T);

impl<T: crate::fmt::Debug> crate::fmt::Debug for Wrapping<T> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        self.0.fmt(f)
    }
}

macro_rules! wrapping_impl {
    ($($t:ty)*) => {$(
        impl crate::ops::Add for Wrapping<$t> {
            type Output = Self;
            #[inline]
            fn add(self, rhs: Self) -> Self { Wrapping(self.0.wrapping_add(rhs.0)) }
        }
        impl crate::ops::AddAssign for Wrapping<$t> {
            #[inline]
            fn add_assign(&mut self, rhs: Self) { *self = *self + rhs; }
        }
        impl crate::ops::Sub for Wrapping<$t> {
            type Output = Self;
            #[inline]
            fn sub(self, rhs: Self) -> Self { Wrapping(self.0.wrapping_sub(rhs.0)) }
        }
        impl crate::ops::SubAssign for Wrapping<$t> {
            #[inline]
            fn sub_assign(&mut self, rhs: Self) { *self = *self - rhs; }
        }
        impl crate::ops::Mul for Wrapping<$t> {
            type Output = Self;
            #[inline]
            fn mul(self, rhs: Self) -> Self { Wrapping(self.0.wrapping_mul(rhs.0)) }
        }
        impl crate::ops::MulAssign for Wrapping<$t> {
            #[inline]
            fn mul_assign(&mut self, rhs: Self) { *self = *self * rhs; }
        }
        impl crate::ops::Neg for Wrapping<$t> {
            type Output = Self;
            #[inline]
            fn neg(self) -> Self { Wrapping(self.0.wrapping_neg()) }
        }
        impl crate::ops::Not for Wrapping<$t> {
            type Output = Self;
            #[inline]
            fn not(self) -> Self { Wrapping(!self.0) }
        }
        impl crate::ops::BitAnd for Wrapping<$t> {
            type Output = Self;
            #[inline]
            fn bitand(self, rhs: Self) -> Self { Wrapping(self.0 & rhs.0) }
        }
        impl crate::ops::BitAndAssign for Wrapping<$t> {
            #[inline]
            fn bitand_assign(&mut self, rhs: Self) { *self = *self & rhs; }
        }
        impl crate::ops::BitOr for Wrapping<$t> {
            type Output = Self;
            #[inline]
            fn bitor(self, rhs: Self) -> Self { Wrapping(self.0 | rhs.0) }
        }
        impl crate::ops::BitOrAssign for Wrapping<$t> {
            #[inline]
            fn bitor_assign(&mut self, rhs: Self) { *self = *self | rhs; }
        }
        impl crate::ops::BitXor for Wrapping<$t> {
            type Output = Self;
            #[inline]
            fn bitxor(self, rhs: Self) -> Self { Wrapping(self.0 ^ rhs.0) }
        }
        impl crate::ops::BitXorAssign for Wrapping<$t> {
            #[inline]
            fn bitxor_assign(&mut self, rhs: Self) { *self = *self ^ rhs; }
        }
        impl crate::ops::Shl<usize> for Wrapping<$t> {
            type Output = Self;
            #[inline]
            fn shl(self, rhs: usize) -> Self { Wrapping(self.0.wrapping_shl(rhs as u32)) }
        }
        impl crate::ops::ShlAssign<usize> for Wrapping<$t> {
            #[inline]
            fn shl_assign(&mut self, rhs: usize) { *self = *self << rhs; }
        }
        impl crate::ops::Shr<usize> for Wrapping<$t> {
            type Output = Self;
            #[inline]
            fn shr(self, rhs: usize) -> Self { Wrapping(self.0.wrapping_shr(rhs as u32)) }
        }
        impl crate::ops::ShrAssign<usize> for Wrapping<$t> {
            #[inline]
            fn shr_assign(&mut self, rhs: usize) { *self = *self >> rhs; }
        }
    )*};
}
wrapping_impl!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);
