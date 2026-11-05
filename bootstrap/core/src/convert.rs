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
impl<T: ?Sized> AsRef<T> for &T {
    fn as_ref(&self) -> &T { *self }
}
impl<T: ?Sized> AsRef<T> for &mut T {
    fn as_ref(&self) -> &T { &**self }
}
impl<T: ?Sized> AsMut<T> for &mut T {
    fn as_mut(&mut self) -> &mut T { &mut **self }
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
