//! Application process entry point (`process_entry!`) and exit outcome conversion (`EntryOutcome`).

/// Trait allowing [`crate::process_entry!`] to accept `()`, `u8`, `i32`, or `Result<(), E>`.
pub trait EntryOutcome {
    /// Converts the entry point return value into a process exit status code.
    fn into_exit_code(self) -> i32;
}

impl EntryOutcome for () {
    #[inline]
    fn into_exit_code(self) -> i32 {
        0
    }
}

impl EntryOutcome for u8 {
    #[inline]
    fn into_exit_code(self) -> i32 {
        self as i32
    }
}

impl EntryOutcome for i32 {
    #[inline]
    fn into_exit_code(self) -> i32 {
        self
    }
}

impl<E> EntryOutcome for core::result::Result<(), E> {
    #[inline]
    fn into_exit_code(self) -> i32 {
        match self {
            Ok(()) => 0,
            Err(_) => 1,
        }
    }
}

/// Emits the process entry point for `main_fn`.
///
/// On freestanding `aarch64-unknown-none` with `feature = "libc-rs"`, delegates to
/// `::libc_rs::process_entry!` (emitting `_start`, stack initialization, and panic handler)
/// with `#[allow(unsafe_code)]` so crates with `#![deny(unsafe_code)]` compile cleanly.
/// On hosted targets (when `not(test)`), emits a standard `fn main()` that exits with
/// the outcome code of `main_fn()`.
#[macro_export]
macro_rules! process_entry {
    ($main_fn:path) => {
        #[cfg(all(feature = "libc-rs", target_arch = "aarch64", target_os = "none"))]
        #[allow(unsafe_code)]
        mod _eko_process_entry {
            use super::*;
            ::libc_rs::process_entry!($main_fn);
        }

        #[cfg(not(all(feature = "libc-rs", target_arch = "aarch64", target_os = "none")))]
        #[cfg(not(test))]
        #[allow(unsafe_code)]
        fn main() {
            let code = $crate::rt::EntryOutcome::into_exit_code($main_fn());
            if code != 0 {
                $crate::exit(code);
            }
        }
    };
}

pub use crate::process_entry;
