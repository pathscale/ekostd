use crate::marker::{PointeeSized, Tuple, Unsize};

mod arith;
mod bit;
mod range;
mod try_trait;
pub use self::arith::*;
pub use self::bit::{BitAnd, BitOr, BitXor, Not};
pub use self::range::*;
pub use self::try_trait::{ControlFlow, FromResidual, Try};

#[lang = "coerce_unsized"]
pub trait CoerceUnsized<T: PointeeSized>: PointeeSized {}

impl<'a, 'b: 'a, T: PointeeSized + Unsize<U>, U: PointeeSized> CoerceUnsized<&'a U> for &'b T {}
impl<'a, 'b: 'a, T: PointeeSized + Unsize<U>, U: PointeeSized> CoerceUnsized<&'a mut U> for &'b mut T {}
impl<'a, T: PointeeSized + Unsize<U>, U: PointeeSized> CoerceUnsized<*const U> for &'a T {}
impl<'a, T: PointeeSized + Unsize<U>, U: PointeeSized> CoerceUnsized<*const U> for &'a mut T {}
impl<'a, T: PointeeSized + Unsize<U>, U: PointeeSized> CoerceUnsized<*mut U> for &'a mut T {}
impl<T: PointeeSized + Unsize<U>, U: PointeeSized> CoerceUnsized<*const U> for *const T {}
impl<T: PointeeSized + Unsize<U>, U: PointeeSized> CoerceUnsized<*mut U> for *mut T {}
impl<T: PointeeSized + Unsize<U>, U: PointeeSized> CoerceUnsized<*const U> for *mut T {}

#[lang = "dispatch_from_dyn"]
pub trait DispatchFromDyn<T> {}

impl<'a, T: PointeeSized + Unsize<U>, U: PointeeSized> DispatchFromDyn<&'a U> for &'a T {}
impl<'a, T: PointeeSized + Unsize<U>, U: PointeeSized> DispatchFromDyn<&'a mut U> for &'a mut T {}
impl<T: PointeeSized + Unsize<U>, U: PointeeSized> DispatchFromDyn<*const U> for *const T {}
impl<T: PointeeSized + Unsize<U>, U: PointeeSized> DispatchFromDyn<*mut U> for *mut T {}

#[lang = "drop"]
pub const trait Drop {
    fn drop(&mut self);
}

#[lang = "legacy_receiver"]
pub trait LegacyReceiver: PointeeSized {}
impl<T: PointeeSized> LegacyReceiver for &T {}
impl<T: PointeeSized> LegacyReceiver for &mut T {}

#[lang = "deref"]
pub const trait Deref: PointeeSized {
    #[lang = "deref_target"]
    type Target: ?Sized;
    fn deref(&self) -> &Self::Target;
}

#[lang = "deref_mut"]
pub const trait DerefMut: [const] Deref + PointeeSized {
    fn deref_mut(&mut self) -> &mut Self::Target;
}

const impl<T: ?Sized> Deref for &T {
    type Target = T;
    fn deref(&self) -> &T { self }
}
impl<T: ?Sized> !DerefMut for &T {}
const impl<T: ?Sized> Deref for &mut T {
    type Target = T;
    fn deref(&self) -> &T { self }
}
const impl<T: ?Sized> DerefMut for &mut T {
    fn deref_mut(&mut self) -> &mut T { self }
}

#[lang = "fn_once"]
#[rustc_paren_sugar]
#[fundamental]
pub const trait FnOnce<Args: Tuple> {
    #[lang = "fn_once_output"]
    type Output;
    extern "rust-call" fn call_once(self, args: Args) -> Self::Output;
}

#[lang = "fn_mut"]
#[rustc_paren_sugar]
#[fundamental]
pub const trait FnMut<Args: Tuple>: FnOnce<Args> {
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output;
}

#[lang = "fn"]
#[rustc_paren_sugar]
#[fundamental]
pub const trait Fn<Args: Tuple>: [const] FnMut<Args> {
    extern "rust-call" fn call(&self, args: Args) -> Self::Output;
}

impl<A: Tuple, F: ?Sized> FnOnce<A> for &F
where
    F: Fn<A>,
{
    type Output = F::Output;
    extern "rust-call" fn call_once(self, args: A) -> F::Output {
        (*self).call(args)
    }
}

impl<A: Tuple, F: ?Sized> FnMut<A> for &F
where
    F: Fn<A>,
{
    extern "rust-call" fn call_mut(&mut self, args: A) -> F::Output {
        (**self).call(args)
    }
}

impl<A: Tuple, F: ?Sized> Fn<A> for &F
where
    F: Fn<A>,
{
    extern "rust-call" fn call(&self, args: A) -> F::Output {
        (**self).call(args)
    }
}

impl<A: Tuple, F: ?Sized> FnOnce<A> for &mut F
where
    F: FnMut<A>,
{
    type Output = F::Output;
    extern "rust-call" fn call_once(self, args: A) -> F::Output {
        (*self).call_mut(args)
    }
}

impl<A: Tuple, F: ?Sized> FnMut<A> for &mut F
where
    F: FnMut<A>,
{
    extern "rust-call" fn call_mut(&mut self, args: A) -> F::Output {
        (*self).call_mut(args)
    }
}

#[lang = "index"]
pub trait Index<Idx: ?Sized>: PointeeSized {
    type Output: ?Sized;
    #[track_caller]
    fn index(&self, index: Idx) -> &Self::Output;
}

#[lang = "index_mut"]
pub trait IndexMut<Idx: ?Sized>: Index<Idx> + PointeeSized {
    #[track_caller]
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output;
}
