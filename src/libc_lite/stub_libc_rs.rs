#![allow(non_camel_case_types, non_snake_case, dead_code, clippy::missing_safety_doc)]

pub use core::ffi::{c_char, c_int, c_long, c_short, c_ulong, c_void};

pub type off_t = i64;
pub type uid_t = u32;
pub type gid_t = u32;
pub type mode_t = u32;
pub type pid_t = i32;
pub type nfds_t = usize;
pub type socklen_t = u32;
pub type size_t = usize;
pub type time_t = i64;
pub type rlim_t = u64;

pub type stat = ::libc_rs::fs::Stat;
pub type timespec = ::libc_rs::time::Timespec;
pub type timeval = ::libc_rs::time::Timeval;
pub type pollfd = ::libc_rs::poll::PollFd;
pub type rlimit = ::libc_rs::process::Rlimit;
pub type rusage = ::libc_rs::process::Rusage;
pub type flock = ::libc_rs::io::Flock;
pub type sigset_t = ::libc_rs::process::KernelSigSet;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sockaddr {
    pub sa_family: u16,
    pub sa_data: [c_char; 14],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sockaddr_un {
    pub sun_family: u16,
    pub sun_path: [c_char; 108],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sigaction {
    pub sa_sigaction: usize,
    pub sa_mask: sigset_t,
    pub sa_flags: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Dl_info {
    pub dli_fname: *const c_char,
    pub dli_fbase: *mut c_void,
    pub dli_sname: *const c_char,
    pub dli_saddr: *mut c_void,
}

pub use ::libc_rs::errno::{
    E2BIG, EACCES, EADDRINUSE, EAGAIN, EBADF, EBUSY, ECHILD, ECONNREFUSED, ECONNRESET, EDEADLK,
    EDOM, EEXIST, EFAULT, EFBIG, EILSEQ, EINTR, EINVAL, EIO, EISDIR, ELOOP, EMFILE, EMLINK,
    EMSGSIZE, ENAMETOOLONG, ENFILE, ENODEV, ENOENT, ENOEXEC, ENOLCK, ENOMEM, ENOSPC, ENOSYS,
    ENOTCONN, ENOTDIR, ENOTEMPTY, ENOTSOCK, ENOTSUP, ENOTTY, ENXIO, EOPNOTSUPP, EOVERFLOW, EPERM,
    EPIPE, EPROTO, ERANGE, EROFS, ESPIPE, ESRCH, ETIMEDOUT, ETXTBSY, EWOULDBLOCK, EXDEV,
};
pub use ::libc_rs::fd::{AT_FDCWD, AT_REMOVEDIR, AT_SYMLINK_NOFOLLOW};
pub use ::libc_rs::fs::{
    F_OK, LOCK_EX, LOCK_NB, LOCK_SH, LOCK_UN, O_APPEND, O_CLOEXEC, O_CREAT, O_DIRECTORY, O_EXCL,
    O_NOFOLLOW, O_NONBLOCK, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY, SEEK_CUR, SEEK_END, SEEK_SET,
    S_IFDIR, S_IFLNK, S_IFMT, S_IFREG,
};
pub const PATH_MAX: c_int = ::libc_rs::fs::PATH_MAX as c_int;
pub use ::libc_rs::io::{
    FD_CLOEXEC, F_DUPFD, F_DUPFD_CLOEXEC, F_GETFD, F_GETFL, F_RDLCK, F_SETFD, F_SETFL, F_SETLK,
    F_SETLKW, F_UNLCK, F_WRLCK, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO,
};
pub use ::libc_rs::mm::{
    MADV_DONTNEED, MADV_FREE, MAP_ANON, MAP_FIXED, MAP_NORESERVE, MAP_PRIVATE, MAP_SHARED,
    MS_INVALIDATE, PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE,
};
pub const MAP_FAILED: *mut c_void = !0 as *mut c_void;
pub use ::libc_rs::net::{
    AF_UNIX, MSG_NOSIGNAL, SOCK_STREAM, SOL_SOCKET, SO_RCVTIMEO, SO_SNDTIMEO,
};
pub use ::libc_rs::poll::{POLLERR, POLLHUP, POLLIN, POLLNVAL, POLLOUT};
pub use ::libc_rs::process::{
    RLIM_INFINITY, RUSAGE_SELF, SIGHUP, SIGINT, SIGKILL, SIGPIPE, SIGTERM, SIG_DFL, SIG_IGN,
    WNOHANG,
};
pub type rlimit_resource_t = c_int;
pub const RLIMIT_CPU: rlimit_resource_t = ::libc_rs::process::RLIMIT_CPU as c_int;
pub const RLIMIT_NOFILE: rlimit_resource_t = ::libc_rs::process::RLIMIT_NOFILE as c_int;
pub const SA_RESTART: c_int = ::libc_rs::process::SA_RESTART as c_int;
pub use ::libc_rs::time::{CLOCK_MONOTONIC, CLOCK_REALTIME};

pub const _SC_PAGESIZE: c_int = 30;
pub const _SC_NPROCESSORS_ONLN: c_int = 84;
pub const AT_SECURE: c_ulong = 23;
pub const RTLD_NOW: c_int = 2;
pub const RTLD_LOCAL: c_int = 0;

pub const fn WTERMSIG(status: c_int) -> c_int {
    ::libc_rs::process::wtermsig(status)
}

pub const fn WIFEXITED(status: c_int) -> bool {
    ::libc_rs::process::wifexited(status)
}

pub const fn WEXITSTATUS(status: c_int) -> c_int {
    ::libc_rs::process::wexitstatus(status)
}

#[inline]
pub unsafe fn __errno_location() -> *mut c_int {
    ::libc_rs::thread::errno_cell()
}

#[inline]
fn set_errno(e: ::libc_rs::errno::Errno) {
    unsafe { *::libc_rs::thread::errno_cell() = e.0 };
}

#[inline]
fn ret_unit(r: ::libc_rs::errno::SysResult<()>) -> c_int {
    match r {
        Ok(()) => 0,
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

#[inline]
fn ret_fd(r: ::libc_rs::errno::SysResult<i32>) -> c_int {
    match r {
        Ok(v) => v,
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

#[inline]
fn ret_isize(r: ::libc_rs::errno::SysResult<usize>) -> isize {
    match r {
        Ok(v) => v as isize,
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

pub unsafe fn strerror_r(errnum: c_int, buf: *mut c_char, buflen: usize) -> c_int {
    if buf.is_null() || buflen == 0 {
        return EINVAL;
    }
    let msg = ::libc_rs::errno::Errno(errnum).message().as_bytes();
    let copy_len = msg.len().min(buflen - 1);
    unsafe {
        core::ptr::copy_nonoverlapping(msg.as_ptr(), buf.cast::<u8>(), copy_len);
        *buf.cast::<u8>().add(copy_len) = 0;
    }
    0
}

pub unsafe fn open2(path: *const c_char, oflag: c_int) -> c_int {
    unsafe { open3(path, oflag, 0) }
}

pub unsafe fn open3(path: *const c_char, oflag: c_int, mode: mode_t) -> c_int {
    let s = match unsafe { ::libc_rs::fs::cstr_slice(path.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    ret_fd(::libc_rs::fs::open(s, oflag, mode))
}

pub unsafe fn fcntl2(fd: c_int, cmd: c_int) -> c_int {
    ret_fd(::libc_rs::io::fcntl(fd, cmd, 0))
}

pub unsafe fn fcntl3(fd: c_int, cmd: c_int, arg: c_int) -> c_int {
    ret_fd(::libc_rs::io::fcntl(fd, cmd, arg as usize))
}

pub unsafe fn fcntl_flock(fd: c_int, cmd: c_int, lock: *const flock) -> c_int {
    ret_fd(::libc_rs::io::fcntl(fd, cmd, lock as usize))
}

pub unsafe fn close(fd: c_int) -> c_int {
    ret_unit(::libc_rs::io::close(fd))
}

pub unsafe fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize {
    ret_isize(unsafe { ::libc_rs::io::read_raw(fd, buf.cast(), count) })
}

pub unsafe fn write(fd: c_int, buf: *const c_void, count: usize) -> isize {
    ret_isize(unsafe { ::libc_rs::io::write_raw(fd, buf.cast(), count) })
}

pub unsafe fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t {
    match ::libc_rs::fs::lseek(fd, offset, whence) {
        Ok(off) => off,
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

pub unsafe fn fsync(fd: c_int) -> c_int {
    ret_unit(::libc_rs::fs::fsync(fd))
}

pub unsafe fn fchmod(fd: c_int, mode: mode_t) -> c_int {
    ret_unit(::libc_rs::fs::fchmod(fd, mode))
}

pub unsafe fn ftruncate(fd: c_int, length: i64) -> c_int {
    ret_unit(::libc_rs::fs::ftruncate(fd, length))
}

pub unsafe fn stat(path: *const c_char, buf: *mut stat) -> c_int {
    if buf.is_null() {
        set_errno(::libc_rs::errno::Errno(EFAULT));
        return -1;
    }
    let s = match unsafe { ::libc_rs::fs::cstr_slice(path.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    match ::libc_rs::fs::stat(s) {
        Ok(st) => {
            unsafe { *buf = st };
            0
        }
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

pub unsafe fn lstat(path: *const c_char, buf: *mut stat) -> c_int {
    if buf.is_null() {
        set_errno(::libc_rs::errno::Errno(EFAULT));
        return -1;
    }
    let s = match unsafe { ::libc_rs::fs::cstr_slice(path.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    match ::libc_rs::fs::lstat(s) {
        Ok(st) => {
            unsafe { *buf = st };
            0
        }
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

pub unsafe fn fstat(fd: c_int, buf: *mut stat) -> c_int {
    if buf.is_null() {
        set_errno(::libc_rs::errno::Errno(EFAULT));
        return -1;
    }
    match ::libc_rs::fs::fstat(fd) {
        Ok(st) => {
            unsafe { *buf = st };
            0
        }
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

pub unsafe fn mkdir(path: *const c_char, mode: mode_t) -> c_int {
    let s = match unsafe { ::libc_rs::fs::cstr_slice(path.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    ret_unit(::libc_rs::fs::mkdir(s, mode))
}

pub unsafe fn rmdir(path: *const c_char) -> c_int {
    let s = match unsafe { ::libc_rs::fs::cstr_slice(path.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    ret_unit(::libc_rs::fs::rmdir(s))
}

pub unsafe fn unlink(path: *const c_char) -> c_int {
    let s = match unsafe { ::libc_rs::fs::cstr_slice(path.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    ret_unit(::libc_rs::fs::unlink(s))
}

pub unsafe fn rename(oldname: *const c_char, newname: *const c_char) -> c_int {
    let o = match unsafe { ::libc_rs::fs::cstr_slice(oldname.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    let n = match unsafe { ::libc_rs::fs::cstr_slice(newname.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    ret_unit(::libc_rs::fs::rename(o, n))
}

pub unsafe fn link(src: *const c_char, dst: *const c_char) -> c_int {
    let o = match unsafe { ::libc_rs::fs::cstr_slice(src.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    let n = match unsafe { ::libc_rs::fs::cstr_slice(dst.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    ret_unit(::libc_rs::fs::link(o, n))
}

pub unsafe fn readlink(path: *const c_char, buf: *mut c_char, bufsz: usize) -> isize {
    if buf.is_null() {
        set_errno(::libc_rs::errno::Errno(EFAULT));
        return -1;
    }
    let s = match unsafe { ::libc_rs::fs::cstr_slice(path.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    let out = unsafe { core::slice::from_raw_parts_mut(buf.cast::<u8>(), bufsz) };
    match ::libc_rs::fs::readlink(s, out) {
        Ok(filled) => filled.len() as isize,
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

pub unsafe fn access(path: *const c_char, amode: c_int) -> c_int {
    let s = match unsafe { ::libc_rs::fs::cstr_slice(path.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    ret_unit(::libc_rs::fs::access(s, amode))
}

pub unsafe fn getcwd(buf: *mut c_char, size: usize) -> *mut c_char {
    if buf.is_null() || size == 0 {
        set_errno(::libc_rs::errno::Errno(EINVAL));
        return core::ptr::null_mut();
    }
    let out = unsafe { core::slice::from_raw_parts_mut(buf.cast::<u8>(), size) };
    match ::libc_rs::fs::getcwd(out) {
        Ok(_) => buf,
        Err(e) => {
            set_errno(e);
            core::ptr::null_mut()
        }
    }
}

pub unsafe fn chdir(dir: *const c_char) -> c_int {
    let s = match unsafe { ::libc_rs::fs::cstr_slice(dir.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    ret_unit(::libc_rs::fs::chdir(s))
}

pub unsafe fn isatty(fd: c_int) -> c_int {
    if ::libc_rs::io::isatty(fd) {
        1
    } else {
        set_errno(::libc_rs::errno::Errno(ENOTTY));
        0
    }
}

pub unsafe fn flock(fd: c_int, operation: c_int) -> c_int {
    ret_unit(::libc_rs::fs::flock(fd, operation))
}

pub unsafe fn dup(fd: c_int) -> c_int {
    ret_fd(::libc_rs::io::dup(fd))
}

pub unsafe fn dup2(src: c_int, dst: c_int) -> c_int {
    ret_fd(::libc_rs::io::dup2(src, dst))
}

pub unsafe fn pipe(fds: *mut c_int) -> c_int {
    unsafe { pipe2(fds, 0) }
}

pub unsafe fn pipe2(fds: *mut c_int, flags: c_int) -> c_int {
    if fds.is_null() {
        set_errno(::libc_rs::errno::Errno(EFAULT));
        return -1;
    }
    match ::libc_rs::io::pipe2(flags) {
        Ok([r, w]) => {
            unsafe {
                *fds = r;
                *fds.add(1) = w;
            }
            0
        }
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

pub unsafe fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int {
    let slice = if nfds == 0 || fds.is_null() {
        &mut [][..]
    } else {
        unsafe { core::slice::from_raw_parts_mut(fds, nfds) }
    };
    match ::libc_rs::poll::poll(slice, timeout) {
        Ok(n) => n as c_int,
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

pub unsafe fn mmap(
    addr: *mut c_void,
    len: usize,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    offset: i64,
) -> *mut c_void {
    match unsafe { ::libc_rs::mm::mmap(addr.cast(), len, prot, flags, fd, offset) } {
        Ok(p) => p.cast(),
        Err(e) => {
            set_errno(e);
            MAP_FAILED
        }
    }
}

pub unsafe fn munmap(addr: *mut c_void, len: usize) -> c_int {
    ret_unit(unsafe { ::libc_rs::mm::munmap(addr.cast(), len) })
}

pub unsafe fn msync(addr: *mut c_void, len: usize, flags: c_int) -> c_int {
    ret_unit(unsafe { ::libc_rs::mm::msync(addr.cast(), len, flags) })
}

pub unsafe fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int {
    ret_unit(unsafe { ::libc_rs::mm::mprotect(addr.cast(), len, prot) })
}

pub unsafe fn madvise(addr: *mut c_void, len: usize, advice: c_int) -> c_int {
    ret_unit(unsafe { ::libc_rs::mm::madvise(addr.cast(), len, advice) })
}

pub unsafe fn mincore(addr: *mut c_void, len: usize, vec: *mut u8) -> c_int {
    ret_unit(unsafe { ::libc_rs::mm::mincore(addr.cast(), len, vec) })
}

pub unsafe fn malloc(size: usize) -> *mut c_void {
    unsafe { ::libc_rs::heap::c_malloc(size).cast() }
}

pub unsafe fn calloc(nobj: usize, size: usize) -> *mut c_void {
    unsafe { ::libc_rs::heap::c_calloc(nobj, size).cast() }
}

pub unsafe fn realloc(p: *mut c_void, size: usize) -> *mut c_void {
    unsafe { ::libc_rs::heap::c_realloc(p.cast(), size).cast() }
}

pub unsafe fn free(p: *mut c_void) {
    unsafe { ::libc_rs::heap::c_free(p.cast()) }
}

pub unsafe fn posix_memalign(memptr: *mut *mut c_void, align: usize, size: usize) -> c_int {
    unsafe { ::libc_rs::heap::c_posix_memalign(memptr.cast(), align, size) }
}

pub unsafe fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int {
    ret_fd(::libc_rs::net::socket(domain, ty, protocol))
}

pub unsafe fn socketpair(
    domain: c_int,
    ty: c_int,
    protocol: c_int,
    socket_vector: *mut c_int,
) -> c_int {
    if socket_vector.is_null() {
        set_errno(::libc_rs::errno::Errno(EFAULT));
        return -1;
    }
    match ::libc_rs::net::socketpair(domain, ty, protocol) {
        Ok([a, b]) => {
            unsafe {
                *socket_vector = a;
                *socket_vector.add(1) = b;
            }
            0
        }
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

pub unsafe fn bind(socket: c_int, address: *const sockaddr, address_len: socklen_t) -> c_int {
    ret_unit(unsafe { ::libc_rs::net::bind_raw(socket, address.cast(), address_len) })
}

pub unsafe fn connect(socket: c_int, address: *const sockaddr, len: socklen_t) -> c_int {
    ret_unit(unsafe { ::libc_rs::net::connect_raw(socket, address.cast(), len) })
}

pub unsafe fn listen(socket: c_int, backlog: c_int) -> c_int {
    ret_unit(::libc_rs::net::listen(socket, backlog))
}

pub unsafe fn accept(socket: c_int, address: *mut sockaddr, address_len: *mut socklen_t) -> c_int {
    ret_fd(unsafe { ::libc_rs::net::accept4_raw(socket, address.cast(), address_len, 0) })
}

pub unsafe fn setsockopt(
    socket: c_int,
    level: c_int,
    name: c_int,
    value: *const c_void,
    option_len: socklen_t,
) -> c_int {
    ret_unit(unsafe {
        ::libc_rs::net::setsockopt_raw(socket, level, name, value.cast(), option_len)
    })
}

pub unsafe fn send(socket: c_int, buf: *const c_void, len: usize, flags: c_int) -> isize {
    ret_isize(unsafe { ::libc_rs::net::send_raw(socket, buf.cast(), len, flags) })
}

pub unsafe fn recv(socket: c_int, buf: *mut c_void, len: usize, flags: c_int) -> isize {
    ret_isize(unsafe { ::libc_rs::net::recv_raw(socket, buf.cast(), len, flags) })
}

pub unsafe fn clock_gettime(clk_id: c_int, tp: *mut timespec) -> c_int {
    if tp.is_null() {
        set_errno(::libc_rs::errno::Errno(EFAULT));
        return -1;
    }
    match ::libc_rs::time::clock_gettime(clk_id) {
        Ok(ts) => {
            unsafe { *tp = ts };
            0
        }
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

pub unsafe fn nanosleep(rqtp: *const timespec, rmtp: *mut timespec) -> c_int {
    if rqtp.is_null() {
        set_errno(::libc_rs::errno::Errno(EFAULT));
        return -1;
    }
    let rem = if rmtp.is_null() { None } else { Some(unsafe { &mut *rmtp }) };
    ret_unit(::libc_rs::time::nanosleep(unsafe { &*rqtp }, rem))
}

pub unsafe fn sched_yield() -> c_int {
    ::libc_rs::process::sched_yield();
    0
}

pub unsafe fn getpid() -> i32 {
    ::libc_rs::process::getpid()
}

pub unsafe fn getuid() -> uid_t {
    ::libc_rs::process::getuid()
}

pub unsafe fn geteuid() -> uid_t {
    ::libc_rs::process::geteuid()
}

pub unsafe fn getgid() -> gid_t {
    ::libc_rs::process::getgid()
}

pub unsafe fn getegid() -> gid_t {
    ::libc_rs::process::getegid()
}

pub unsafe fn setpgid(pid: pid_t, pgid: pid_t) -> c_int {
    ret_unit(::libc_rs::process::setpgid(pid, pgid))
}

pub unsafe fn setsid() -> pid_t {
    ret_fd(::libc_rs::process::setsid())
}

pub unsafe fn umask(mask: mode_t) -> mode_t {
    ::libc_rs::process::umask(mask)
}

pub unsafe fn getrlimit(resource: c_int, rlim: *mut rlimit) -> c_int {
    if rlim.is_null() {
        set_errno(::libc_rs::errno::Errno(EFAULT));
        return -1;
    }
    match ::libc_rs::process::getrlimit(resource as u32) {
        Ok(r) => {
            unsafe { *rlim = r };
            0
        }
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

pub unsafe fn setrlimit(resource: c_int, rlim: *const rlimit) -> c_int {
    if rlim.is_null() {
        set_errno(::libc_rs::errno::Errno(EFAULT));
        return -1;
    }
    ret_unit(::libc_rs::process::setrlimit(resource as u32, unsafe { &*rlim }))
}

pub unsafe fn getrusage(resource: c_int, usage: *mut rusage) -> c_int {
    if usage.is_null() {
        set_errno(::libc_rs::errno::Errno(EFAULT));
        return -1;
    }
    match ::libc_rs::process::getrusage(resource) {
        Ok(u) => {
            unsafe { *usage = u };
            0
        }
        Err(e) => {
            set_errno(e);
            -1
        }
    }
}

pub unsafe fn sigemptyset(set: *mut sigset_t) -> c_int {
    if set.is_null() {
        set_errno(::libc_rs::errno::Errno(EFAULT));
        return -1;
    }
    unsafe { (*set).bits = [0; 1] };
    0
}

pub unsafe fn sigaction(signum: c_int, act: *const sigaction, oldact: *mut sigaction) -> c_int {
    let k_act_val = if act.is_null() {
        None
    } else {
        let a = unsafe { &*act };
        Some(::libc_rs::process::KernelSigAction {
            sa_handler: a.sa_sigaction,
            sa_flags: a.sa_flags as u32 as usize,
            sa_restorer: 0,
            sa_mask: a.sa_mask,
        })
    };
    let mut k_old = ::libc_rs::process::KernelSigAction::default();
    let res = unsafe {
        ::libc_rs::process::sigaction(
            signum,
            k_act_val.as_ref(),
            if oldact.is_null() { None } else { Some(&mut k_old) },
        )
    };
    if res.is_ok() && !oldact.is_null() {
        unsafe {
            *oldact = sigaction {
                sa_sigaction: k_old.sa_handler,
                sa_mask: k_old.sa_mask,
                sa_flags: k_old.sa_flags as c_int,
            };
        }
    }
    ret_unit(res)
}

pub unsafe fn kill(pid: pid_t, sig: c_int) -> c_int {
    ret_unit(::libc_rs::process::kill(pid, sig))
}

pub unsafe fn fork() -> pid_t {
    ret_fd(::libc_rs::process::fork())
}

pub unsafe fn execve(
    prog: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    ret_unit(unsafe { ::libc_rs::process::execve_raw(prog.cast(), argv.cast(), envp.cast()) })
}

pub unsafe fn waitpid(pid: pid_t, status: *mut c_int, options: c_int) -> pid_t {
    let st = if status.is_null() { None } else { Some(unsafe { &mut *status }) };
    ret_fd(::libc_rs::process::waitpid_opt(pid, st, options))
}

pub unsafe fn exit(status: c_int) -> ! {
    ::libc_rs::process::exit_group(status)
}

pub unsafe fn _exit(status: c_int) -> ! {
    ::libc_rs::process::exit_group(status)
}

pub unsafe fn abort() -> ! {
    ::libc_rs::process::abort()
}

pub unsafe fn sysconf(name: c_int) -> c_long {
    match name {
        _SC_PAGESIZE => ::libc_rs::startup::page_size() as c_long,
        _SC_NPROCESSORS_ONLN => 1,
        _ => {
            set_errno(::libc_rs::errno::Errno(EINVAL));
            -1
        }
    }
}

pub unsafe fn getauxval(type_: c_ulong) -> c_ulong {
    ::libc_rs::startup::getauxval(type_ as u64) as c_ulong
}

pub unsafe fn dladdr(_addr: *const c_void, _info: *mut Dl_info) -> c_int {
    0
}

pub const POSIX_SPAWN_SETPGROUP: c_int = 0x02;

#[derive(Clone, Copy)]
enum SpawnFileAction {
    Close(c_int),
    Dup2(c_int, c_int),
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct posix_spawn_file_actions_t {
    inner: *mut alloc::vec::Vec<SpawnFileAction>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct posix_spawnattr_t {
    flags: c_short,
    pgroup: pid_t,
}

pub unsafe fn posix_spawn_file_actions_init(
    file_actions: *mut posix_spawn_file_actions_t,
) -> c_int {
    if file_actions.is_null() {
        return EINVAL;
    }
    let actions = alloc::boxed::Box::new(alloc::vec::Vec::new());
    unsafe { (*file_actions).inner = alloc::boxed::Box::into_raw(actions) };
    0
}

pub unsafe fn posix_spawn_file_actions_destroy(
    file_actions: *mut posix_spawn_file_actions_t,
) -> c_int {
    if file_actions.is_null() {
        return EINVAL;
    }
    let ptr = unsafe { (*file_actions).inner };
    if !ptr.is_null() {
        unsafe {
            drop(alloc::boxed::Box::from_raw(ptr));
            (*file_actions).inner = core::ptr::null_mut();
        }
    }
    0
}

pub unsafe fn posix_spawn_file_actions_addclose(
    file_actions: *mut posix_spawn_file_actions_t,
    fd: c_int,
) -> c_int {
    if file_actions.is_null() || unsafe { (*file_actions).inner.is_null() } {
        return EINVAL;
    }
    if fd < 0 {
        return EBADF;
    }
    unsafe { (*(*file_actions).inner).push(SpawnFileAction::Close(fd)) };
    0
}

pub unsafe fn posix_spawn_file_actions_adddup2(
    file_actions: *mut posix_spawn_file_actions_t,
    fd: c_int,
    newfd: c_int,
) -> c_int {
    if file_actions.is_null() || unsafe { (*file_actions).inner.is_null() } {
        return EINVAL;
    }
    if fd < 0 || newfd < 0 {
        return EBADF;
    }
    unsafe { (*(*file_actions).inner).push(SpawnFileAction::Dup2(fd, newfd)) };
    0
}

pub unsafe fn posix_spawnattr_init(attr: *mut posix_spawnattr_t) -> c_int {
    if attr.is_null() {
        return EINVAL;
    }
    unsafe {
        (*attr).flags = 0;
        (*attr).pgroup = 0;
    }
    0
}

pub unsafe fn posix_spawnattr_destroy(_attr: *mut posix_spawnattr_t) -> c_int {
    0
}

pub unsafe fn posix_spawnattr_setflags(attr: *mut posix_spawnattr_t, flags: c_short) -> c_int {
    if attr.is_null() {
        return EINVAL;
    }
    unsafe { (*attr).flags = flags };
    0
}

pub unsafe fn posix_spawnattr_setpgroup(attr: *mut posix_spawnattr_t, pgroup: pid_t) -> c_int {
    if attr.is_null() {
        return EINVAL;
    }
    unsafe { (*attr).pgroup = pgroup };
    0
}

unsafe fn apply_spawn_actions(actions: &[SpawnFileAction], flags: c_short, pgroup: pid_t) {
    if (flags & POSIX_SPAWN_SETPGROUP as c_short) != 0 {
        let _ = ::libc_rs::process::setpgid(0, pgroup);
    }
    for &action in actions {
        match action {
            SpawnFileAction::Close(fd) => {
                if ::libc_rs::io::close(fd).is_err() {
                    unsafe { _exit(127) };
                }
            }
            SpawnFileAction::Dup2(fd, newfd) => {
                if ::libc_rs::io::dup2(fd, newfd).is_err() {
                    unsafe { _exit(127) };
                }
            }
        }
    }
}

pub unsafe fn posix_spawn(
    pid: *mut pid_t,
    path: *const c_char,
    file_actions: *const posix_spawn_file_actions_t,
    attrp: *const posix_spawnattr_t,
    argv: *const *mut c_char,
    envp: *const *mut c_char,
) -> c_int {
    if pid.is_null() || path.is_null() || argv.is_null() {
        return EINVAL;
    }
    let actions: &[SpawnFileAction] =
        if file_actions.is_null() || unsafe { (*file_actions).inner.is_null() } {
            &[]
        } else {
            unsafe { (*(*file_actions).inner).as_slice() }
        };
    let (flags, pgroup) =
        if attrp.is_null() { (0, 0) } else { unsafe { ((*attrp).flags, (*attrp).pgroup) } };
    match ::libc_rs::process::fork() {
        Err(e) => e.0,
        Ok(0) => {
            unsafe { apply_spawn_actions(actions, flags, pgroup) };
            let _ =
                unsafe { ::libc_rs::process::execve_raw(path.cast(), argv.cast(), envp.cast()) };
            unsafe { _exit(127) }
        }
        Ok(child_pid) => {
            unsafe { *pid = child_pid };
            0
        }
    }
}

pub unsafe fn posix_spawnp(
    pid: *mut pid_t,
    file: *const c_char,
    file_actions: *const posix_spawn_file_actions_t,
    attrp: *const posix_spawnattr_t,
    argv: *const *mut c_char,
    envp: *const *mut c_char,
) -> c_int {
    if pid.is_null() || file.is_null() || argv.is_null() {
        return EINVAL;
    }
    let file_bytes = match unsafe { ::libc_rs::fs::cstr_slice(file.cast()) } {
        Ok(s) => s,
        Err(e) => return e.0,
    };
    if file_bytes.contains(&b'/') {
        return unsafe { posix_spawn(pid, file, file_actions, attrp, argv, envp) };
    }
    let actions: &[SpawnFileAction] =
        if file_actions.is_null() || unsafe { (*file_actions).inner.is_null() } {
            &[]
        } else {
            unsafe { (*(*file_actions).inner).as_slice() }
        };
    let (flags, pgroup) =
        if attrp.is_null() { (0, 0) } else { unsafe { ((*attrp).flags, (*attrp).pgroup) } };
    let mut candidates: alloc::vec::Vec<alloc::vec::Vec<u8>> = alloc::vec::Vec::new();
    if let Some(path_val) = ::libc_rs::startup::getenv_vec(b"PATH") {
        for dir in path_val.split(|&b| b == b':') {
            let mut cand = alloc::vec::Vec::with_capacity(dir.len() + 1 + file_bytes.len() + 1);
            if !dir.is_empty() {
                cand.extend_from_slice(dir);
                cand.push(b'/');
            }
            cand.extend_from_slice(file_bytes);
            cand.push(0);
            candidates.push(cand);
        }
    }
    match ::libc_rs::process::fork() {
        Err(e) => e.0,
        Ok(0) => {
            unsafe { apply_spawn_actions(actions, flags, pgroup) };
            for cand in &candidates {
                let _ = unsafe {
                    ::libc_rs::process::execve_raw(cand.as_ptr(), argv.cast(), envp.cast())
                };
            }
            unsafe { _exit(127) }
        }
        Ok(child_pid) => {
            unsafe { *pid = child_pid };
            0
        }
    }
}

pub unsafe fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int {
    let n = match unsafe { ::libc_rs::fs::cstr_slice(name.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    let v = match unsafe { ::libc_rs::fs::cstr_slice(value.cast()) } {
        Ok(s) => s,
        Err(e) => {
            set_errno(e);
            return -1;
        }
    };
    ret_unit(::libc_rs::startup::setenv_bytes(n, v, overwrite != 0))
}

pub unsafe fn atexit(_func: extern "C" fn()) -> c_int {
    0
}

pub unsafe fn dlopen(_filename: *const c_char, _flag: c_int) -> *mut c_void {
    core::ptr::null_mut()
}

pub unsafe fn dlsym(_handle: *mut c_void, _symbol: *const c_char) -> *mut c_void {
    core::ptr::null_mut()
}

pub unsafe fn dlclose(_handle: *mut c_void) -> c_int {
    0
}
