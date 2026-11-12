use super::*;

use core::ffi::{c_int, c_long};
pub use core::ffi::{c_ulong, c_void};

// `clockid_t` and `time_t` are not on the item list, so they stay private aliases. Apple makes the
// clock id unsigned and the Linux targets make it signed.
#[cfg(target_os = "macos")]
type clockid_t = core::ffi::c_uint;
#[cfg(target_os = "linux")]
type clockid_t = c_int;

#[cfg(target_os = "macos")]
type time_t = c_long;
#[cfg(all(target_os = "linux", target_env = "gnu"))]
type time_t = i64;
#[cfg(all(target_os = "linux", target_env = "musl"))]
type time_t = c_long;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct timespec {
    pub tv_sec: time_t,
    pub tv_nsec: c_long,
}

// Apple keeps the microseconds in 32 bits, the Linux 64-bit targets in 64.
#[cfg(target_os = "macos")]
type suseconds_t = i32;
#[cfg(target_os = "linux")]
type suseconds_t = i64;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct timeval {
    pub tv_sec: time_t,
    pub tv_usec: suseconds_t,
}

pub const CLOCK_REALTIME: clockid_t = 0;
#[cfg(target_os = "macos")]
pub const CLOCK_MONOTONIC: clockid_t = 6;
#[cfg(target_os = "linux")]
pub const CLOCK_MONOTONIC: clockid_t = 1;

pub const EINVAL: c_int = 22;

#[cfg(target_os = "macos")]
pub const PTHREAD_CREATE_DETACHED: c_int = 2;
#[cfg(target_os = "linux")]
pub const PTHREAD_CREATE_DETACHED: c_int = 1;

// macOS: `pthread_t` is `uintptr_t`, every lock is a signature word and an opaque tail.
#[cfg(target_os = "macos")]
pub type pthread_t = usize;
#[cfg(target_os = "macos")]
pub type pthread_key_t = c_ulong;

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pthread_attr_t {
    __sig: c_long,
    __opaque: [core::ffi::c_char; 56],
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pthread_cond_t {
    __sig: c_long,
    __opaque: [u8; 40],
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pthread_mutex_t {
    __sig: c_long,
    __opaque: [u8; 56],
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pthread_once_t {
    __sig: c_long,
    __opaque: [core::ffi::c_char; 8],
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pthread_rwlock_t {
    __sig: c_long,
    __opaque: [u8; 192],
}

#[cfg(target_os = "macos")]
const MUTEX_SIG_INIT: c_long = 0x32AAABA7;
#[cfg(target_os = "macos")]
const COND_SIG_INIT: c_long = 0x3CB0B1BB;
#[cfg(target_os = "macos")]
const RWLOCK_SIG_INIT: c_long = 0x2DA8B3B4;

#[cfg(target_os = "macos")]
pub const PTHREAD_MUTEX_INITIALIZER: pthread_mutex_t =
    pthread_mutex_t { __sig: MUTEX_SIG_INIT, __opaque: [0; 56] };
#[cfg(target_os = "macos")]
pub const PTHREAD_COND_INITIALIZER: pthread_cond_t =
    pthread_cond_t { __sig: COND_SIG_INIT, __opaque: [0; 40] };
#[cfg(target_os = "macos")]
pub const PTHREAD_RWLOCK_INITIALIZER: pthread_rwlock_t =
    pthread_rwlock_t { __sig: RWLOCK_SIG_INIT, __opaque: [0; 192] };

// Linux: the types are plain byte blocks and the initialisers are all zero.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
pub type pthread_t = c_ulong;
#[cfg(all(target_os = "linux", target_env = "musl"))]
pub type pthread_t = *mut c_void;
#[cfg(target_os = "linux")]
pub type pthread_key_t = core::ffi::c_uint;
#[cfg(target_os = "linux")]
pub type pthread_once_t = c_int;

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pthread_attr_t {
    __size: [usize; 8],
}

#[cfg(all(target_os = "linux", target_env = "musl"))]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pthread_attr_t {
    __size: [u64; 7],
}

#[cfg(target_os = "linux")]
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct pthread_cond_t {
    size: [u8; 48],
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct pthread_mutex_t {
    size: [core::ffi::c_char; 48],
}

#[cfg(all(target_os = "linux", target_env = "musl"))]
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct pthread_mutex_t {
    size: [core::ffi::c_char; 40],
}

#[cfg(target_os = "linux")]
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct pthread_rwlock_t {
    size: [u8; 56],
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
pub const PTHREAD_MUTEX_INITIALIZER: pthread_mutex_t = pthread_mutex_t { size: [0; 48] };
#[cfg(all(target_os = "linux", target_env = "musl"))]
pub const PTHREAD_MUTEX_INITIALIZER: pthread_mutex_t = pthread_mutex_t { size: [0; 40] };
#[cfg(target_os = "linux")]
pub const PTHREAD_COND_INITIALIZER: pthread_cond_t = pthread_cond_t { size: [0; 48] };
#[cfg(target_os = "linux")]
pub const PTHREAD_RWLOCK_INITIALIZER: pthread_rwlock_t = pthread_rwlock_t { size: [0; 56] };

/// The result of `dladdr`: the image and the symbol that hold an address.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Dl_info {
    pub dli_fname: *const core::ffi::c_char,
    pub dli_fbase: *mut c_void,
    pub dli_sname: *const core::ffi::c_char,
    pub dli_saddr: *mut c_void,
}

unsafe extern "C" {
    pub fn clock_gettime(clk_id: clockid_t, tp: *mut timespec) -> c_int;
    pub fn sched_yield() -> c_int;

    pub fn pthread_self() -> pthread_t;
    pub fn pthread_create(
        native: *mut pthread_t,
        attr: *const pthread_attr_t,
        f: extern "C" fn(*mut c_void) -> *mut c_void,
        value: *mut c_void,
    ) -> c_int;
    pub fn pthread_join(native: pthread_t, value: *mut *mut c_void) -> c_int;

    pub fn pthread_attr_init(attr: *mut pthread_attr_t) -> c_int;
    pub fn pthread_attr_destroy(attr: *mut pthread_attr_t) -> c_int;
    pub fn pthread_attr_setstacksize(attr: *mut pthread_attr_t, stack_size: usize) -> c_int;
    pub fn pthread_attr_setdetachstate(attr: *mut pthread_attr_t, state: c_int) -> c_int;

    pub fn pthread_key_create(
        key: *mut pthread_key_t,
        dtor: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> c_int;
    pub fn pthread_key_delete(key: pthread_key_t) -> c_int;
    pub fn pthread_getspecific(key: pthread_key_t) -> *mut c_void;
    pub fn pthread_setspecific(key: pthread_key_t, value: *const c_void) -> c_int;

    pub fn pthread_mutex_destroy(lock: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_mutex_lock(lock: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_mutex_trylock(lock: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_mutex_unlock(lock: *mut pthread_mutex_t) -> c_int;

    pub fn pthread_cond_wait(cond: *mut pthread_cond_t, lock: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_cond_signal(cond: *mut pthread_cond_t) -> c_int;
    pub fn pthread_cond_broadcast(cond: *mut pthread_cond_t) -> c_int;
    pub fn pthread_cond_destroy(cond: *mut pthread_cond_t) -> c_int;

    pub fn pthread_rwlock_rdlock(lock: *mut pthread_rwlock_t) -> c_int;
    pub fn pthread_rwlock_wrlock(lock: *mut pthread_rwlock_t) -> c_int;
    pub fn pthread_rwlock_unlock(lock: *mut pthread_rwlock_t) -> c_int;

    pub fn dladdr(addr: *const c_void, info: *mut Dl_info) -> c_int;
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    pub fn sysctlbyname(
        name: *const core::ffi::c_char,
        oldp: *mut c_void,
        oldlenp: *mut usize,
        newp: *mut c_void,
        newlen: usize,
    ) -> c_int;

    pub fn pthread_get_stackaddr_np(thread: pthread_t) -> *mut c_void;
    pub fn pthread_get_stacksize_np(thread: pthread_t) -> usize;
}

#[cfg(target_os = "linux")]
pub const AT_SECURE: c_ulong = 23;

#[cfg(target_os = "linux")]
unsafe extern "C" {
    pub fn getauxval(type_: c_ulong) -> c_ulong;

    pub fn pthread_getattr_np(native: pthread_t, attr: *mut pthread_attr_t) -> c_int;
    pub fn pthread_attr_getstack(
        attr: *const pthread_attr_t,
        stackaddr: *mut *mut c_void,
        stacksize: *mut usize,
    ) -> c_int;
}
