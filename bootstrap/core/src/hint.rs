//! Compiler and processor hints without an operating-system dependency.
/// Makes the optimizer treat the value as opaque.
#[inline(always)]
pub fn black_box<T>(value: T) -> T {
    // Intrinsic names are recognized by rustc, so use the intrinsic's exact name.
    mod intrinsic {
        #[rustc_intrinsic]
        #[rustc_nounwind]
        pub fn black_box<T>(value: T) -> T;
    }
    intrinsic::black_box(value)
}
#[rustc_builtin_macro]
macro_rules! asm { ($($tokens:tt)*) => {}; }
#[inline(always)]
pub fn spin_loop() {
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    unsafe { asm!("yield", options(nomem, nostack, preserves_flags)); }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    unsafe { asm!("pause", options(nomem, nostack, preserves_flags)); }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "arm", target_arch = "x86", target_arch = "x86_64")))]
    { black_box(()); }
}
/// # Safety
/// Execution must never reach this call.
#[inline(always)]
pub const unsafe fn unreachable_unchecked() -> ! { unsafe { crate::intrinsics::unreachable() } }
/// # Safety
/// The condition must be true whenever this function is called.
#[inline(always)]
pub const unsafe fn assert_unchecked(condition: bool) {
    if !condition { unsafe { unreachable_unchecked() } }
}
