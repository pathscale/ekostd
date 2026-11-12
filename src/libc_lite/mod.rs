//! The declarations of the system C library that this crate uses, and nothing else.
//!
//! ekostd is a wrapper around the platform's C library. It builds no C library and implements none
//! of these functions: every item here is a declaration of something the system already provides.
//! The third-party `libc` crate carried the same declarations for every platform there is, with a
//! build script. This module carries the ones ekostd uses, for the targets it is built for, with none.
//!
//! `tests/libc_lite_parity_*.rs` compare every item with the `libc` crate at compile time.

#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "aarch64")))]
mod file;
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "aarch64")))]
mod memnet;
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "aarch64")))]
mod process;
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "aarch64")))]
mod thread;

#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "aarch64")))]
pub use self::{file::*, memnet::*, process::*, thread::*};

// Targets the declarations have not been written and checked for yet keep the crate.
#[cfg(not(any(target_os = "macos", all(target_os = "linux", target_arch = "aarch64"))))]
pub use ::libc::*;
