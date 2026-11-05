use crate::marker::{Copy, Destruct, PointeeSized, Sized};

#[lang = "clone"]
#[rustc_diagnostic_item = "Clone"]
pub const trait Clone: Sized {
    fn clone(&self) -> Self;

    fn clone_from(&mut self, source: &Self)
    where
        Self: [const] Destruct,
    {
        *self = source.clone();
    }
}

pub unsafe trait TrivialClone: Clone + Copy {}

pub struct AssertParamIsClone<T: Clone + PointeeSized> {
    _field: crate::marker::PhantomData<T>,
}
pub struct AssertParamIsCopy<T: Copy + PointeeSized> {
    _field: crate::marker::PhantomData<T>,
}

macro_rules! scalar_clone {
    ($($ty:ty),* $(,)?) => { $(
        const impl Clone for $ty {
            fn clone(&self) -> Self { *self }
        }
        unsafe impl TrivialClone for $ty {}
    )* };
}
scalar_clone!(bool, char, u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize, f32, f64);

const impl<T: PointeeSized> Clone for *const T {
    fn clone(&self) -> Self { *self }
}
unsafe impl<T: PointeeSized> TrivialClone for *const T {}
const impl<T: PointeeSized> Clone for *mut T {
    fn clone(&self) -> Self { *self }
}
unsafe impl<T: PointeeSized> TrivialClone for *mut T {}
const impl<T: PointeeSized> Clone for &T {
    fn clone(&self) -> Self { *self }
}
unsafe impl<T: PointeeSized> TrivialClone for &T {}
impl<T: PointeeSized> !Clone for &mut T {}

impl<T: Copy, const N: usize> Clone for [T; N] {
    fn clone(&self) -> Self { *self }
}
unsafe impl<T: Copy + TrivialClone, const N: usize> TrivialClone for [T; N] {}
