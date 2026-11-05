#[lang = "not"]
pub const trait Not {
    type Output;
    fn not(self) -> Self::Output;
}

#[lang = "bitand"]
pub const trait BitAnd<Rhs = Self> {
    type Output;
    fn bitand(self, rhs: Rhs) -> Self::Output;
}

#[lang = "bitor"]
pub const trait BitOr<Rhs = Self> {
    type Output;
    fn bitor(self, rhs: Rhs) -> Self::Output;
}

#[lang = "bitxor"]
pub const trait BitXor<Rhs = Self> {
    type Output;
    fn bitxor(self, rhs: Rhs) -> Self::Output;
}

macro_rules! scalar_bit_ops {
    ($($ty:ty),* $(,)?) => { $(
        const impl Not for $ty {
            type Output = Self;
            #[inline]
            fn not(self) -> Self { !self }
        }
        const impl BitAnd for $ty {
            type Output = Self;
            #[inline]
            fn bitand(self, rhs: Self) -> Self { self & rhs }
        }
        const impl BitOr for $ty {
            type Output = Self;
            #[inline]
            fn bitor(self, rhs: Self) -> Self { self | rhs }
        }
        const impl BitXor for $ty {
            type Output = Self;
            #[inline]
            fn bitxor(self, rhs: Self) -> Self { self ^ rhs }
        }
    )* };
}

scalar_bit_ops!(bool, u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize);
