#[allow(unused_imports)]
use super::*;

use core::ffi::{c_long, c_void};

pub use core::ffi::{c_char, c_int, c_short};

pub type off_t = i64;
pub type uid_t = u32;
pub type gid_t = u32;

#[cfg(target_os = "macos")]
pub type mode_t = u16;
#[cfg(target_os = "linux")]
pub type mode_t = u32;

/// An open directory stream. Opaque: only ever behind a pointer.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DIR {
    _data: (),
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

#[cfg(target_os = "macos")]
#[allow(non_camel_case_types, dead_code)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stat {
    pub st_dev: i32,
    pub st_mode: mode_t,
    pub st_nlink: u16,
    pub st_ino: u64,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: i32,
    pub st_atime: i64,
    pub st_atime_nsec: c_long,
    pub st_mtime: i64,
    pub st_mtime_nsec: c_long,
    pub st_ctime: i64,
    pub st_ctime_nsec: c_long,
    pub st_birthtime: i64,
    pub st_birthtime_nsec: c_long,
    pub st_size: off_t,
    pub st_blocks: i64,
    pub st_blksize: i32,
    pub st_flags: u32,
    pub st_gen: u32,
    pub st_lspare: i32,
    pub st_qspare: [i64; 2],
}

#[cfg(target_os = "linux")]
#[allow(non_camel_case_types, dead_code)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: mode_t,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    __pad1: u64,
    pub st_size: off_t,
    pub st_blksize: i32,
    __pad2: c_int,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atime_nsec: c_long,
    pub st_mtime: i64,
    pub st_mtime_nsec: c_long,
    pub st_ctime: i64,
    pub st_ctime_nsec: c_long,
    __unused: [c_int; 2],
}

#[cfg(target_os = "macos")]
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct dirent {
    pub d_ino: u64,
    pub d_seekoff: u64,
    pub d_reclen: u16,
    pub d_namlen: u16,
    pub d_type: u8,
    pub d_name: [c_char; 1024],
}

#[cfg(target_os = "linux")]
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct dirent {
    pub d_ino: u64,
    pub d_off: i64,
    pub d_reclen: u16,
    pub d_type: u8,
    pub d_name: [c_char; 256],
}

pub const EPERM: c_int = 1;
pub const ENOENT: c_int = 2;
pub const EINTR: c_int = 4;
pub const EACCES: c_int = 13;
pub const EEXIST: c_int = 17;
pub const EISDIR: c_int = 21;
pub const EFBIG: c_int = 27;
pub const EXDEV: c_int = 18;
#[cfg(target_os = "macos")]
pub const ENOSYS: c_int = 78;
#[cfg(target_os = "linux")]
pub const ENOSYS: c_int = 38;
#[cfg(target_os = "macos")]
pub const EOPNOTSUPP: c_int = 102;
#[cfg(target_os = "linux")]
pub const EOPNOTSUPP: c_int = 95;
// `ENOTSUP` is its own number on macOS and the same number as `EOPNOTSUPP` on Linux.
#[cfg(target_os = "macos")]
pub const ENOTSUP: c_int = 45;
#[cfg(target_os = "linux")]
pub const ENOTSUP: c_int = EOPNOTSUPP;
#[cfg(target_os = "macos")]
pub const EILSEQ: c_int = 92;
#[cfg(target_os = "linux")]
pub const EILSEQ: c_int = 84;

pub const F_OK: c_int = 0;

pub const O_RDONLY: c_int = 0;
pub const O_WRONLY: c_int = 1;
pub const O_RDWR: c_int = 2;
#[cfg(target_os = "macos")]
pub const O_APPEND: c_int = 8;
#[cfg(target_os = "linux")]
pub const O_APPEND: c_int = 1024;
#[cfg(target_os = "macos")]
pub const O_CREAT: c_int = 512;
#[cfg(target_os = "linux")]
pub const O_CREAT: c_int = 64;
#[cfg(target_os = "macos")]
pub const O_TRUNC: c_int = 1024;
#[cfg(target_os = "linux")]
pub const O_TRUNC: c_int = 512;
#[cfg(target_os = "macos")]
pub const O_EXCL: c_int = 2048;
#[cfg(target_os = "linux")]
pub const O_EXCL: c_int = 128;
#[cfg(target_os = "macos")]
pub const O_NOFOLLOW: c_int = 0x100;
#[cfg(target_os = "linux")]
pub const O_NOFOLLOW: c_int = 0x8000;

#[cfg(target_os = "macos")]
pub const PATH_MAX: c_int = 1024;
#[cfg(target_os = "linux")]
pub const PATH_MAX: c_int = 4096;

pub const SEEK_SET: c_int = 0;
pub const SEEK_CUR: c_int = 1;

pub const S_IFDIR: mode_t = 0o4_0000;
pub const S_IFREG: mode_t = 0o10_0000;
pub const S_IFMT: mode_t = 0o17_0000;

unsafe extern "C" {
    pub fn rename(oldname: *const c_char, newname: *const c_char) -> c_int;
    pub fn getenv(s: *const c_char) -> *mut c_char;
    #[cfg_attr(
        all(target_os = "macos", not(target_arch = "aarch64")),
        link_name = "fstat$INODE64"
    )]
    pub fn fstat(fildes: c_int, buf: *mut stat) -> c_int;
    pub fn mkdir(path: *const c_char, mode: mode_t) -> c_int;
    #[cfg_attr(
        all(target_os = "macos", not(target_arch = "aarch64")),
        link_name = "stat$INODE64"
    )]
    pub fn stat(path: *const c_char, buf: *mut stat) -> c_int;
    pub fn open(path: *const c_char, oflag: c_int, ...) -> c_int;
    #[cfg_attr(
        all(target_os = "macos", target_arch = "x86_64"),
        link_name = "opendir$INODE64"
    )]
    pub fn opendir(dirname: *const c_char) -> *mut DIR;
    #[cfg_attr(
        all(target_os = "macos", not(target_arch = "aarch64")),
        link_name = "readdir$INODE64"
    )]
    pub fn readdir(dirp: *mut DIR) -> *mut dirent;
    pub fn closedir(dirp: *mut DIR) -> c_int;
    pub fn access(path: *const c_char, amode: c_int) -> c_int;
    #[cfg_attr(
        all(target_os = "macos", target_arch = "x86_64"),
        link_name = "close$NOCANCEL"
    )]
    pub fn close(fd: c_int) -> c_int;
    pub fn getcwd(buf: *mut c_char, size: usize) -> *mut c_char;
    pub fn getpid() -> i32;
    pub fn isatty(fd: c_int) -> c_int;
    pub fn link(src: *const c_char, dst: *const c_char) -> c_int;
    pub fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t;
    pub fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    pub fn rmdir(path: *const c_char) -> c_int;
    pub fn unlink(c: *const c_char) -> c_int;
    pub fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    #[cfg_attr(
        all(target_os = "macos", not(target_arch = "aarch64")),
        link_name = "lstat$INODE64"
    )]
    pub fn lstat(path: *const c_char, buf: *mut stat) -> c_int;
    pub fn readlink(path: *const c_char, buf: *mut c_char, bufsz: usize) -> isize;
    pub fn fsync(fd: c_int) -> c_int;
    pub fn fchmod(fd: c_int, mode: mode_t) -> c_int;
    pub fn geteuid() -> uid_t;
    pub fn getuid() -> uid_t;
    pub fn getegid() -> gid_t;
    pub fn getgid() -> gid_t;
}

// Only the BSD family has `issetugid`. The Linux targets read `AT_SECURE` from the aux vector.
#[cfg(target_os = "macos")]
unsafe extern "C" {
    pub fn issetugid() -> c_int;
}
