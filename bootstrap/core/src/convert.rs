use crate::clone::Clone;
use crate::marker::{Copy, Sized};

pub const trait From<T>: Sized {
    #[lang = "from"]
    fn from(value: T) -> Self;
}

pub const trait Into<T>: Sized {
    fn into(self) -> T;
}

const impl<T> From<T> for T {
    fn from(value: T) -> T { value }
}

const impl<T, U> Into<U> for T
where
    U: [const] From<T>,
{
    fn into(self) -> U { U::from(self) }
}

pub trait AsRef<T: ?Sized>: crate::marker::PointeeSized {
    fn as_ref(&self) -> &T;
}
pub trait AsMut<T: ?Sized>: crate::marker::PointeeSized {
    fn as_mut(&mut self) -> &mut T;
}
// A reference converts the way its referent does.
impl<T: ?Sized + AsRef<U>, U: ?Sized> AsRef<U> for &T {
    fn as_ref(&self) -> &U { <T as AsRef<U>>::as_ref(*self) }
}
impl<T: ?Sized + AsRef<U>, U: ?Sized> AsRef<U> for &mut T {
    fn as_ref(&self) -> &U { <T as AsRef<U>>::as_ref(*self) }
}
impl<T: ?Sized + AsMut<U>, U: ?Sized> AsMut<U> for &mut T {
    fn as_mut(&mut self) -> &mut U { (*self).as_mut() }
}
impl<T> AsRef<[T]> for [T] {
    fn as_ref(&self) -> &[T] { self }
}
impl<T> AsMut<[T]> for [T] {
    fn as_mut(&mut self) -> &mut [T] { self }
}
impl AsRef<str> for str {
    fn as_ref(&self) -> &str { self }
}
impl AsMut<str> for str {
    fn as_mut(&mut self) -> &mut str { self }
}
pub trait TryFrom<T>: Sized {
    type Error;
    fn try_from(value: T) -> crate::result::Result<Self, Self::Error>;
}
pub trait TryInto<T>: Sized {
    type Error;
    fn try_into(self) -> crate::result::Result<T, Self::Error>;
}
impl<T, U: TryFrom<T>> TryInto<U> for T {
    type Error = U::Error;
    fn try_into(self) -> crate::result::Result<U, Self::Error> { U::try_from(self) }
}
impl<T, U: Into<T>> TryFrom<U> for T {
    type Error = Infallible;
    fn try_from(value: U) -> crate::result::Result<T, Infallible> { Ok(value.into()) }
}

pub enum Infallible {}

impl Copy for Infallible {}
const impl Clone for Infallible {
    fn clone(&self) -> Self { match *self {} }
}

pub struct TryFromIntError(());
impl Copy for TryFromIntError {}
const impl Clone for TryFromIntError {
    fn clone(&self) -> Self { *self }
}
impl crate::fmt::Debug for TryFromIntError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("TryFromIntError(())")
    }
}
impl crate::fmt::Display for TryFromIntError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("out of range integral type conversion attempted")
    }
}

macro_rules! impl_from_int {
    ($($small:ty => $large:ty),* $(,)?) => { $(
        const impl From<$small> for $large {
            #[inline(always)]
            fn from(small: $small) -> Self { small as Self }
        }
    )* };
}

impl_from_int!(
    u8 => u16, u8 => u32, u8 => u64, u8 => u128, u8 => usize,
    u8 => i16, u8 => i32, u8 => i64, u8 => i128, u8 => isize,
    u16 => u32, u16 => u64, u16 => u128, u16 => usize,
    u16 => i32, u16 => i64, u16 => i128,
    u32 => u64, u32 => u128,
    u32 => i64, u32 => i128,
    u64 => u128,
    i8 => i16, i8 => i32, i8 => i64, i8 => i128, i8 => isize,
    i16 => i32, i16 => i64, i16 => i128, i16 => isize,
    i32 => i64, i32 => i128,
    i64 => i128,
);

macro_rules! impl_try_from_upper_bounded {
    ($($source:ty => $target:ty),* $(,)?) => { $(
        impl TryFrom<$source> for $target {
            type Error = TryFromIntError;
            #[inline(always)]
            fn try_from(u: $source) -> crate::result::Result<Self, Self::Error> {
                if u > (Self::MAX as $source) {
                    crate::result::Result::Err(TryFromIntError(()))
                } else {
                    crate::result::Result::Ok(u as Self)
                }
            }
        }
    )* };
}

impl_try_from_upper_bounded!(
    u16 => u8, u32 => u8, u32 => u16, u64 => u8, u64 => u16, u64 => u32,
    u128 => u8, u128 => u16, u128 => u32, u128 => u64,
    usize => u8, usize => u16, usize => u32,
);

impl TryFrom<u32> for usize {
    type Error = TryFromIntError;
    #[inline(always)]
    fn try_from(u: u32) -> crate::result::Result<Self, Self::Error> {
        crate::result::Result::Ok(u as Self)
    }
}
impl TryFrom<u64> for usize {
    type Error = TryFromIntError;
    #[inline(always)]
    fn try_from(u: u64) -> crate::result::Result<Self, Self::Error> {
        crate::result::Result::Ok(u as Self)
    }
}
impl TryFrom<usize> for u64 {
    type Error = TryFromIntError;
    #[inline(always)]
    fn try_from(u: usize) -> crate::result::Result<Self, Self::Error> {
        crate::result::Result::Ok(u as Self)
    }
}
impl TryFrom<usize> for u128 {
    type Error = TryFromIntError;
    #[inline(always)]
    fn try_from(u: usize) -> crate::result::Result<Self, Self::Error> {
        crate::result::Result::Ok(u as Self)
    }
}
impl TryFrom<u128> for usize {
    type Error = TryFromIntError;
    #[inline(always)]
    fn try_from(u: u128) -> crate::result::Result<Self, Self::Error> {
        if u > (usize::MAX as u128) {
            crate::result::Result::Err(TryFromIntError(()))
        } else {
            crate::result::Result::Ok(u as Self)
        }
    }
}

impl<T, const N: usize> AsRef<[T]> for [T; N] {
    fn as_ref(&self) -> &[T] { self }
}
impl<T, const N: usize> AsMut<[T]> for [T; N] {
    fn as_mut(&mut self) -> &mut [T] { self }
}
impl AsRef<[u8]> for str {
    fn as_ref(&self) -> &[u8] { self.as_bytes() }
}

/// A slice whose length is not the array's length.
pub struct TryFromSliceError(());
impl Copy for TryFromSliceError {}
impl Clone for TryFromSliceError { fn clone(&self) -> Self { *self } }
impl crate::fmt::Debug for TryFromSliceError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { f.write_str("TryFromSliceError") }
}
impl crate::fmt::Display for TryFromSliceError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("could not convert slice to array")
    }
}
impl<T: Copy, const N: usize> TryFrom<&[T]> for [T; N] {
    type Error = TryFromSliceError;
    fn try_from(slice: &[T]) -> crate::result::Result<Self, TryFromSliceError> {
        if slice.len() == N {
            // Same length and element type, so the slice's storage is exactly one `[T; N]`.
            Ok(unsafe { *(slice.as_ptr() as *const [T; N]) })
        } else {
            Err(TryFromSliceError(()))
        }
    }
}

macro_rules! impl_from_bool {
    ($($t:ty)*) => {$(
        const impl From<bool> for $t {
            #[inline(always)]
            fn from(value: bool) -> Self { value as Self }
        }
    )*};
}
impl_from_bool!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);

macro_rules! impl_try_from_signed {
    ($($source:ty => $target:ty),* $(,)?) => {$(
        impl TryFrom<$source> for $target {
            type Error = TryFromIntError;
            #[inline(always)]
            fn try_from(value: $source) -> crate::result::Result<Self, Self::Error> {
                if value < 0 {
                    crate::result::Result::Err(TryFromIntError(()))
                } else {
                    crate::result::Result::Ok(value as Self)
                }
            }
        }
    )*};
}
// Same-width or wider targets only, so the one way to fail is a negative value.
impl_try_from_signed!(i8 => u8, i16 => u16, i32 => u32, i64 => u64, i128 => u128, isize => usize,
    i32 => usize, i64 => usize, i32 => u64, isize => u64);

impl TryFromIntError {
    pub(crate) const fn new() -> Self { TryFromIntError(()) }
}

impl_from_int!(u64 => i128);

impl_try_from_upper_bounded!(
    u32 => i32, u64 => i32, u64 => i64, usize => i32, usize => i64, usize => isize, u128 => i128,
);

/// Narrowing between signed types: the value must lie within the target's whole range.
macro_rules! impl_try_from_both_bounded {
    ($($source:ty => $target:ty),* $(,)?) => { $(
        impl TryFrom<$source> for $target {
            type Error = TryFromIntError;
            #[inline(always)]
            fn try_from(value: $source) -> crate::result::Result<Self, Self::Error> {
                if value < (Self::MIN as $source) || value > (Self::MAX as $source) {
                    crate::result::Result::Err(TryFromIntError(()))
                } else {
                    crate::result::Result::Ok(value as Self)
                }
            }
        }
    )* };
}

impl_try_from_both_bounded!(
    i16 => i8, i32 => i8, i32 => i16, i64 => i8, i64 => i16, i64 => i32,
    i128 => i32, i128 => i64, isize => i32,
);

impl crate::cmp::PartialEq for Infallible {
    fn eq(&self, _other: &Self) -> bool { match *self {} }
}
impl crate::cmp::Eq for Infallible {}
impl crate::fmt::Debug for Infallible {
    fn fmt(&self, _f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { match *self {} }
}
impl crate::fmt::Display for Infallible {
    fn fmt(&self, _f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { match *self {} }
}
