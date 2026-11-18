//! The declarations of the system C library that this crate uses, and nothing else.
//!
//! ekostd is a wrapper around the platform's C library. It builds no C library and implements none
//! of these functions: every item here is a declaration of something the system already provides.
//! The third-party `libc` crate carried the same declarations for every platform there is, with a
//! build script. This module carries the ones ekostd uses, for the targets it is built for, with none.
//!
//! `tests/libc_lite_parity_*.rs` compare every item with the `libc` crate at compile time.

#![allow(non_camel_case_types)]

#[cfg(target_os = "none")]
mod stub_libc_rs;
#[cfg(target_os = "none")]
pub use self::stub_libc_rs::*;

#[cfg(all(
    not(target_os = "none"),
    any(
        target_os = "macos",
        all(target_os = "linux", any(target_arch = "aarch64", target_arch = "x86_64"))
    )
))]
mod file;
#[cfg(all(
    not(target_os = "none"),
    any(
        target_os = "macos",
        all(target_os = "linux", any(target_arch = "aarch64", target_arch = "x86_64"))
    )
))]
mod memnet;
#[cfg(all(
    not(target_os = "none"),
    any(
        target_os = "macos",
        all(target_os = "linux", any(target_arch = "aarch64", target_arch = "x86_64"))
    )
))]
mod process;
#[cfg(all(
    not(target_os = "none"),
    any(
        target_os = "macos",
        all(target_os = "linux", any(target_arch = "aarch64", target_arch = "x86_64"))
    )
))]
mod thread;

#[cfg(all(
    not(target_os = "none"),
    any(
        target_os = "macos",
        all(target_os = "linux", any(target_arch = "aarch64", target_arch = "x86_64"))
    )
))]
pub use self::{file::*, memnet::*, process::*, thread::*};

#[cfg(not(target_os = "none"))]
#[inline]
pub unsafe fn open2(path: *const c_char, oflag: c_int) -> c_int {
    unsafe { open(path, oflag) }
}

#[cfg(not(target_os = "none"))]
#[inline]
pub unsafe fn open3(path: *const c_char, oflag: c_int, mode: mode_t) -> c_int {
    unsafe { open(path, oflag, mode as c_int) }
}

#[cfg(not(target_os = "none"))]
#[inline]
pub unsafe fn fcntl2(fd: c_int, cmd: c_int) -> c_int {
    unsafe { fcntl(fd, cmd) }
}

#[cfg(not(target_os = "none"))]
#[inline]
pub unsafe fn fcntl3(fd: c_int, cmd: c_int, arg: c_int) -> c_int {
    unsafe { fcntl(fd, cmd, arg) }
}

#[cfg(not(target_os = "none"))]
#[inline]
pub unsafe fn fcntl_flock(fd: c_int, cmd: c_int, lock: *const flock) -> c_int {
    unsafe { fcntl(fd, cmd, lock) }
}

#[cfg(not(target_os = "none"))]
#[inline]
pub unsafe fn ioctl_ptr(fd: c_int, request: u64, arg: usize) -> c_int {
    #[cfg(not(target_env = "musl"))]
    {
        unsafe { ioctl(fd, request as c_ulong, arg) }
    }
    #[cfg(target_env = "musl")]
    {
        unsafe { ioctl(fd, request as c_int, arg) }
    }
}
