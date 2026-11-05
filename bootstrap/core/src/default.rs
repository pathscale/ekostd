use crate::marker::{PhantomData, PointeeSized, Sized};

pub const trait Default: Sized {
    fn default() -> Self;
}

macro_rules! scalar_default {
    ($($ty:ty => $value:expr),* $(,)?) => { $(
        const impl Default for $ty {
            fn default() -> Self { $value }
        }
    )* };
}

scalar_default!(
    () => (), bool => false, char => '\0',
    u8 => 0, u16 => 0, u32 => 0, u64 => 0, u128 => 0, usize => 0,
    i8 => 0, i16 => 0, i32 => 0, i64 => 0, i128 => 0, isize => 0,
    f32 => 0.0, f64 => 0.0,
);

const impl<T: PointeeSized> Default for PhantomData<T> {
    fn default() -> Self { PhantomData }
}
