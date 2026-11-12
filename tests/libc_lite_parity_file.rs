//! `libc_lite` file declarations against the `libc` crate.
//!
//! Every comparison is a `const` assertion or a `const` of an explicit function pointer type, so
//! `cargo check --tests --target T` proves it without running anything.

use core::ffi::{c_char, c_int, c_void};
use core::mem::{align_of, offset_of, size_of};

macro_rules! same_const {
    ($($name:ident),* $(,)?) => {
        $( const _: () = assert!(eko::libc_lite::$name == libc::$name); )*
    };
}

same_const!(
    EACCES, EEXIST, EFBIG, EILSEQ, EINTR, EISDIR, ENOENT, EPERM, F_OK, O_APPEND, O_CREAT,
    O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY, PATH_MAX, SEEK_CUR, SEEK_SET, S_IFDIR, S_IFMT, S_IFREG,
);

same_const!(ENOSYS, ENOTSUP, EOPNOTSUPP, EXDEV, O_EXCL, O_NOFOLLOW);

// The aliases must be the very same type, not only the same size.
const _: Option<libc::c_char> = None::<eko::libc_lite::c_char>;
const _: Option<libc::c_int> = None::<eko::libc_lite::c_int>;
const _: Option<libc::mode_t> = None::<eko::libc_lite::mode_t>;
const _: Option<libc::off_t> = None::<eko::libc_lite::off_t>;
const _: Option<libc::c_short> = None::<eko::libc_lite::c_short>;
const _: Option<libc::uid_t> = None::<eko::libc_lite::uid_t>;

macro_rules! same_layout {
    ($($ty:ident),* $(,)?) => {
        $(
            const _: () = {
                assert!(size_of::<eko::libc_lite::$ty>() == size_of::<libc::$ty>());
                assert!(align_of::<eko::libc_lite::$ty>() == align_of::<libc::$ty>());
            };
        )*
    };
}

same_layout!(DIR, stat, dirent);

macro_rules! same_offsets {
    ($ty:ident: $($field:ident),* $(,)?) => {
        $(
            const _: () = assert!(
                offset_of!(eko::libc_lite::$ty, $field) == offset_of!(libc::$ty, $field)
            );
        )*
    };
}

#[cfg(target_os = "macos")]
same_offsets!(
    stat: st_dev, st_mode, st_nlink, st_ino, st_uid, st_gid, st_rdev, st_atime, st_atime_nsec,
    st_mtime, st_mtime_nsec, st_ctime, st_ctime_nsec, st_birthtime, st_birthtime_nsec, st_size,
    st_blocks, st_blksize, st_flags, st_gen, st_lspare, st_qspare,
);
#[cfg(target_os = "macos")]
same_offsets!(dirent: d_ino, d_seekoff, d_reclen, d_namlen, d_type, d_name);

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
same_offsets!(
    stat: st_dev, st_ino, st_mode, st_nlink, st_uid, st_gid, st_rdev, st_size, st_blksize,
    st_blocks, st_atime, st_atime_nsec, st_mtime, st_mtime_nsec, st_ctime, st_ctime_nsec,
);
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
same_offsets!(dirent: d_ino, d_off, d_reclen, d_type, d_name);

// Functions whose signature names no `libc_lite` struct: one function pointer type, two constants.
macro_rules! same_fn {
    ($ty:ty: $($name:ident),* $(,)?) => {
        $(
            const _: $ty = eko::libc_lite::$name;
            const _: $ty = libc::$name;
        )*
    };
}

same_fn!(unsafe extern "C" fn(*const c_char, *const c_char) -> c_int: rename, link);
same_fn!(unsafe extern "C" fn(*const c_char) -> *mut c_char: getenv);
same_fn!(unsafe extern "C" fn(*const c_char, libc::mode_t) -> c_int: mkdir);
same_fn!(unsafe extern "C" fn(*const c_char, c_int, ...) -> c_int: open);
same_fn!(unsafe extern "C" fn(*const c_char, c_int) -> c_int: access);
same_fn!(unsafe extern "C" fn(c_int) -> c_int: close, isatty);
same_fn!(unsafe extern "C" fn(*mut c_char, usize) -> *mut c_char: getcwd);
same_fn!(unsafe extern "C" fn() -> i32: getpid);
same_fn!(unsafe extern "C" fn(c_int, libc::off_t, c_int) -> libc::off_t: lseek);
same_fn!(unsafe extern "C" fn(c_int, *mut c_void, usize) -> isize: read);
same_fn!(unsafe extern "C" fn(c_int, *const c_void, usize) -> isize: write);
same_fn!(unsafe extern "C" fn(*const c_char) -> c_int: rmdir, unlink);
same_fn!(unsafe extern "C" fn(*const c_char, *mut c_char, usize) -> isize: readlink);
same_fn!(unsafe extern "C" fn(c_int) -> c_int: fsync);
same_fn!(unsafe extern "C" fn(c_int, libc::mode_t) -> c_int: fchmod);
same_fn!(unsafe extern "C" fn() -> libc::uid_t: geteuid);

// Functions that take or return a `libc_lite` struct. The struct is a different Rust type on each
// side, so the one explicit function pointer type is generic over it and instantiated per side;
// the structs themselves are compared above by size, alignment and field offsets.
type FstatFn<S> = unsafe extern "C" fn(c_int, *mut S) -> c_int;
type PathStatFn<S> = unsafe extern "C" fn(*const c_char, *mut S) -> c_int;
type OpendirFn<D> = unsafe extern "C" fn(*const c_char) -> *mut D;
type ReaddirFn<D, E> = unsafe extern "C" fn(*mut D) -> *mut E;
type ClosedirFn<D> = unsafe extern "C" fn(*mut D) -> c_int;

const _: FstatFn<eko::libc_lite::stat> = eko::libc_lite::fstat;
const _: FstatFn<libc::stat> = libc::fstat;
const _: PathStatFn<eko::libc_lite::stat> = eko::libc_lite::stat;
const _: PathStatFn<libc::stat> = libc::stat;
const _: PathStatFn<eko::libc_lite::stat> = eko::libc_lite::lstat;
const _: PathStatFn<libc::stat> = libc::lstat;
const _: OpendirFn<eko::libc_lite::DIR> = eko::libc_lite::opendir;
const _: OpendirFn<libc::DIR> = libc::opendir;
const _: ReaddirFn<eko::libc_lite::DIR, eko::libc_lite::dirent> = eko::libc_lite::readdir;
const _: ReaddirFn<libc::DIR, libc::dirent> = libc::readdir;
const _: ClosedirFn<eko::libc_lite::DIR> = eko::libc_lite::closedir;
const _: ClosedirFn<libc::DIR> = libc::closedir;

#[test]
fn libc_lite_file_matches_the_libc_crate() {
    // Every assertion above is a compile-time assertion; `cargo check --tests` is the proof.
}
