//! `eko::libc_lite` thread area against the `libc` crate. Every comparison is a compile-time
//! assertion, so `cargo check --tests --target T` proves it without running anything.

use core::ffi::c_int;
use core::mem::{align_of, offset_of, size_of};

// Apple makes the clock id unsigned, the Linux targets make it signed.
#[cfg(target_os = "macos")]
type ClockId = core::ffi::c_uint;
#[cfg(not(target_os = "macos"))]
type ClockId = c_int;

macro_rules! same_consts {
    ($($name:ident),* $(,)?) => {
        $( const _: () = assert!(eko::libc_lite::$name == libc::$name); )*
    };
}

same_consts!(CLOCK_MONOTONIC, CLOCK_REALTIME, EINVAL, PTHREAD_CREATE_DETACHED);
// `AT_SECURE` is read from the aux vector, so only the Linux targets have it.
#[cfg(target_os = "linux")]
same_consts!(AT_SECURE);

// Aliases must be the very same type, not just the same size.
macro_rules! same_alias {
    ($($name:ident),* $(,)?) => {
        $( const _: fn(eko::libc_lite::$name) -> libc::$name = |x| x; )*
    };
}

same_alias!(c_ulong, pthread_t, pthread_key_t);
const _: fn(*mut eko::libc_lite::c_void) -> *mut libc::c_void = |p| p;
#[cfg(target_os = "linux")]
same_alias!(pthread_once_t);

macro_rules! same_layout {
    ($($name:ident),* $(,)?) => {
        $(
            const _: () = {
                assert!(size_of::<eko::libc_lite::$name>() == size_of::<libc::$name>());
                assert!(align_of::<eko::libc_lite::$name>() == align_of::<libc::$name>());
            };
        )*
    };
}

same_layout!(
    c_ulong,
    c_void,
    Dl_info,
    pthread_attr_t,
    pthread_cond_t,
    pthread_key_t,
    pthread_mutex_t,
    pthread_once_t,
    pthread_rwlock_t,
    pthread_t,
    timespec,
    timeval,
);

// The only public fields are on `timespec`. The lock types keep their fields private in both
// crates, so their layout is checked by size and alignment above and by initialiser bytes below.
const _: () = {
    assert!(
        offset_of!(eko::libc_lite::timespec, tv_sec) == offset_of!(libc::timespec, tv_sec)
    );
    assert!(
        offset_of!(eko::libc_lite::timespec, tv_nsec) == offset_of!(libc::timespec, tv_nsec)
    );
};

const _: () = {
    assert!(offset_of!(eko::libc_lite::timeval, tv_sec) == offset_of!(libc::timeval, tv_sec));
    assert!(offset_of!(eko::libc_lite::timeval, tv_usec) == offset_of!(libc::timeval, tv_usec));
    let ours: eko::libc_lite::timeval = unsafe { core::mem::zeroed() };
    let theirs: libc::timeval = unsafe { core::mem::zeroed() };
    same_field_type(&ours.tv_sec, &theirs.tv_sec);
    same_field_type(&ours.tv_usec, &theirs.tv_usec);
};

// `Dl_info` is all public fields: every offset and every field type.
const _: () = {
    assert!(
        offset_of!(eko::libc_lite::Dl_info, dli_fname) == offset_of!(libc::Dl_info, dli_fname)
    );
    assert!(
        offset_of!(eko::libc_lite::Dl_info, dli_fbase) == offset_of!(libc::Dl_info, dli_fbase)
    );
    assert!(
        offset_of!(eko::libc_lite::Dl_info, dli_sname) == offset_of!(libc::Dl_info, dli_sname)
    );
    assert!(
        offset_of!(eko::libc_lite::Dl_info, dli_saddr) == offset_of!(libc::Dl_info, dli_saddr)
    );
    let ours: eko::libc_lite::Dl_info = unsafe { core::mem::zeroed() };
    let theirs: libc::Dl_info = unsafe { core::mem::zeroed() };
    same_field_type(&ours.dli_fname, &theirs.dli_fname);
    same_field_type(&ours.dli_fbase, &theirs.dli_fbase);
    same_field_type(&ours.dli_sname, &theirs.dli_sname);
    same_field_type(&ours.dli_saddr, &theirs.dli_saddr);
};

const fn same_field_type<T>(_: &T, _: &T) {}

const fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

macro_rules! same_initialiser {
    ($ty:ident, $init:ident) => {
        const _: () = {
            const N: usize = size_of::<libc::$ty>();
            let ours = unsafe {
                core::mem::transmute::<eko::libc_lite::$ty, [u8; N]>(eko::libc_lite::$init)
            };
            let theirs = unsafe { core::mem::transmute::<libc::$ty, [u8; N]>(libc::$init) };
            assert!(bytes_eq(&ours, &theirs));
        };
    };
}

same_initialiser!(pthread_mutex_t, PTHREAD_MUTEX_INITIALIZER);
same_initialiser!(pthread_cond_t, PTHREAD_COND_INITIALIZER);
same_initialiser!(pthread_rwlock_t, PTHREAD_RWLOCK_INITIALIZER);

// Each side is coerced to one explicit `unsafe extern "C" fn` type, written once in the macro.
// The lock types are different nominal types in the two crates, so the type is instantiated once
// per side: a call that did not match the written signature would not compile on that side.
macro_rules! fn_signatures {
    ($c:ident) => {
        const _: unsafe extern "C" fn($crate::ClockId, *mut $c::timespec) -> c_int =
            $c::clock_gettime;
        const _: unsafe extern "C" fn() -> c_int = $c::sched_yield;
        const _: unsafe extern "C" fn() -> $c::pthread_t = $c::pthread_self;
        const _: unsafe extern "C" fn(
            *mut $c::pthread_t,
            *const $c::pthread_attr_t,
            extern "C" fn(*mut $c::c_void) -> *mut $c::c_void,
            *mut $c::c_void,
        ) -> c_int = $c::pthread_create;
        const _: unsafe extern "C" fn($c::pthread_t, *mut *mut $c::c_void) -> c_int =
            $c::pthread_join;

        const _: unsafe extern "C" fn(*mut $c::pthread_attr_t) -> c_int = $c::pthread_attr_init;
        const _: unsafe extern "C" fn(*mut $c::pthread_attr_t) -> c_int =
            $c::pthread_attr_destroy;
        const _: unsafe extern "C" fn(*mut $c::pthread_attr_t, usize) -> c_int =
            $c::pthread_attr_setstacksize;
        const _: unsafe extern "C" fn(*mut $c::pthread_attr_t, c_int) -> c_int =
            $c::pthread_attr_setdetachstate;

        const _: unsafe extern "C" fn(
            *mut $c::pthread_key_t,
            Option<unsafe extern "C" fn(*mut $c::c_void)>,
        ) -> c_int = $c::pthread_key_create;
        const _: unsafe extern "C" fn($c::pthread_key_t) -> c_int = $c::pthread_key_delete;
        const _: unsafe extern "C" fn($c::pthread_key_t) -> *mut $c::c_void =
            $c::pthread_getspecific;
        const _: unsafe extern "C" fn($c::pthread_key_t, *const $c::c_void) -> c_int =
            $c::pthread_setspecific;

        const _: unsafe extern "C" fn(*mut $c::pthread_mutex_t) -> c_int =
            $c::pthread_mutex_destroy;
        const _: unsafe extern "C" fn(*mut $c::pthread_mutex_t) -> c_int = $c::pthread_mutex_lock;
        const _: unsafe extern "C" fn(*mut $c::pthread_mutex_t) -> c_int =
            $c::pthread_mutex_trylock;
        const _: unsafe extern "C" fn(*mut $c::pthread_mutex_t) -> c_int =
            $c::pthread_mutex_unlock;

        const _: unsafe extern "C" fn(*mut $c::pthread_cond_t, *mut $c::pthread_mutex_t) -> c_int =
            $c::pthread_cond_wait;
        const _: unsafe extern "C" fn(*mut $c::pthread_cond_t) -> c_int = $c::pthread_cond_signal;
        const _: unsafe extern "C" fn(*mut $c::pthread_cond_t) -> c_int =
            $c::pthread_cond_broadcast;
        const _: unsafe extern "C" fn(*mut $c::pthread_cond_t) -> c_int = $c::pthread_cond_destroy;

        const _: unsafe extern "C" fn(*mut $c::pthread_rwlock_t) -> c_int =
            $c::pthread_rwlock_rdlock;
        const _: unsafe extern "C" fn(*mut $c::pthread_rwlock_t) -> c_int =
            $c::pthread_rwlock_wrlock;
        const _: unsafe extern "C" fn(*mut $c::pthread_rwlock_t) -> c_int =
            $c::pthread_rwlock_unlock;

        #[cfg(target_os = "macos")]
        const _: unsafe extern "C" fn(
            *const core::ffi::c_char,
            *mut $c::c_void,
            *mut usize,
            *mut $c::c_void,
            usize,
        ) -> c_int = $c::sysctlbyname;
        #[cfg(target_os = "linux")]
        const _: unsafe extern "C" fn($c::c_ulong) -> $c::c_ulong = $c::getauxval;

        const _: unsafe extern "C" fn(*const $c::c_void, *mut $c::Dl_info) -> c_int = $c::dladdr;
        #[cfg(target_os = "macos")]
        const _: unsafe extern "C" fn($c::pthread_t) -> *mut $c::c_void =
            $c::pthread_get_stackaddr_np;
        #[cfg(target_os = "macos")]
        const _: unsafe extern "C" fn($c::pthread_t) -> usize = $c::pthread_get_stacksize_np;
        #[cfg(target_os = "linux")]
        const _: unsafe extern "C" fn($c::pthread_t, *mut $c::pthread_attr_t) -> c_int =
            $c::pthread_getattr_np;
        #[cfg(target_os = "linux")]
        const _: unsafe extern "C" fn(
            *const $c::pthread_attr_t,
            *mut *mut $c::c_void,
            *mut usize,
        ) -> c_int = $c::pthread_attr_getstack;
    };
}

mod ours {
    use super::c_int;
    use eko::libc_lite as c;
    fn_signatures!(c);
}

mod theirs {
    use super::c_int;
    use libc as c;
    fn_signatures!(c);
}

#[test]
fn libc_lite_thread_matches_the_libc_crate() {
    // Every assertion above is evaluated at compile time. Compiling this file is the check.
}
