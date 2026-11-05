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
                if !overflow { value } else if $signed && self < 0 { Self::MIN } else { Self::MAX }
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
        }
    };
}
nonzero!(NonZeroU8, u8); nonzero!(NonZeroU16, u16); nonzero!(NonZeroU32, u32);
nonzero!(NonZeroU64, u64); nonzero!(NonZeroU128, u128); nonzero!(NonZeroUsize, usize);
