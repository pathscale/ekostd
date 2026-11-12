#[lang = "add"]
pub trait Add<Rhs = Self> {
    type Output;
    fn add(self, rhs: Rhs) -> Self::Output;
}

#[lang = "sub"]
pub trait Sub<Rhs = Self> {
    type Output;
    fn sub(self, rhs: Rhs) -> Self::Output;
}

#[lang = "mul"]
pub trait Mul<Rhs = Self> {
    type Output;
    fn mul(self, rhs: Rhs) -> Self::Output;
}

#[lang = "div"]
pub trait Div<Rhs = Self> {
    type Output;
    fn div(self, rhs: Rhs) -> Self::Output;
}

#[lang = "rem"]
pub trait Rem<Rhs = Self> {
    type Output;
    fn rem(self, rhs: Rhs) -> Self::Output;
}

#[lang = "neg"]
pub trait Neg {
    type Output;
    fn neg(self) -> Self::Output;
}

#[lang = "shl"]
pub trait Shl<Rhs = Self> {
    type Output;
    fn shl(self, rhs: Rhs) -> Self::Output;
}

#[lang = "shr"]
pub trait Shr<Rhs = Self> {
    type Output;
    fn shr(self, rhs: Rhs) -> Self::Output;
}

#[lang = "add_assign"]
pub trait AddAssign<Rhs = Self> {
    fn add_assign(&mut self, rhs: Rhs);
}

#[lang = "sub_assign"]
pub trait SubAssign<Rhs = Self> {
    fn sub_assign(&mut self, rhs: Rhs);
}

#[lang = "mul_assign"]
pub trait MulAssign<Rhs = Self> {
    fn mul_assign(&mut self, rhs: Rhs);
}

#[lang = "div_assign"]
pub trait DivAssign<Rhs = Self> {
    fn div_assign(&mut self, rhs: Rhs);
}

#[lang = "rem_assign"]
pub trait RemAssign<Rhs = Self> {
    fn rem_assign(&mut self, rhs: Rhs);
}

#[lang = "bitand_assign"]
pub trait BitAndAssign<Rhs = Self> {
    fn bitand_assign(&mut self, rhs: Rhs);
}

#[lang = "bitor_assign"]
pub trait BitOrAssign<Rhs = Self> {
    fn bitor_assign(&mut self, rhs: Rhs);
}

#[lang = "bitxor_assign"]
pub trait BitXorAssign<Rhs = Self> {
    fn bitxor_assign(&mut self, rhs: Rhs);
}

#[lang = "shl_assign"]
pub trait ShlAssign<Rhs = Self> {
    fn shl_assign(&mut self, rhs: Rhs);
}

#[lang = "shr_assign"]
pub trait ShrAssign<Rhs = Self> {
    fn shr_assign(&mut self, rhs: Rhs);
}

macro_rules! scalar_binary_ops {
    ($($ty:ty),* $(,)?) => { $(
        impl Add for $ty {
            type Output = Self;
            #[inline]
            fn add(self, rhs: Self) -> Self { self + rhs }
        }
        impl Sub for $ty {
            type Output = Self;
            #[inline]
            fn sub(self, rhs: Self) -> Self { self - rhs }
        }
        impl Mul for $ty {
            type Output = Self;
            #[inline]
            fn mul(self, rhs: Self) -> Self { self * rhs }
        }
        impl Div for $ty {
            type Output = Self;
            #[inline]
            fn div(self, rhs: Self) -> Self { self / rhs }
        }
        impl AddAssign for $ty {
            #[inline]
            fn add_assign(&mut self, rhs: Self) { *self += rhs; }
        }
        impl SubAssign for $ty {
            #[inline]
            fn sub_assign(&mut self, rhs: Self) { *self -= rhs; }
        }
        impl MulAssign for $ty {
            #[inline]
            fn mul_assign(&mut self, rhs: Self) { *self *= rhs; }
        }
        impl DivAssign for $ty {
            #[inline]
            fn div_assign(&mut self, rhs: Self) { *self /= rhs; }
        }
    )* };
}

macro_rules! scalar_bit_assign_ops {
    ($($ty:ty),* $(,)?) => { $(
        impl BitAndAssign for $ty {
            #[inline]
            fn bitand_assign(&mut self, rhs: Self) { *self &= rhs; }
        }
        impl BitOrAssign for $ty {
            #[inline]
            fn bitor_assign(&mut self, rhs: Self) { *self |= rhs; }
        }
        impl BitXorAssign for $ty {
            #[inline]
            fn bitxor_assign(&mut self, rhs: Self) { *self ^= rhs; }
        }
    )* };
}

macro_rules! scalar_shift_ops {
    ($($ty:ty),* $(,)?) => { $(
        impl Shl for $ty {
            type Output = Self;
            #[inline]
            fn shl(self, rhs: Self) -> Self { self << rhs }
        }
        impl Shr for $ty {
            type Output = Self;
            #[inline]
            fn shr(self, rhs: Self) -> Self { self >> rhs }
        }
        impl ShlAssign for $ty {
            #[inline]
            fn shl_assign(&mut self, rhs: Self) { *self <<= rhs; }
        }
        impl ShrAssign for $ty {
            #[inline]
            fn shr_assign(&mut self, rhs: Self) { *self >>= rhs; }
        }
    )* };
}

macro_rules! scalar_neg_ops {
    ($($ty:ty),* $(,)?) => { $(
        impl Neg for $ty {
            type Output = Self;
            #[inline]
            fn neg(self) -> Self { -self }
        }
    )* };
}

/// The remainder operators, on the integers only: no crate in the graph takes a floating-point remainder.
macro_rules! scalar_rem_ops {
    ($($ty:ty),* $(,)?) => { $(
        impl Rem for $ty {
            type Output = Self;
            #[inline]
            fn rem(self, rhs: Self) -> Self { self % rhs }
        }
        impl RemAssign for $ty {
            #[inline]
            fn rem_assign(&mut self, rhs: Self) { *self %= rhs; }
        }
    )* };
}

scalar_binary_ops!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
scalar_rem_ops!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
scalar_bit_assign_ops!(bool, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
scalar_shift_ops!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
scalar_neg_ops!(i8, i16, i32, i64, i128, isize, f32, f64);

/// Shifts take any integer type on the right, as in Rust: `x << 3u8`, `x >> n` with `n: usize`.
macro_rules! mixed_shift_ops {
    ($lhs:ty: $($rhs:ty)*) => {$(
        impl Shl<$rhs> for $lhs {
            type Output = Self;
            #[inline]
            fn shl(self, rhs: $rhs) -> Self { self << rhs }
        }
        impl Shr<$rhs> for $lhs {
            type Output = Self;
            #[inline]
            fn shr(self, rhs: $rhs) -> Self { self >> rhs }
        }
        impl ShlAssign<$rhs> for $lhs {
            #[inline]
            fn shl_assign(&mut self, rhs: $rhs) { *self <<= rhs; }
        }
        impl ShrAssign<$rhs> for $lhs {
            #[inline]
            fn shr_assign(&mut self, rhs: $rhs) { *self >>= rhs; }
        }
    )*};
}
mixed_shift_ops!(u8: u16 u32 u64 usize i8 i16 i32 i64 isize);
mixed_shift_ops!(u16: u8 u32 u64 usize i8 i16 i32 i64 isize);
mixed_shift_ops!(u32: u8 u16 u64 usize i8 i16 i32 i64 isize);
mixed_shift_ops!(u64: u8 u16 u32 usize i8 i16 i32 i64 isize);
mixed_shift_ops!(u128: u8 u16 u32 u64 usize i8 i16 i32 i64 isize);
mixed_shift_ops!(usize: u8 u16 u32 u64 i8 i16 i32 i64 isize);
mixed_shift_ops!(i8: u8 u16 u32 u64 usize i16 i32 i64 isize);
mixed_shift_ops!(i16: u8 u16 u32 u64 usize i8 i32 i64 isize);
mixed_shift_ops!(i32: u8 u16 u32 u64 usize i8 i16 i64 isize);
mixed_shift_ops!(i64: u8 u16 u32 u64 usize i8 i16 i32 isize);
mixed_shift_ops!(i128: u8 u16 u32 u64 usize i8 i16 i32 i64 isize);
mixed_shift_ops!(isize: u8 u16 u32 u64 usize i8 i16 i32 i64);

/// The operators with a reference on either side: they read the value and apply the plain operator.
macro_rules! scalar_ref_ops {
    ($($ty:ty),* $(,)?) => { $(
        scalar_ref_ops!(@binary $ty, Add add, Sub sub, Mul mul, Div div, Rem rem);
        scalar_ref_ops!(@assign $ty, AddAssign add_assign, SubAssign sub_assign, MulAssign mul_assign,
            DivAssign div_assign, RemAssign rem_assign);
    )* };
    (@binary $ty:ty, $($op:ident $method:ident),*) => { $(
        impl $op<&$ty> for $ty {
            type Output = $ty;
            #[inline]
            fn $method(self, rhs: &$ty) -> $ty { $op::$method(self, *rhs) }
        }
        impl $op<$ty> for &$ty {
            type Output = $ty;
            #[inline]
            fn $method(self, rhs: $ty) -> $ty { $op::$method(*self, rhs) }
        }
        impl $op<&$ty> for &$ty {
            type Output = $ty;
            #[inline]
            fn $method(self, rhs: &$ty) -> $ty { $op::$method(*self, *rhs) }
        }
    )* };
    (@assign $ty:ty, $($op:ident $method:ident),*) => { $(
        impl $op<&$ty> for $ty {
            #[inline]
            fn $method(&mut self, rhs: &$ty) { $op::$method(self, *rhs) }
        }
    )* };
}
scalar_ref_ops!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
