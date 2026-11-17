use crate::option::Option::{self, None, Some};

impl<T, E> crate::result::Result<T, E> {
    #[inline]
    pub fn is_err_and<F: crate::ops::FnOnce(E) -> bool>(self, f: F) -> bool {
        match self {
            crate::result::Result::Ok(_) => false,
            crate::result::Result::Err(e) => f(e),
        }
    }
}

macro_rules! unsigned_ext {
    ($($t:ty, $s:ty);*) => {$(
        impl $t {
            /// Adds a signed value, wrapping around at the bounds of the type.
            pub const fn wrapping_add_signed(self, rhs: $s) -> Self { self.wrapping_add(rhs as Self) }
            /// Adds a signed value, with a flag telling whether the true sum left the range of the type.
            pub const fn overflowing_add_signed(self, rhs: $s) -> (Self, bool) {
                let (value, overflow) = self.overflowing_add(rhs as Self);
                (value, overflow ^ (rhs < 0))
            }
            /// Adds a signed value, stopping at 0 and at MAX.
            pub const fn saturating_add_signed(self, rhs: $s) -> Self {
                let (value, overflow) = self.overflowing_add_signed(rhs);
                if !overflow { value } else if rhs > 0 { Self::MAX } else { 0 }
            }
            /// Subtracts a signed value, wrapping around at the bounds of the type.
            pub const fn wrapping_sub_signed(self, rhs: $s) -> Self { self.wrapping_sub(rhs as Self) }
            /// Subtracts a signed value, with a flag telling whether the true difference left the range of the type.
            pub const fn overflowing_sub_signed(self, rhs: $s) -> (Self, bool) {
                let (value, overflow) = self.overflowing_sub(rhs as Self);
                (value, overflow ^ (rhs < 0))
            }
            /// Subtracts a signed value, or None when the true difference leaves the range of the type.
            pub const fn checked_sub_signed(self, rhs: $s) -> Option<Self> {
                let (value, overflow) = self.overflowing_sub_signed(rhs);
                if overflow { None } else { Some(value) }
            }
            /// Subtracts a signed value, stopping at 0 and at MAX.
            pub const fn saturating_sub_signed(self, rhs: $s) -> Self {
                let (value, overflow) = self.overflowing_sub_signed(rhs);
                if !overflow { value } else if rhs > 0 { 0 } else { Self::MAX }
            }
            /// The square root rounded down, computed one result bit at a time with no floating point.
            pub const fn isqrt(self) -> Self {
                if self == 0 { return 0; }
                let mut rest = self;
                let mut root: Self = 0;
                // The highest power of four that is not above the value.
                let mut bit: Self = (1 as Self) << (((Self::BITS - 1 - self.leading_zeros()) / 2 * 2) as Self);
                while bit != 0 {
                    if rest >= root + bit {
                        rest = rest - (root + bit);
                        root = (root >> (1 as Self)) + bit;
                    } else {
                        root = root >> (1 as Self);
                    }
                    bit = bit >> (2 as Self);
                }
                root
            }
            /// The average of the two values rounded down, without overflowing.
            pub const fn midpoint(self, rhs: Self) -> Self { (self & rhs) + ((self ^ rhs) >> (1 as Self)) }
            /// The same bits read as the signed type of the same width.
            pub const fn cast_signed(self) -> $s { self as $s }
            /// Shift left, or 0 when the amount is the width of the type or more.
            pub const fn unbounded_shl(self, rhs: u32) -> Self {
                if rhs < Self::BITS { self << (rhs as Self) } else { 0 }
            }
            /// Shift right, or 0 when the amount is the width of the type or more.
            pub const fn unbounded_shr(self, rhs: u32) -> Self {
                if rhs < Self::BITS { self >> (rhs as Self) } else { 0 }
            }
        }
    )*};
}
unsigned_ext!(u8, i8; u16, i16; u32, i32; u64, i64; u128, i128; usize, isize);

macro_rules! signed_ext {
    ($($t:ty, $u:ty);*) => {$(
        impl $t {
            /// Adds an unsigned value, with a flag telling whether the true sum left the range of the type.
            pub const fn overflowing_add_unsigned(self, rhs: $u) -> (Self, bool) {
                let rhs = rhs as Self;
                let (value, overflow) = self.overflowing_add(rhs);
                (value, overflow ^ (rhs < 0))
            }
            /// Adds an unsigned value, wrapping around at the bounds of the type.
            pub const fn wrapping_add_unsigned(self, rhs: $u) -> Self { self.wrapping_add(rhs as Self) }
            /// Adds an unsigned value, or None when the true sum leaves the range of the type.
            pub const fn checked_add_unsigned(self, rhs: $u) -> Option<Self> {
                let (value, overflow) = self.overflowing_add_unsigned(rhs);
                if overflow { None } else { Some(value) }
            }
            /// Adds an unsigned value, stopping at MAX.
            pub const fn saturating_add_unsigned(self, rhs: $u) -> Self {
                match self.checked_add_unsigned(rhs) { Some(value) => value, None => Self::MAX }
            }
            /// Subtracts an unsigned value, with a flag telling whether the true difference left the range of the type.
            pub const fn overflowing_sub_unsigned(self, rhs: $u) -> (Self, bool) {
                let rhs = rhs as Self;
                let (value, overflow) = self.overflowing_sub(rhs);
                (value, overflow ^ (rhs < 0))
            }
            /// Subtracts an unsigned value, wrapping around at the bounds of the type.
            pub const fn wrapping_sub_unsigned(self, rhs: $u) -> Self { self.wrapping_sub(rhs as Self) }
            /// Subtracts an unsigned value, or None when the true difference leaves the range of the type.
            pub const fn checked_sub_unsigned(self, rhs: $u) -> Option<Self> {
                let (value, overflow) = self.overflowing_sub_unsigned(rhs);
                if overflow { None } else { Some(value) }
            }
            /// Subtracts an unsigned value, stopping at MIN.
            pub const fn saturating_sub_unsigned(self, rhs: $u) -> Self {
                match self.checked_sub_unsigned(rhs) { Some(value) => value, None => Self::MIN }
            }
            /// The square root rounded down, or None for a negative value.
            pub const fn checked_isqrt(self) -> Option<Self> {
                if self < 0 { None } else { Some((self as $u).isqrt() as Self) }
            }
            /// The square root rounded down; panics for a negative value.
            pub fn isqrt(self) -> Self {
                match self.checked_isqrt() {
                    Some(root) => root,
                    None => panic!("argument of integer square root cannot be negative"),
                }
            }
            /// The average of the two values rounded towards zero, without overflowing.
            pub const fn midpoint(self, rhs: Self) -> Self {
                // Rounds towards negative infinity first, then moves an odd negative sum up by one.
                let t = ((self ^ rhs) >> (1 as Self)) + (self & rhs);
                t + ((if t < 0 { 1 as Self } else { 0 }) & (self ^ rhs))
            }
            /// The same bits read as the unsigned type of the same width.
            pub const fn cast_unsigned(self) -> $u { self as $u }
            /// Shift left, or 0 when the amount is the width of the type or more.
            pub const fn unbounded_shl(self, rhs: u32) -> Self {
                if rhs < Self::BITS { self << (rhs as Self) } else { 0 }
            }
            /// Arithmetic shift right; when the amount is the width of the type or more, 0 for a
            /// value that is not negative and -1 for a negative one.
            pub const fn unbounded_shr(self, rhs: u32) -> Self {
                if rhs < Self::BITS { self >> (rhs as Self) } else if self < 0 { -1 } else { 0 }
            }
        }
    )*};
}
signed_ext!(i8, u8; i16, u16; i32, u32; i64, u64; i128, u128; isize, usize);

impl f64 {
    /// Half of the sum, computed so that it cannot overflow; NaN if either value is NaN.
    pub const fn midpoint(self, other: f64) -> f64 {
        const LO: f64 = 2.2250738585072014e-308_f64 * 2.0;
        const HI: f64 = 1.7976931348623157e308_f64 / 2.0;
        let abs_a = self.abs();
        let abs_b = other.abs();
        if abs_a <= HI && abs_b <= HI {
            (self + other) / 2.0
        } else if abs_a < LO {
            self + other / 2.0
        } else if abs_b < LO {
            self / 2.0 + other
        } else {
            self / 2.0 + other / 2.0
        }
    }
}

impl f32 {
    /// Half of the sum, computed in f64 so that it cannot overflow; NaN if either value is NaN.
    pub const fn midpoint(self, other: f32) -> f32 { ((self as f64 + other as f64) / 2.0) as f32 }
}

impl crate::cmp::PartialEq for crate::num::IntErrorKind {
    fn eq(&self, other: &Self) -> bool { (*self as u8) == (*other as u8) }
}
impl crate::cmp::Eq for crate::num::IntErrorKind {}

impl crate::fmt::Debug for crate::num::IntErrorKind {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str(match self {
            crate::num::IntErrorKind::Empty => "Empty",
            crate::num::IntErrorKind::InvalidDigit => "InvalidDigit",
            crate::num::IntErrorKind::PosOverflow => "PosOverflow",
            crate::num::IntErrorKind::NegOverflow => "NegOverflow",
            crate::num::IntErrorKind::Zero => "Zero",
        })
    }
}

/// Two parse errors are equal when they have the same kind.
impl crate::cmp::PartialEq for crate::num::ParseIntError {
    fn eq(&self, other: &Self) -> bool { *self.kind() == *other.kind() }
}
impl crate::cmp::Eq for crate::num::ParseIntError {}

/// The error carries no data, so any two are equal.
impl crate::cmp::PartialEq for crate::convert::TryFromIntError {
    fn eq(&self, _other: &Self) -> bool { true }
}
impl crate::cmp::Eq for crate::convert::TryFromIntError {}
