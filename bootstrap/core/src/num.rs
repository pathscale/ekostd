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
            pub const fn overflowing_add(self, rhs: Self) -> (Self, bool) { add_with_overflow(self, rhs) }
            pub const fn clamp(self, min: Self, max: Self) -> Self {
                if self < min { min } else if self > max { max } else { self }
            }
            pub const fn max(self, other: Self) -> Self { if self < other { other } else { self } }
            pub const fn min(self, other: Self) -> Self { if self < other { self } else { other } }
        }
    };
}

macro_rules! unsigned {
    ($t:ty, $bits:expr) => {
        integer!($t, $bits, false);
        impl $t {
            pub const fn is_power_of_two(self) -> bool { self != 0 && (self & self.wrapping_sub(1)) == 0 }
            pub const fn next_power_of_two(self) -> Self {
                if self <= 1 { return 1; }
                let mut n: Self = 1;
                while n < self {
                    if n > Self::MAX / 2 { crate::intrinsics::abort(); }
                    n = n * 2;
                }
                n
            }
            /// Reverses the byte order.
            pub const fn swap_bytes(self) -> Self {
                let (mut rest, mut out, mut i) = (self, 0 as $t, 0);
                while i < $bits / 8 {
                    out = out.wrapping_shl(8) | (rest & (0xff as $t));
                    rest = rest.wrapping_shr(8);
                    i += 1;
                }
                out
            }
            /// Division rounded towards positive infinity.
            pub const fn div_ceil(self, rhs: Self) -> Self {
                let quotient = self / rhs;
                if self % rhs != 0 { quotient + 1 } else { quotient }
            }
            pub const fn rotate_left(self, n: u32) -> Self {
                let n = n & ($bits - 1);
                if n == 0 { self } else { (self << (n as $t)) | (self >> (($bits - n) as $t)) }
            }
            pub const fn rotate_right(self, n: u32) -> Self {
                let n = n & ($bits - 1);
                if n == 0 { self } else { (self >> (n as $t)) | (self << (($bits - n) as $t)) }
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
            pub const fn count_ones(self) -> u32 { ctpop(self) }
            pub const fn leading_zeros(self) -> u32 { ctlz(self) }
            pub const fn trailing_zeros(self) -> u32 { cttz(self) }
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
            pub const fn from_ne_bytes(bytes: [u8; $n]) -> Self {
                Self::from_le_bytes(bytes)
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
            /// Absolute value. `MIN` has none and wraps to itself.
            pub const fn abs(self) -> Self { if self < 0 { (0 as $t).wrapping_sub(self) } else { self } }
        }
    )*};
}
signed_abs!(i8, u8; i16, u16; i32, u32; i64, u64; i128, u128; isize, usize);

/// Why `from_str_radix` refused its input.
pub enum IntErrorKind { Empty, InvalidDigit, PosOverflow, NegOverflow, Zero }
impl crate::marker::Copy for IntErrorKind {}
impl crate::clone::Clone for IntErrorKind { fn clone(&self) -> Self { *self } }

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
}
