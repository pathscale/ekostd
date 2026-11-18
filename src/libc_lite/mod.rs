//! The declarations of the system C library that this crate uses, and nothing else.
//!
//! ekostd is a wrapper around the platform's C library. It builds no C library and implements none
//! of these functions: every item here is a declaration of something the system already provides.
//! The third-party `libc` crate carried the same declarations for every platform there is, with a
//! build script. This module carries the ones ekostd uses, for the targets it is built for, with none.
//!
//! `tests/libc_lite_parity_*.rs` compare every item with the `libc` crate at compile time.

#![allow(non_camel_case_types)]

#[cfg(feature = "libc-rs")]
mod stub_libc_rs;
#[cfg(feature = "libc-rs")]
pub use self::stub_libc_rs::*;

#[cfg(all(
    not(feature = "libc-rs"),
    any(
        target_os = "macos",
        all(target_os = "linux", any(target_arch = "aarch64", target_arch = "x86_64"))
    )
))]
mod file;
#[cfg(all(
    not(feature = "libc-rs"),
    any(
        target_os = "macos",
        all(target_os = "linux", any(target_arch = "aarch64", target_arch = "x86_64"))
    )
))]
mod memnet;
#[cfg(all(
    not(feature = "libc-rs"),
    any(
        target_os = "macos",
        all(target_os = "linux", any(target_arch = "aarch64", target_arch = "x86_64"))
    )
))]
mod process;
#[cfg(all(
    not(feature = "libc-rs"),
    any(
        target_os = "macos",
        all(target_os = "linux", any(target_arch = "aarch64", target_arch = "x86_64"))
    )
))]
mod thread;

#[cfg(all(
    not(feature = "libc-rs"),
    any(
        target_os = "macos",
        all(target_os = "linux", any(target_arch = "aarch64", target_arch = "x86_64"))
    )
))]
pub use self::{file::*, memnet::*, process::*, thread::*};

#[cfg(not(feature = "libc-rs"))]
#[inline]
pub unsafe fn open2(path: *const c_char, oflag: c_int) -> c_int {
    unsafe { open(path, oflag) }
}

#[cfg(not(feature = "libc-rs"))]
#[inline]
pub unsafe fn open3(path: *const c_char, oflag: c_int, mode: mode_t) -> c_int {
    unsafe { open(path, oflag, mode as c_int) }
}

#[cfg(not(feature = "libc-rs"))]
#[inline]
pub unsafe fn fcntl2(fd: c_int, cmd: c_int) -> c_int {
    unsafe { fcntl(fd, cmd) }
}

#[cfg(not(feature = "libc-rs"))]
#[inline]
pub unsafe fn fcntl3(fd: c_int, cmd: c_int, arg: c_int) -> c_int {
    unsafe { fcntl(fd, cmd, arg) }
}

#[cfg(not(feature = "libc-rs"))]
#[inline]
pub unsafe fn fcntl_flock(fd: c_int, cmd: c_int, lock: *const flock) -> c_int {
    unsafe { fcntl(fd, cmd, lock) }
}
