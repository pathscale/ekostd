//! `eko::libc_lite` against the `libc` crate, for the memory and socket items.
//!
//! Every comparison is a compile-time assertion, so `cargo check --tests --target T` proves it
//! without running anything. Constants are compared by value, types by size, alignment and field
//! offsets, functions by two constants of one explicit function pointer type, one from each side.
//!
//! Where `libc_lite` and `libc` name different types for the same pointer (`sockaddr`, `timespec`),
//! both sides are coerced to a function pointer with the pointer type kept open, then turned into
//! one type with erased pointers. The integer arguments and the return type still have to match.
#![cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "aarch64")))]

use core::ffi::{c_char, c_int, c_void};
use core::mem::{align_of, offset_of, size_of};

// Constants. Each one is compared by value, which also fixes its type.
const _: () = assert!(eko::libc_lite::AF_UNIX == libc::AF_UNIX);
const _: () = assert!(eko::libc_lite::ENAMETOOLONG == libc::ENAMETOOLONG);
const _: () = assert!(eko::libc_lite::EPIPE == libc::EPIPE);
const _: () = assert!(eko::libc_lite::MAP_ANON == libc::MAP_ANON);
const _: () = assert!(eko::libc_lite::MAP_PRIVATE == libc::MAP_PRIVATE);
const _: () = assert!(eko::libc_lite::PROT_EXEC == libc::PROT_EXEC);
const _: () = assert!(eko::libc_lite::PROT_NONE == libc::PROT_NONE);
const _: () = assert!(eko::libc_lite::PROT_READ == libc::PROT_READ);
const _: () = assert!(eko::libc_lite::PROT_WRITE == libc::PROT_WRITE);
const _: () = assert!(eko::libc_lite::SOCK_STREAM == libc::SOCK_STREAM);
const _: () = assert!(eko::libc_lite::_SC_PAGESIZE == libc::_SC_PAGESIZE);
const _: () = assert!(eko::libc_lite::MAP_FIXED == libc::MAP_FIXED);
const _: () = assert!(eko::libc_lite::MAP_NORESERVE == libc::MAP_NORESERVE);
const _: () = assert!(eko::libc_lite::MADV_DONTNEED == libc::MADV_DONTNEED);
const _: () = assert!(eko::libc_lite::MADV_FREE == libc::MADV_FREE);
// Only Apple has `VM_PROT_NONE` in the `libc` crate for these targets.
#[cfg(target_os = "macos")]
const _: () = assert!(eko::libc_lite::VM_PROT_NONE == libc::VM_PROT_NONE);
const _: () = assert!(eko::libc_lite::EMFILE == libc::EMFILE);
const _: () = assert!(eko::libc_lite::ENFILE == libc::ENFILE);
const _: () = assert!(eko::libc_lite::EPROTO == libc::EPROTO);
const _: () = assert!(eko::libc_lite::MSG_NOSIGNAL == libc::MSG_NOSIGNAL);
const _: () = assert!(eko::libc_lite::SOL_SOCKET == libc::SOL_SOCKET);
const _: () = assert!(eko::libc_lite::SO_RCVTIMEO == libc::SO_RCVTIMEO);
const _: () = assert!(eko::libc_lite::SO_SNDTIMEO == libc::SO_SNDTIMEO);
// Only Apple has `SO_NOSIGPIPE`.
#[cfg(target_os = "macos")]
const _: () = assert!(eko::libc_lite::SO_NOSIGPIPE == libc::SO_NOSIGPIPE);

// A pointer constant: same type, same address.
const _: *mut c_void = eko::libc_lite::MAP_FAILED;
// Both sides are `!0 as *mut c_void`, a pointer without provenance, so const evaluation can read
// its address as an integer. `<*mut T>::addr` is not a const fn.
const _: () = assert!(unsafe {
    core::mem::transmute::<*mut c_void, usize>(eko::libc_lite::MAP_FAILED)
        == core::mem::transmute::<*mut c_void, usize>(libc::MAP_FAILED)
});

// Type aliases: the identity function only compiles when the two are one type.
// The allocator's names: the identity function only compiles when the two are one type.
const _: fn(eko::libc_lite::size_t) -> libc::size_t = |x| x;
const _: () = assert!(size_of::<eko::libc_lite::size_t>() == size_of::<libc::size_t>());
const _: () = assert!(align_of::<eko::libc_lite::size_t>() == align_of::<libc::size_t>());

#[cfg(target_os = "macos")]
macro_rules! same_mach_alias {
    ($($name:ident),* $(,)?) => {
        $(
            const _: fn(eko::libc_lite::$name) -> libc::$name = |x| x;
            const _: () = assert!(size_of::<eko::libc_lite::$name>() == size_of::<libc::$name>());
            const _: () = assert!(align_of::<eko::libc_lite::$name>() == align_of::<libc::$name>());
        )*
    };
}
#[cfg(target_os = "macos")]
same_mach_alias!(mach_port_t, mach_vm_address_t, mach_vm_size_t, vm_prot_t);

const fn same_socklen(x: eko::libc_lite::socklen_t) -> libc::socklen_t {
    x
}
const _: fn(eko::libc_lite::socklen_t) -> libc::socklen_t = same_socklen;
const _: () = assert!(size_of::<eko::libc_lite::socklen_t>() == size_of::<libc::socklen_t>());
const _: () = assert!(align_of::<eko::libc_lite::socklen_t>() == align_of::<libc::socklen_t>());

const fn same_time(x: eko::libc_lite::time_t) -> libc::time_t {
    x
}
const _: fn(eko::libc_lite::time_t) -> libc::time_t = same_time;
const _: () = assert!(size_of::<eko::libc_lite::time_t>() == size_of::<libc::time_t>());
const _: () = assert!(align_of::<eko::libc_lite::time_t>() == align_of::<libc::time_t>());

// `sockaddr_un`: size, alignment and every field offset.
const _: () =
    assert!(size_of::<eko::libc_lite::sockaddr_un>() == size_of::<libc::sockaddr_un>());
const _: () =
    assert!(align_of::<eko::libc_lite::sockaddr_un>() == align_of::<libc::sockaddr_un>());
#[cfg(target_os = "macos")]
const _: () = assert!(
    offset_of!(eko::libc_lite::sockaddr_un, sun_len) == offset_of!(libc::sockaddr_un, sun_len)
);
const _: () = assert!(
    offset_of!(eko::libc_lite::sockaddr_un, sun_family)
        == offset_of!(libc::sockaddr_un, sun_family)
);
const _: () = assert!(
    offset_of!(eko::libc_lite::sockaddr_un, sun_path) == offset_of!(libc::sockaddr_un, sun_path)
);

// Functions whose signature names only integers, `c_void` and `c_char`: one explicit type, two
// constants.
macro_rules! same_fn {
    ($name:ident, $ty:ty) => {
        const _: $ty = eko::libc_lite::$name;
        const _: $ty = libc::$name;
    };
}

same_fn!(malloc, unsafe extern "C" fn(usize) -> *mut c_void);
same_fn!(calloc, unsafe extern "C" fn(usize, usize) -> *mut c_void);
same_fn!(realloc, unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void);
same_fn!(free, unsafe extern "C" fn(*mut c_void));
same_fn!(posix_memalign, unsafe extern "C" fn(*mut *mut c_void, usize, usize) -> c_int);
same_fn!(
    mmap,
    unsafe extern "C" fn(*mut c_void, usize, c_int, c_int, c_int, i64) -> *mut c_void
);
same_fn!(munmap, unsafe extern "C" fn(*mut c_void, usize) -> c_int);
same_fn!(mprotect, unsafe extern "C" fn(*mut c_void, usize, c_int) -> c_int);
same_fn!(madvise, unsafe extern "C" fn(*mut c_void, usize, c_int) -> c_int);
// Apple takes a `const` address and a `c_char` vector. Linux takes a `mut` address and `c_uchar`.
#[cfg(target_os = "macos")]
same_fn!(mincore, unsafe extern "C" fn(*const c_void, usize, *mut c_char) -> c_int);
#[cfg(target_os = "linux")]
same_fn!(mincore, unsafe extern "C" fn(*mut c_void, usize, *mut core::ffi::c_uchar) -> c_int);
same_fn!(socket, unsafe extern "C" fn(c_int, c_int, c_int) -> c_int);
same_fn!(listen, unsafe extern "C" fn(c_int, c_int) -> c_int);
same_fn!(dup, unsafe extern "C" fn(c_int) -> c_int);
same_fn!(strerror_r, unsafe extern "C" fn(c_int, *mut c_char, usize) -> c_int);
same_fn!(
    setsockopt,
    unsafe extern "C" fn(c_int, c_int, c_int, *const c_void, u32) -> c_int
);
same_fn!(socketpair, unsafe extern "C" fn(c_int, c_int, c_int, *mut c_int) -> c_int);
same_fn!(send, unsafe extern "C" fn(c_int, *const c_void, usize, c_int) -> isize);
same_fn!(recv, unsafe extern "C" fn(c_int, *mut c_void, usize, c_int) -> isize);
#[cfg(target_os = "macos")]
same_fn!(__error, unsafe extern "C" fn() -> *mut c_int);
#[cfg(target_os = "linux")]
same_fn!(__errno_location, unsafe extern "C" fn() -> *mut c_int);

// Functions that take a pointer to `sockaddr` or `timespec`. Each side is first coerced with the
// pointee left open, then both are turned into the same type with `c_void` pointers.
macro_rules! same_fn_erased {
    (
        $name:ident,
        ours: $ours:ty,
        theirs: $theirs:ty,
        erased: $erased:ty $(,)?
    ) => {
        const _: $erased = unsafe {
            let f: $ours = eko::libc_lite::$name;
            core::mem::transmute(f)
        };
        const _: $erased = unsafe {
            let f: $theirs = libc::$name;
            core::mem::transmute(f)
        };
    };
}

same_fn_erased!(
    bind,
    ours: unsafe extern "C" fn(c_int, *const _, u32) -> c_int,
    theirs: unsafe extern "C" fn(c_int, *const libc::sockaddr, libc::socklen_t) -> c_int,
    erased: unsafe extern "C" fn(c_int, *const c_void, u32) -> c_int,
);
same_fn_erased!(
    connect,
    ours: unsafe extern "C" fn(c_int, *const _, u32) -> c_int,
    theirs: unsafe extern "C" fn(c_int, *const libc::sockaddr, libc::socklen_t) -> c_int,
    erased: unsafe extern "C" fn(c_int, *const c_void, u32) -> c_int,
);
same_fn_erased!(
    accept,
    ours: unsafe extern "C" fn(c_int, *mut _, *mut u32) -> c_int,
    theirs: unsafe extern "C" fn(c_int, *mut libc::sockaddr, *mut libc::socklen_t) -> c_int,
    erased: unsafe extern "C" fn(c_int, *mut c_void, *mut u32) -> c_int,
);
same_fn_erased!(
    nanosleep,
    ours: unsafe extern "C" fn(*const _, *mut _) -> c_int,
    theirs: unsafe extern "C" fn(*const libc::timespec, *mut libc::timespec) -> c_int,
    erased: unsafe extern "C" fn(*const c_void, *mut c_void) -> c_int,
);

#[test]
fn libc_lite_memnet_matches_the_libc_crate() {
    // Every comparison in this file is a constant assertion, checked when the file compiles.
}
