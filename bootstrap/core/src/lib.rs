//! Minimal bootstrap candidate for the selected rustc, independent of installed core.
#![no_core]
#![rustc_coherence_is_core]
#![crate_name = "core"]
#![crate_type = "rlib"]
#![feature(no_core, lang_items, rustc_attrs, intrinsics)]
#![feature(auto_traits, negative_impls, fundamental, never_type)]
#![feature(freeze_impls, unsized_fn_params)]
#![feature(const_precise_live_drops, transparent_unions)]
#![feature(const_trait_impl, const_destruct, unboxed_closures, prelude_import)]
#![feature(decl_macro, allow_internal_unstable, structural_match, builtin_syntax)]
#![feature(adt_const_params, generic_const_parameter_types)]

extern crate self as core;

#[macro_use]
pub mod macros;

pub mod alloc;
pub mod arch;
pub mod ascii;
pub mod array;
pub mod any;
pub mod borrow;
pub mod cell;
pub mod char;
pub mod clone;
pub mod cmp;
pub mod convert;
pub mod default;
pub mod ffi;
pub mod fmt;
pub mod hash;
pub mod hint;
pub mod intrinsics;
pub mod iter;
pub mod marker;
pub mod mem;
pub mod num;
pub mod ops;
pub mod option;
pub mod panic;
pub mod pin;
pub mod prelude;
pub mod ptr;
pub mod result;
pub mod slice;
pub mod str;
pub mod sync;
pub mod time;
mod unicode;

pub mod task {
    #[lang = "Poll"]
    #[derive(Copy, Clone)]
    pub enum Poll<T> {
        #[lang = "Ready"]
        Ready(T),
        #[lang = "Pending"]
        Pending,
    }

    impl<T> Poll<T> {
        pub fn map<U, F: crate::ops::FnOnce(T) -> U>(self, f: F) -> Poll<U> {
            match self { Poll::Ready(t) => Poll::Ready(f(t)), Poll::Pending => Poll::Pending }
        }
        pub const fn is_ready(&self) -> bool { matches!(*self, Poll::Ready(_)) }
        pub const fn is_pending(&self) -> bool { !self.is_ready() }
    }

    #[derive(Copy, Clone)]
    pub struct RawWakerVTable {
        clone: unsafe fn(*const ()) -> RawWaker,
        wake: unsafe fn(*const ()),
        wake_by_ref: unsafe fn(*const ()),
        drop: unsafe fn(*const ()),
    }

    impl PartialEq for RawWakerVTable {
        fn eq(&self, other: &Self) -> bool {
            self.clone as usize == other.clone as usize
                && self.wake as usize == other.wake as usize
                && self.wake_by_ref as usize == other.wake_by_ref as usize
                && self.drop as usize == other.drop as usize
        }
    }
    impl Eq for RawWakerVTable {}
    impl crate::fmt::Debug for RawWakerVTable {
        fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
            f.write_str("RawWakerVTable")
        }
    }

    impl RawWakerVTable {
        pub const fn new(
            clone: unsafe fn(*const ()) -> RawWaker,
            wake: unsafe fn(*const ()),
            wake_by_ref: unsafe fn(*const ()),
            drop: unsafe fn(*const ()),
        ) -> Self {
            Self { clone, wake, wake_by_ref, drop }
        }
    }

    #[derive(PartialEq, Eq)]
    pub struct RawWaker {
        data: *const (),
        vtable: &'static RawWakerVTable,
    }
    impl crate::fmt::Debug for RawWaker {
        fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
            f.write_str("RawWaker")
        }
    }

    impl RawWaker {
        pub const fn new(data: *const (), vtable: &'static RawWakerVTable) -> Self {
            Self { data, vtable }
        }
    }

    #[repr(transparent)]
    pub struct Waker {
        waker: RawWaker,
    }

    unsafe impl Send for Waker {}
    unsafe impl Sync for Waker {}

    unsafe fn noop_clone(_: *const ()) -> RawWaker {
        RawWaker::new(crate::ptr::null(), &NOOP_VTABLE)
    }

    unsafe fn noop(_: *const ()) {}

    static NOOP_VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop, noop, noop);
    static NOOP_WAKER: Waker = unsafe { Waker::from_raw(RawWaker::new(crate::ptr::null(), &NOOP_VTABLE)) };

    impl Waker {
        pub const unsafe fn from_raw(waker: RawWaker) -> Waker {
            Waker { waker }
        }

        pub fn wake(self) {
            unsafe { (self.waker.vtable.wake)(self.waker.data) };
            crate::mem::forget(self);
        }

        pub fn wake_by_ref(&self) {
            unsafe { (self.waker.vtable.wake_by_ref)(self.waker.data) };
        }

        pub fn will_wake(&self, other: &Waker) -> bool {
            self.waker == other.waker
        }

        pub const fn data(&self) -> *const () {
            self.waker.data
        }

        pub const fn vtable(&self) -> &'static RawWakerVTable {
            self.waker.vtable
        }

        pub const fn noop() -> &'static Waker {
            &NOOP_WAKER
        }
    }

    impl Clone for Waker {
        fn clone(&self) -> Self {
            unsafe { Waker::from_raw((self.waker.vtable.clone)(self.waker.data)) }
        }
    }

    impl Drop for Waker {
        fn drop(&mut self) {
            unsafe { (self.waker.vtable.drop)(self.waker.data) };
        }
    }

    impl crate::fmt::Debug for Waker {
        fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
            f.write_str("Waker")
        }
    }

    #[lang = "Context"]
    pub struct Context<'a> {
        waker: &'a Waker,
        _marker: crate::marker::PhantomData<fn(&'a ()) -> &'a ()>,
    }

    impl<'a> Context<'a> {
        pub const fn from_waker(waker: &'a Waker) -> Context<'a> {
            Context { waker, _marker: crate::marker::PhantomData }
        }

        pub const fn waker(&self) -> &'a Waker {
            self.waker
        }
    }

    impl crate::fmt::Debug for Context<'_> {
        fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
            f.write_str("Context")
        }
    }
}

pub mod future {
    /// The resume argument of an async body: a pointer to the `Context` it is polled with.
    #[lang = "ResumeTy"]
    pub struct ResumeTy(crate::ptr::NonNull<crate::task::Context<'static>>);
    impl crate::marker::Copy for ResumeTy {}
    impl crate::clone::Clone for ResumeTy { fn clone(&self) -> Self { *self } }
    unsafe impl crate::marker::Send for ResumeTy {}
    unsafe impl crate::marker::Sync for ResumeTy {}

    /// SAFETY: the caller must guarantee that `cx.0` is a valid pointer that fulfils the requirements of a
    /// mutable reference.
    #[lang = "get_context"]
    pub unsafe fn get_context<'a, 'b>(cx: ResumeTy) -> &'a mut crate::task::Context<'b> {
        unsafe { &mut *cx.0.as_ptr().cast() }
    }

    #[lang = "async_drop"]
    #[allow(async_fn_in_trait)]
    pub trait AsyncDrop {
        async fn drop(self: crate::pin::Pin<&mut Self>);
    }

    pub trait IntoFuture {
        type Output;
        type IntoFuture: Future<Output = Self::Output>;
        #[lang = "into_future"]
        fn into_future(self) -> Self::IntoFuture;
    }
    impl<F: Future> IntoFuture for F {
        type Output = F::Output;
        type IntoFuture = F;
        fn into_future(self) -> F { self }
    }

    #[lang = "future_trait"]
    pub trait Future {
        type Output;
        #[lang = "poll"]
        fn poll(self: crate::pin::Pin<&mut Self>, cx: &mut crate::task::Context<'_>) -> crate::task::Poll<Self::Output>;
    }

    impl<P> Future for crate::pin::Pin<P>
    where
        P: crate::ops::DerefMut,
        P::Target: Future,
    {
        type Output = <<P as crate::ops::Deref>::Target as Future>::Output;
        fn poll(self: crate::pin::Pin<&mut Self>, cx: &mut crate::task::Context<'_>) -> crate::task::Poll<Self::Output> {
            // SAFETY: the pinned pointer is only used to project to its pinned target.
            let this: &mut crate::pin::Pin<P> = unsafe { self.get_unchecked_mut() };
            <P::Target as Future>::poll(this.as_mut(), cx)
        }
    }

    /// A future that is immediately ready with a value.
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Ready<T>(crate::option::Option<T>);

    impl<T> crate::marker::Unpin for Ready<T> {}

    impl<T: crate::clone::Clone> crate::clone::Clone for Ready<T> {
        #[inline]
        fn clone(&self) -> Self {
            Ready(self.0.clone())
        }
    }

    impl<T: crate::fmt::Debug> crate::fmt::Debug for Ready<T> {
        fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
            f.debug_tuple("Ready").field(&self.0).finish()
        }
    }

    impl<T> Future for Ready<T> {
        type Output = T;

        #[inline]
        fn poll(mut self: crate::pin::Pin<&mut Self>, _cx: &mut crate::task::Context<'_>) -> crate::task::Poll<T> {
            crate::task::Poll::Ready(self.0.take().expect("`Ready` polled after completion"))
        }
    }

    impl<T> Ready<T> {
        /// Consumes the `Ready`, returning the wrapped value.
        #[inline]
        pub fn into_inner(self) -> T {
            self.0.expect("`Ready` polled after completion")
        }
    }

    /// Creates a future that is immediately ready with a value.
    #[inline]
    pub fn ready<T>(t: T) -> Ready<T> {
        Ready(crate::option::Option::Some(t))
    }

    /// A future that never resolves, representing a computation that never finishes.
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Pending<T> {
        _data: crate::marker::PhantomData<fn() -> T>,
    }

    /// Creates a future which never resolves, representing a computation that never finishes.
    #[inline]
    pub fn pending<T>() -> Pending<T> {
        Pending { _data: crate::marker::PhantomData }
    }

    impl<T> Future for Pending<T> {
        type Output = T;

        #[inline]
        fn poll(self: crate::pin::Pin<&mut Self>, _cx: &mut crate::task::Context<'_>) -> crate::task::Poll<T> {
            crate::task::Poll::Pending
        }
    }

    impl<T> crate::clone::Clone for Pending<T> {
        #[inline]
        fn clone(&self) -> Self {
            pending()
        }
    }

    impl<T> crate::marker::Copy for Pending<T> {}

    impl<T> crate::fmt::Debug for Pending<T> {
        fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
            f.debug_struct("Pending").finish()
        }
    }

    impl<T> crate::default::Default for Pending<T> {
        #[inline]
        fn default() -> Self {
            pending()
        }
    }

    /// A future that wraps a function returning `Poll`.
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct PollFn<F> {
        f: F,
    }

    /// Creates a new future wrapping around a function returning `Poll`.
    #[inline]
    pub fn poll_fn<T, F>(f: F) -> PollFn<F>
    where
        F: crate::ops::FnMut(&mut crate::task::Context<'_>) -> crate::task::Poll<T>,
    {
        PollFn { f }
    }

    impl<F: crate::fmt::Debug> crate::fmt::Debug for PollFn<F> {
        fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
            f.debug_struct("PollFn").field("f", &self.f).finish()
        }
    }

    impl<T, F> Future for PollFn<F>
    where
        F: crate::ops::FnMut(&mut crate::task::Context<'_>) -> crate::task::Poll<T>,
    {
        type Output = T;

        #[inline]
        fn poll(mut self: crate::pin::Pin<&mut Self>, cx: &mut crate::task::Context<'_>) -> crate::task::Poll<T> {
            let this = unsafe { self.get_unchecked_mut() };
            (this.f)(cx)
        }
    }
}

pub mod error {
    pub trait Error: crate::fmt::Debug + crate::fmt::Display {
        fn source(&self) -> crate::option::Option<&(dyn Error + 'static)> {
            crate::option::Option::None
        }
    }
}

/// The pre-associated-constant spellings (`core::usize::MAX`), which older crates still import.
macro_rules! legacy_int_module {
    ($($t:ident)*) => {$(
        pub mod $t {
            pub const MIN: $t = <$t>::MIN;
            pub const MAX: $t = <$t>::MAX;
        }
    )*};
}
legacy_int_module!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);

pub mod f32 {
    pub const RADIX: u32 = 2;
    pub const MANTISSA_DIGITS: u32 = 24;
    pub const DIGITS: u32 = 6;
    pub const EPSILON: f32 = 1.19209290e-07_f32;
    pub const MIN: f32 = -3.40282347e+38_f32;
    pub const MIN_POSITIVE: f32 = 1.17549435e-38_f32;
    pub const MAX: f32 = 3.40282347e+38_f32;
    pub const MIN_EXP: i32 = -125;
    pub const MAX_EXP: i32 = 128;
    pub const MIN_10_EXP: i32 = -37;
    pub const MAX_10_EXP: i32 = 38;
    pub const INFINITY: f32 = 1.0_f32 / 0.0_f32;
    pub const NEG_INFINITY: f32 = -1.0_f32 / 0.0_f32;
    pub const NAN: f32 = 0.0_f32 / 0.0_f32;
    pub mod consts {
        pub const PI: f32 = 3.14159265358979323846_f32;
        pub const TAU: f32 = 6.28318530717958647692_f32;
        pub const FRAC_PI_2: f32 = 1.57079632679489661923_f32;
        pub const FRAC_PI_3: f32 = 1.04719755119659774615_f32;
        pub const FRAC_PI_4: f32 = 0.78539816339744830961_f32;
        pub const FRAC_PI_6: f32 = 0.52359877559829887308_f32;
        pub const FRAC_PI_8: f32 = 0.39269908169872415481_f32;
        pub const FRAC_1_PI: f32 = 0.31830988618379067154_f32;
        pub const FRAC_2_PI: f32 = 0.63661977236758134308_f32;
        pub const FRAC_2_SQRT_PI: f32 = 1.12837916709551257390_f32;
        pub const E: f32 = 2.71828182845904523536_f32;
        pub const LOG2_E: f32 = 1.44269504088896340736_f32;
        pub const LOG10_E: f32 = 0.43429448190325182765_f32;
        pub const LN_2: f32 = 0.69314718055994530941_f32;
        pub const LN_10: f32 = 2.30258509299404568401_f32;
        pub const SQRT_2: f32 = 1.41421356237309504880_f32;
        pub const FRAC_1_SQRT_2: f32 = 0.70710678118654752440_f32;
    }
}

pub mod f64 {
    pub const RADIX: u32 = 2;
    pub const MANTISSA_DIGITS: u32 = 53;
    pub const DIGITS: u32 = 15;
    pub const EPSILON: f64 = 2.2204460492503131e-16_f64;
    pub const MIN: f64 = -1.7976931348623157e+308_f64;
    pub const MIN_POSITIVE: f64 = 2.2250738585072014e-308_f64;
    pub const MAX: f64 = 1.7976931348623157e+308_f64;
    pub const MIN_EXP: i32 = -1021;
    pub const MAX_EXP: i32 = 1024;
    pub const MIN_10_EXP: i32 = -307;
    pub const MAX_10_EXP: i32 = 308;
    pub const INFINITY: f64 = 1.0_f64 / 0.0_f64;
    pub const NEG_INFINITY: f64 = -1.0_f64 / 0.0_f64;
    pub const NAN: f64 = 0.0_f64 / 0.0_f64;
    pub mod consts {
        pub const PI: f64 = 3.14159265358979323846_f64;
        pub const TAU: f64 = 6.28318530717958647692_f64;
        pub const FRAC_PI_2: f64 = 1.57079632679489661923_f64;
        pub const FRAC_PI_3: f64 = 1.04719755119659774615_f64;
        pub const FRAC_PI_4: f64 = 0.78539816339744830961_f64;
        pub const FRAC_PI_6: f64 = 0.52359877559829887308_f64;
        pub const FRAC_PI_8: f64 = 0.39269908169872415481_f64;
        pub const FRAC_1_PI: f64 = 0.31830988618379067154_f64;
        pub const FRAC_2_PI: f64 = 0.63661977236758134308_f64;
        pub const FRAC_2_SQRT_PI: f64 = 1.12837916709551257390_f64;
        pub const E: f64 = 2.71828182845904523536_f64;
        pub const LOG2_E: f64 = 1.44269504088896340736_f64;
        pub const LOG10_E: f64 = 0.43429448190325182765_f64;
        pub const LN_2: f64 = 0.69314718055994530941_f64;
        pub const LN_10: f64 = 2.30258509299404568401_f64;
        pub const SQRT_2: f64 = 1.41421356237309504880_f64;
        pub const FRAC_1_SQRT_2: f64 = 0.70710678118654752440_f64;
    }
}

pub use crate::panic::panicking;
pub use crate::macros::{format_args, const_format_args, cfg, env, option_env, line, column, file, module_path, stringify, concat, include, include_str, include_bytes, derive, compile_error, global_allocator, test};

#[prelude_import]
#[allow(unused_imports)]
use crate::prelude::rust_2024::*;
