use crate::clone::Clone;

#[lang = "pointee_sized"]
#[fundamental]
#[rustc_deny_explicit_impl]
#[rustc_coinductive]
pub trait PointeeSized {}

#[lang = "meta_sized"]
#[fundamental]
#[rustc_deny_explicit_impl]
#[rustc_coinductive]
pub trait MetaSized: PointeeSized {}

#[lang = "sized"]
#[fundamental]
#[rustc_deny_explicit_impl]
#[rustc_dyn_incompatible_trait]
#[rustc_coinductive]
pub trait Sized: MetaSized {}

#[lang = "unsize"]
#[rustc_deny_explicit_impl]
#[rustc_dyn_incompatible_trait]
pub trait Unsize<T: PointeeSized>: PointeeSized {}

#[lang = "copy"]
#[rustc_diagnostic_item = "Copy"]
pub trait Copy: Clone {}

#[lang = "destruct"]
#[rustc_deny_explicit_impl]
#[rustc_dyn_incompatible_trait]
pub const trait Destruct: PointeeSized {}

#[lang = "tuple_trait"]
#[fundamental]
#[rustc_deny_explicit_impl]
#[rustc_dyn_incompatible_trait]
pub trait Tuple {}

#[rustc_diagnostic_item = "Send"]
pub unsafe auto trait Send {}

#[lang = "sync"]
pub unsafe auto trait Sync {}

impl<T: PointeeSized> !Send for *const T {}
impl<T: PointeeSized> !Send for *mut T {}
impl<T: PointeeSized> !Sync for *const T {}
impl<T: PointeeSized> !Sync for *mut T {}
unsafe impl<T: Sync + PointeeSized> Send for &T {}

#[lang = "freeze"]
pub unsafe auto trait Freeze {}

unsafe impl<T: PointeeSized> Freeze for *const T {}
unsafe impl<T: PointeeSized> Freeze for *mut T {}
unsafe impl<T: PointeeSized> Freeze for &T {}
unsafe impl<T: PointeeSized> Freeze for &mut T {}

#[lang = "unpin"]
pub auto trait Unpin {}

impl<T: PointeeSized> Unpin for &T {}
impl<T: PointeeSized> Unpin for &mut T {}

#[lang = "phantom_data"]
pub struct PhantomData<T: PointeeSized>;

impl<T: PointeeSized> Copy for PhantomData<T> {}
impl<T: PointeeSized> Clone for PhantomData<T> {
    fn clone(&self) -> Self { *self }
}
unsafe impl<T: PointeeSized> Freeze for PhantomData<T> {}

pub struct PhantomPinned;
impl !Unpin for PhantomPinned {}

macro_rules! scalar_copy {
    ($($ty:ty),* $(,)?) => { $(impl Copy for $ty {})* };
}
scalar_copy!(bool, char, u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize, f32, f64);

impl<T: PointeeSized> Copy for *const T {}
impl<T: PointeeSized> Copy for *mut T {}
impl<T: PointeeSized> Copy for &T {}
impl<T: Copy, const N: usize> Copy for [T; N] {}
