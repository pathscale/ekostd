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
        impl Rem for $ty {
            type Output = Self;
            #[inline]
            fn rem(self, rhs: Self) -> Self { self % rhs }
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
        impl RemAssign for $ty {
            #[inline]
            fn rem_assign(&mut self, rhs: Self) { *self %= rhs; }
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

scalar_binary_ops!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
scalar_bit_assign_ops!(bool, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
scalar_shift_ops!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
scalar_neg_ops!(i8, i16, i32, i64, i128, isize, f32, f64);
