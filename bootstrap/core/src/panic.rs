//! Panic metadata and unwind-safety markers for the bootstrap runtime.
use crate::fmt;
#[lang = "panic_location"]
pub struct Location<'a> {
    file: &'a str,
    line: u32,
    col: u32,
}
impl Copy for Location<'_> {}
impl Clone for Location<'_> { fn clone(&self) -> Self { *self } }
#[rustc_intrinsic]
#[rustc_nounwind]
const fn caller_location() -> &'static Location<'static>;
impl<'a> Location<'a> {
    #[track_caller]
    pub const fn caller() -> &'static Location<'static> { caller_location() }
    pub const fn file(&self) -> &'a str { self.file }
    pub const fn line(&self) -> u32 { self.line }
    pub const fn column(&self) -> u32 { self.col }
}
impl fmt::Display for Location<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match f.write_str(self.file) { Err(e) => return Err(e), Ok(()) => {} }
        match f.write_str(":") { Err(e) => return Err(e), Ok(()) => {} }
        match fmt::Display::fmt(&self.line, f) { Err(e) => return Err(e), Ok(()) => {} }
        match f.write_str(":") { Err(e) => return Err(e), Ok(()) => {} }
        fmt::Display::fmt(&self.col, f)
    }
}
#[lang = "panic_info"]
pub struct PanicInfo<'a> {
    message: &'a fmt::Arguments<'a>,
    location: &'static Location<'static>,
    can_unwind: bool,
    force_no_backtrace: bool,
}
impl<'a> PanicInfo<'a> {
    pub(crate) fn new(
        message: &'a fmt::Arguments<'a>, location: &'static Location<'static>,
        can_unwind: bool, force_no_backtrace: bool,
    ) -> Self { Self { message, location, can_unwind, force_no_backtrace } }
    pub const fn message(&self) -> PanicMessage<'_> { PanicMessage { message: self.message } }
    pub const fn location(&self) -> Option<&Location<'_>> { Some(self.location) }
    pub const fn can_unwind(&self) -> bool { self.can_unwind }
    pub const fn force_no_backtrace(&self) -> bool { self.force_no_backtrace }
}
pub struct PanicMessage<'a> { message: &'a fmt::Arguments<'a> }
impl PanicMessage<'_> {
    pub const fn as_str(&self) -> Option<&str> { self.message.as_str() }
}
impl fmt::Display for PanicMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_fmt(*self.message) }
}
impl fmt::Debug for PanicMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_fmt(*self.message) }
}
pub auto trait UnwindSafe {}
pub auto trait RefUnwindSafe {}
impl<T: ?Sized> !UnwindSafe for &mut T {}
impl<T: RefUnwindSafe + ?Sized> UnwindSafe for &T {}
impl<T: RefUnwindSafe + ?Sized> UnwindSafe for *const T {}
impl<T: RefUnwindSafe + ?Sized> UnwindSafe for *mut T {}
impl<T: ?Sized> !RefUnwindSafe for crate::cell::UnsafeCell<T> {}
pub struct AssertUnwindSafe<T>(pub T);
impl<T> UnwindSafe for AssertUnwindSafe<T> {}
impl<T> RefUnwindSafe for AssertUnwindSafe<T> {}
impl<T: Copy> Copy for AssertUnwindSafe<T> {}
impl<T: Clone> Clone for AssertUnwindSafe<T> {
    fn clone(&self) -> Self { Self(self.0.clone()) }
}
impl<T> crate::ops::Deref for AssertUnwindSafe<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.0 }
}
impl<T> crate::ops::DerefMut for AssertUnwindSafe<T> {
    fn deref_mut(&mut self) -> &mut T { &mut self.0 }
}
#[macro_export]
#[rustc_diagnostic_item = "core_panic_2021_macro"]
macro_rules! panic_2021 {
    () => { $crate::panic::panic_fmt($crate::fmt::Arguments::from_str("explicit panic")) };
    ($($arg:tt)+) => { $crate::panic::panic_fmt($crate::format_args!($($arg)+)) };
}
pub use crate::panic_2021;
unsafe extern "Rust" {
    #[lang = "panic_impl"]
    fn panic_impl(info: &PanicInfo<'_>) -> !;
}

#[lang = "panic_fmt"]
#[track_caller]
#[rustc_do_not_const_check]
pub const fn panic_fmt(args: fmt::Arguments<'_>) -> ! {
    let info = PanicInfo::new(&args, Location::caller(), true, false);
    unsafe { panic_impl(&info) }
}

#[lang = "panic_nounwind"]
#[rustc_nounwind]
#[rustc_do_not_const_check]
pub const fn panic_nounwind(expr: &'static str) -> ! {
    let args = fmt::Arguments::from_str(expr);
    let info = PanicInfo::new(&args, Location::caller(), false, false);
    unsafe { panic_impl(&info) }
}

#[lang = "panic_bounds_check"]
#[track_caller]
pub fn panic_bounds_check(index: usize, len: usize) -> ! {
    panic_fmt(crate::format_args!("index out of bounds: the len is {} but the index is {}", len, index))
}

#[lang = "panic_cannot_unwind"]
#[rustc_nounwind]
pub fn panic_cannot_unwind() -> ! { panic_nounwind("panic in a function that cannot unwind") }

#[lang = "panic_null_pointer_dereference"]
#[track_caller]
#[rustc_nounwind]
pub const fn panic_null_pointer_dereference() -> ! { panic_nounwind("null pointer dereference occurred") }

#[lang = "panic_misaligned_pointer_dereference"]
#[track_caller]
#[rustc_nounwind]
pub const fn panic_misaligned_pointer_dereference(_required: usize, _found: usize) -> ! {
    panic_nounwind("misaligned pointer dereference")
}

pub mod panic_const {
    use super::panic_nounwind;

    #[lang = "panic_const_add_overflow"]
    #[track_caller]
    pub const fn panic_const_add_overflow() -> ! { panic_nounwind("attempt to add with overflow") }

    #[lang = "panic_const_sub_overflow"]
    #[track_caller]
    pub const fn panic_const_sub_overflow() -> ! { panic_nounwind("attempt to subtract with overflow") }

    #[lang = "panic_const_mul_overflow"]
    #[track_caller]
    pub const fn panic_const_mul_overflow() -> ! { panic_nounwind("attempt to multiply with overflow") }

    #[lang = "panic_const_div_by_zero"]
    #[track_caller]
    pub const fn panic_const_div_by_zero() -> ! { panic_nounwind("attempt to divide by zero") }

    #[lang = "panic_const_rem_by_zero"]
    #[track_caller]
    pub const fn panic_const_rem_by_zero() -> ! { panic_nounwind("attempt to calculate the remainder with a divisor of zero") }

    #[lang = "panic_const_neg_overflow"]
    #[track_caller]
    pub const fn panic_const_neg_overflow() -> ! { panic_nounwind("attempt to negate with overflow") }

    #[lang = "panic_const_shr_overflow"]
    #[track_caller]
    pub const fn panic_const_shr_overflow() -> ! { panic_nounwind("attempt to shift right with overflow") }

    #[lang = "panic_const_shl_overflow"]
    #[track_caller]
    pub const fn panic_const_shl_overflow() -> ! { panic_nounwind("attempt to shift left with overflow") }

    #[lang = "panic_const_div_overflow"]
    #[track_caller]
    pub const fn panic_const_div_overflow() -> ! { panic_nounwind("attempt to divide with overflow") }

    #[lang = "panic_const_rem_overflow"]
    #[track_caller]
    pub const fn panic_const_rem_overflow() -> ! { panic_nounwind("attempt to calculate the remainder with overflow") }
}

pub mod panicking {
    pub use super::panic_const::*;
    pub use super::{panic_fmt, panic_nounwind, panic_bounds_check};
}
