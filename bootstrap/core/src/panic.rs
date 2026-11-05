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
#[track_caller]
pub fn panic_fmt(args: fmt::Arguments<'_>) -> ! {
    unsafe extern "Rust" {
        #[lang = "panic_impl"]
        fn panic_impl(info: &PanicInfo<'_>) -> !;
    }
    let info = PanicInfo::new(&args, Location::caller(), true, false);
    unsafe { panic_impl(&info) }
}
