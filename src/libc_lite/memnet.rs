use super::*;

use core::ffi::{c_char, c_int, c_void};

// Layout and values below are copied from the `libc` crate, 0.2.189, per target. The `libc` crate
// path of each one is in the parity test's comment, `tests/libc_lite_parity_memnet.rs`.

#[allow(non_camel_case_types)]
pub type socklen_t = u32;

#[cfg(target_os = "macos")]
#[allow(non_camel_case_types)]
pub type time_t = core::ffi::c_long;
#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[allow(non_camel_case_types)]
pub type time_t = i64;
#[cfg(all(target_os = "linux", target_env = "musl"))]
#[allow(non_camel_case_types)]
pub type time_t = core::ffi::c_long;

#[cfg(target_os = "macos")]
#[allow(non_camel_case_types)]
type sa_family_t = u8;
#[cfg(target_os = "linux")]
#[allow(non_camel_case_types)]
type sa_family_t = u16;

/// The generic address that `bind`, `connect` and `accept` take a pointer to. Kept in a private
/// module: it is part of those signatures and of nothing else this crate says.
mod abi {
    use super::sa_family_t;
    use core::ffi::c_char;

    #[repr(C)]
    #[derive(Clone, Copy)]
    #[allow(dead_code, non_camel_case_types)]
    pub struct sockaddr {
        #[cfg(target_os = "macos")]
        pub sa_len: u8,
        pub sa_family: sa_family_t,
        pub sa_data: [c_char; 14],
    }
}
use self::abi::sockaddr;

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct sockaddr_un {
    #[cfg(target_os = "macos")]
    pub sun_len: u8,
    pub sun_family: sa_family_t,
    #[cfg(target_os = "macos")]
    pub sun_path: [c_char; 104],
    #[cfg(target_os = "linux")]
    pub sun_path: [c_char; 108],
}

// `AF_UNIX` is `AF_LOCAL` in the `libc` crate, and both are 1 on every target here.
pub const AF_UNIX: c_int = 1;

#[cfg(target_os = "macos")]
pub const ENAMETOOLONG: c_int = 63;
#[cfg(target_os = "linux")]
pub const ENAMETOOLONG: c_int = 36;

pub const EPIPE: c_int = 32;
pub const ENFILE: c_int = 23;
pub const EMFILE: c_int = 24;

#[cfg(target_os = "macos")]
pub const EPROTO: c_int = 100;
#[cfg(target_os = "linux")]
pub const EPROTO: c_int = 71;

#[cfg(target_os = "macos")]
pub const SOL_SOCKET: c_int = 0xffff;
#[cfg(target_os = "linux")]
pub const SOL_SOCKET: c_int = 1;

#[cfg(target_os = "macos")]
pub const SO_RCVTIMEO: c_int = 0x1006;
#[cfg(target_os = "linux")]
pub const SO_RCVTIMEO: c_int = 20;
#[cfg(target_os = "macos")]
pub const SO_SNDTIMEO: c_int = 0x1005;
#[cfg(target_os = "linux")]
pub const SO_SNDTIMEO: c_int = 21;

// Only Apple has `SO_NOSIGPIPE`; the Linux targets use `MSG_NOSIGNAL` on each send instead.
#[cfg(target_os = "macos")]
pub const SO_NOSIGPIPE: c_int = 0x1022;

#[cfg(target_os = "macos")]
pub const MSG_NOSIGNAL: c_int = 0x80000;
#[cfg(target_os = "linux")]
pub const MSG_NOSIGNAL: c_int = 0x4000;

#[cfg(target_os = "macos")]
pub const MAP_ANON: c_int = 0x1000;
#[cfg(target_os = "linux")]
pub const MAP_ANON: c_int = 0x0020;

pub const MAP_FAILED: *mut c_void = !0 as *mut c_void;
pub const MAP_PRIVATE: c_int = 0x0002;

pub const PROT_NONE: c_int = 0;
pub const PROT_READ: c_int = 1;
pub const PROT_WRITE: c_int = 2;
pub const PROT_EXEC: c_int = 4;

pub const SOCK_STREAM: c_int = 1;

#[cfg(target_os = "macos")]
pub const _SC_PAGESIZE: c_int = 29;
#[cfg(target_os = "linux")]
pub const _SC_PAGESIZE: c_int = 30;

unsafe extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn calloc(nobj: usize, size: usize) -> *mut c_void;
    pub fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn posix_memalign(memptr: *mut *mut c_void, align: usize, size: usize) -> c_int;

    pub fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: i64,
    ) -> *mut c_void;
    pub fn munmap(addr: *mut c_void, len: usize) -> c_int;
    pub fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;

    pub fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    pub fn bind(socket: c_int, address: *const sockaddr, address_len: socklen_t) -> c_int;
    pub fn connect(socket: c_int, address: *const sockaddr, len: socklen_t) -> c_int;
    pub fn listen(socket: c_int, backlog: c_int) -> c_int;
    pub fn accept(socket: c_int, address: *mut sockaddr, address_len: *mut socklen_t) -> c_int;
    pub fn dup(fd: c_int) -> c_int;
    pub fn setsockopt(
        socket: c_int,
        level: c_int,
        name: c_int,
        value: *const c_void,
        option_len: socklen_t,
    ) -> c_int;
    pub fn socketpair(domain: c_int, ty: c_int, protocol: c_int, socket_vector: *mut c_int)
    -> c_int;
    pub fn send(socket: c_int, buf: *const c_void, len: usize, flags: c_int) -> isize;
    pub fn recv(socket: c_int, buf: *mut c_void, len: usize, flags: c_int) -> isize;

    pub fn nanosleep(rqtp: *const timespec, rmtp: *mut timespec) -> c_int;

    #[cfg(target_os = "macos")]
    pub fn __error() -> *mut c_int;
    #[cfg(target_os = "linux")]
    pub fn __errno_location() -> *mut c_int;

    // glibc names the XSI form `__xpg_strerror_r`; musl exports `strerror_r` as that form.
    #[cfg_attr(all(target_os = "linux", target_env = "gnu"), link_name = "__xpg_strerror_r")]
    pub fn strerror_r(errnum: c_int, buf: *mut c_char, buflen: usize) -> c_int;
}
