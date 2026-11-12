//! Processes, descriptors' flags, polling and signals: declarations only.
//!
//! Every value, layout and symbol here is copied from the `libc` crate and checked against it by
//! `tests/libc_lite_parity_process.rs`. Nothing in this file is implemented by ekostd.

#[allow(unused_imports)]
use super::*;
use core::ffi::{c_char, c_int, c_long, c_short, c_void};

pub type pid_t = i32;

#[cfg(target_os = "macos")]
pub type nfds_t = core::ffi::c_uint;
#[cfg(target_os = "linux")]
pub type nfds_t = core::ffi::c_ulong;

pub const STDIN_FILENO: c_int = 0;
pub const STDOUT_FILENO: c_int = 1;
pub const STDERR_FILENO: c_int = 2;

pub const EIO: c_int = 5;
pub const FD_CLOEXEC: c_int = 0x1;
pub const F_GETFD: c_int = 1;
pub const F_SETFD: c_int = 2;
pub const F_GETFL: c_int = 3;
pub const F_SETFL: c_int = 4;
pub const LOCK_EX: c_int = 2;
pub const LOCK_NB: c_int = 4;
pub const POLLIN: c_short = 0x1;
pub const SIGHUP: c_int = 1;
pub const SIGINT: c_int = 2;
pub const SIGKILL: c_int = 9;
pub const SIGTERM: c_int = 15;
pub const WNOHANG: c_int = 0x00000001;
pub const LOCK_SH: c_int = 1;
pub const POLLHUP: c_short = 0x10;
pub const RUSAGE_SELF: c_int = 0;
pub const POSIX_SPAWN_SETPGROUP: c_int = 0x02;
#[cfg(target_os = "macos")]
pub const POSIX_SPAWN_CLOEXEC_DEFAULT: c_int = 0x4000;
#[cfg(target_os = "macos")]
pub const PROC_PIDTASKINFO: c_int = 4;

// The record-lock commands and types differ per target: Apple numbers them 8 and 9 and types the
// lock kinds `c_short`; Linux numbers them 6 and 7 and types the lock kinds `c_int`.
#[cfg(target_os = "macos")]
pub const F_SETLK: c_int = 8;
#[cfg(target_os = "macos")]
pub const F_SETLKW: c_int = 9;
#[cfg(target_os = "macos")]
pub const F_RDLCK: c_short = 1;
#[cfg(target_os = "macos")]
pub const F_UNLCK: c_short = 2;
#[cfg(target_os = "macos")]
pub const F_WRLCK: c_short = 3;
#[cfg(target_os = "linux")]
pub const F_SETLK: c_int = 6;
#[cfg(target_os = "linux")]
pub const F_SETLKW: c_int = 7;
#[cfg(target_os = "linux")]
pub const F_RDLCK: c_int = 0;
#[cfg(target_os = "linux")]
pub const F_WRLCK: c_int = 1;
#[cfg(target_os = "linux")]
pub const F_UNLCK: c_int = 2;

// glibc types the resource id of `getrlimit` and `setrlimit` unsigned; macOS and musl type it `c_int`.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
type rlimit_resource_t = core::ffi::c_uint;
#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
type rlimit_resource_t = c_int;

pub const RLIMIT_CPU: rlimit_resource_t = 0;
#[cfg(target_os = "macos")]
pub const RLIMIT_NOFILE: rlimit_resource_t = 8;
#[cfg(target_os = "linux")]
pub const RLIMIT_NOFILE: rlimit_resource_t = 7;

#[cfg(target_os = "macos")]
pub const EAGAIN: c_int = 35;
#[cfg(target_os = "linux")]
pub const EAGAIN: c_int = 11;
pub const EWOULDBLOCK: c_int = EAGAIN;

#[cfg(target_os = "macos")]
pub const F_DUPFD_CLOEXEC: c_int = 67;
#[cfg(target_os = "linux")]
pub const F_DUPFD_CLOEXEC: c_int = 1030;

#[cfg(target_os = "macos")]
pub const O_CLOEXEC: c_int = 0x01000000;
#[cfg(target_os = "linux")]
pub const O_CLOEXEC: c_int = 0x80000;

#[cfg(target_os = "macos")]
pub const O_NONBLOCK: c_int = 0x4;
#[cfg(target_os = "linux")]
pub const O_NONBLOCK: c_int = 2048;

#[cfg(target_os = "macos")]
pub const SA_RESTART: c_int = 0x0002;
#[cfg(target_os = "linux")]
pub const SA_RESTART: c_int = 0x10000000;

#[cfg(target_os = "macos")]
pub const _SC_NPROCESSORS_ONLN: c_int = 58;
#[cfg(target_os = "linux")]
pub const _SC_NPROCESSORS_ONLN: c_int = 84;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct pollfd {
    pub fd: c_int,
    pub events: c_short,
    pub revents: c_short,
}

// The limit type is 64 bits wide on all three targets.
pub type rlim_t = u64;

#[cfg(target_os = "macos")]
pub const RLIM_INFINITY: rlim_t = 0x7fff_ffff_ffff_ffff;
#[cfg(target_os = "linux")]
pub const RLIM_INFINITY: rlim_t = !0;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct rlimit {
    pub rlim_cur: rlim_t,
    pub rlim_max: rlim_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct rusage {
    pub ru_utime: timeval,
    pub ru_stime: timeval,
    pub ru_maxrss: c_long,
    pub ru_ixrss: c_long,
    pub ru_idrss: c_long,
    pub ru_isrss: c_long,
    pub ru_minflt: c_long,
    pub ru_majflt: c_long,
    pub ru_nswap: c_long,
    pub ru_inblock: c_long,
    pub ru_oublock: c_long,
    pub ru_msgsnd: c_long,
    pub ru_msgrcv: c_long,
    pub ru_nsignals: c_long,
    pub ru_nvcsw: c_long,
    pub ru_nivcsw: c_long,
    // musl reserves room after the last counter.
    #[cfg(all(target_os = "linux", target_env = "musl"))]
    __reserved: [c_long; 16],
}

// Apple puts the range first and the owner and kind last; Linux puts the kind first.
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct flock {
    pub l_start: off_t,
    pub l_len: off_t,
    pub l_pid: pid_t,
    pub l_type: c_short,
    pub l_whence: c_short,
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct flock {
    pub l_type: c_short,
    pub l_whence: c_short,
    pub l_start: off_t,
    pub l_len: off_t,
    pub l_pid: pid_t,
}

// Apple hands out both spawn objects as opaque pointers. Linux has plain structs for them.
#[cfg(target_os = "macos")]
pub type posix_spawnattr_t = *mut c_void;
#[cfg(target_os = "macos")]
pub type posix_spawn_file_actions_t = *mut c_void;

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct posix_spawn_file_actions_t {
    __allocated: c_int,
    __used: c_int,
    __actions: *mut c_int,
    __pad: [c_int; 16],
}

// glibc stores a `sched_param` (one `c_int`) where musl stores a `c_int` priority: the same bytes.
#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct posix_spawnattr_t {
    __flags: c_short,
    __pgrp: pid_t,
    __sd: sigset_t,
    __ss: sigset_t,
    __prio: c_int,
    __policy: c_int,
    __pad: [c_int; 16],
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct proc_taskinfo {
    pub pti_virtual_size: u64,
    pub pti_resident_size: u64,
    pub pti_total_user: u64,
    pub pti_total_system: u64,
    pub pti_threads_user: u64,
    pub pti_threads_system: u64,
    pub pti_policy: i32,
    pub pti_faults: i32,
    pub pti_pageins: i32,
    pub pti_cow_faults: i32,
    pub pti_messages_sent: i32,
    pub pti_messages_received: i32,
    pub pti_syscalls_mach: i32,
    pub pti_syscalls_unix: i32,
    pub pti_csw: i32,
    pub pti_threadnum: i32,
    pub pti_numrunning: i32,
    pub pti_priority: i32,
}

// The wait-status helpers are macros in C. The libc crate writes them as bit logic, and the bits
// are the same on macOS (`0o177`) and Linux (`0x7f`).
#[allow(non_snake_case)]
pub const fn WTERMSIG(status: c_int) -> c_int {
    status & 0x7f
}

#[allow(non_snake_case)]
pub const fn WIFEXITED(status: c_int) -> bool {
    (status & 0x7f) == 0
}

#[allow(non_snake_case)]
pub const fn WEXITSTATUS(status: c_int) -> c_int {
    (status >> 8) & 0xff
}

// macOS has `sigset_t = u32`, so `sigaction` and `sigemptyset` use `u32` directly there. Linux
// has a struct, and it is the only type here that is not one of the listed items; `sigaction`
// and `sigemptyset` cannot be written without it.
#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sigset_t {
    __val: [u64; 16],
}

#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sigaction {
    pub sa_sigaction: usize,
    pub sa_mask: u32,
    pub sa_flags: c_int,
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sigaction {
    pub sa_sigaction: usize,
    pub sa_mask: sigset_t,
    pub sa_flags: c_int,
    pub sa_restorer: Option<extern "C" fn()>,
}

unsafe extern "C" {
    pub fn abort() -> !;
    pub fn atexit(cb: extern "C" fn()) -> c_int;
    pub fn exit(status: c_int) -> !;
    pub fn _exit(status: c_int) -> !;
    pub fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    pub fn chdir(dir: *const c_char) -> c_int;
    pub fn dup2(src: c_int, dst: c_int) -> c_int;
    pub fn execve(
        prog: *const c_char,
        argv: *const *const c_char,
        envp: *const *const c_char,
    ) -> c_int;
    pub fn fork() -> pid_t;
    pub fn pipe(fds: *mut c_int) -> c_int;
    pub fn setpgid(pid: pid_t, pgid: pid_t) -> c_int;
    pub fn setsid() -> pid_t;
    pub fn waitpid(pid: pid_t, status: *mut c_int, options: c_int) -> pid_t;
    pub fn kill(pid: pid_t, sig: c_int) -> c_int;
    pub fn setenv(name: *const c_char, val: *const c_char, overwrite: c_int) -> c_int;
    pub fn ftruncate(fd: c_int, length: i64) -> c_int;
    pub fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int;
    pub fn sysconf(name: c_int) -> c_long;
    pub fn flock(fd: c_int, operation: c_int) -> c_int;
    pub fn sigaction(signum: c_int, act: *const sigaction, oldact: *mut sigaction) -> c_int;
    pub fn getrlimit(resource: rlimit_resource_t, rlim: *mut rlimit) -> c_int;
    pub fn setrlimit(resource: rlimit_resource_t, rlim: *const rlimit) -> c_int;
    pub fn getrusage(resource: c_int, usage: *mut rusage) -> c_int;

    pub fn posix_spawn(
        pid: *mut pid_t,
        path: *const c_char,
        file_actions: *const posix_spawn_file_actions_t,
        attrp: *const posix_spawnattr_t,
        argv: *const *mut c_char,
        envp: *const *mut c_char,
    ) -> c_int;
    pub fn posix_spawnp(
        pid: *mut pid_t,
        file: *const c_char,
        file_actions: *const posix_spawn_file_actions_t,
        attrp: *const posix_spawnattr_t,
        argv: *const *mut c_char,
        envp: *const *mut c_char,
    ) -> c_int;
    pub fn posix_spawnattr_init(attr: *mut posix_spawnattr_t) -> c_int;
    pub fn posix_spawnattr_destroy(attr: *mut posix_spawnattr_t) -> c_int;
    pub fn posix_spawnattr_setflags(attr: *mut posix_spawnattr_t, flags: c_short) -> c_int;
    pub fn posix_spawnattr_setpgroup(attr: *mut posix_spawnattr_t, pgroup: pid_t) -> c_int;
    pub fn posix_spawn_file_actions_init(actions: *mut posix_spawn_file_actions_t) -> c_int;
    pub fn posix_spawn_file_actions_destroy(actions: *mut posix_spawn_file_actions_t) -> c_int;
    pub fn posix_spawn_file_actions_addclose(
        actions: *mut posix_spawn_file_actions_t,
        fd: c_int,
    ) -> c_int;
    pub fn posix_spawn_file_actions_adddup2(
        actions: *mut posix_spawn_file_actions_t,
        fd: c_int,
        newfd: c_int,
    ) -> c_int;
}

// macOS has no `pipe2`. The `libc` crate declares it only for the linux-like targets.
#[cfg(target_os = "linux")]
unsafe extern "C" {
    pub fn pipe2(fds: *mut c_int, flags: c_int) -> c_int;
}

// `mode_t` is `u16` on macOS and `u32` on Linux.
#[cfg(target_os = "macos")]
unsafe extern "C" {
    pub fn umask(mask: u16) -> u16;
    pub fn sigemptyset(set: *mut u32) -> c_int;
}

#[cfg(target_os = "linux")]
unsafe extern "C" {
    pub fn umask(mask: u32) -> u32;
    pub fn sigemptyset(set: *mut sigset_t) -> c_int;
}

// Only Apple has `proc_pidinfo`.
#[cfg(target_os = "macos")]
unsafe extern "C" {
    pub fn proc_pidinfo(
        pid: c_int,
        flavor: c_int,
        arg: u64,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;
}

// Only Apple has the Mach task calls. These aliases are the ones the signature and the record need.
#[cfg(target_os = "macos")]
type integer_t = c_int;
#[cfg(target_os = "macos")]
type natural_t = u32;
#[cfg(target_os = "macos")]
type kern_return_t = c_int;
#[cfg(target_os = "macos")]
type policy_t = c_int;
#[cfg(target_os = "macos")]
type mach_msg_type_number_t = natural_t;
#[cfg(target_os = "macos")]
type task_info_t = *mut integer_t;
#[cfg(target_os = "macos")]
pub type task_flavor_t = natural_t;

// Only the record is not on the item list: `mach_task_basic_info` has two fields of this type.
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct time_value_t {
    pub seconds: integer_t,
    pub microseconds: integer_t,
}

// Apple packs this record to 4 bytes, so its 64-bit fields are not 8-byte aligned.
#[cfg(target_os = "macos")]
#[repr(C, packed(4))]
#[derive(Clone, Copy)]
pub struct mach_task_basic_info {
    pub virtual_size: mach_vm_size_t,
    pub resident_size: mach_vm_size_t,
    pub resident_size_max: mach_vm_size_t,
    pub user_time: time_value_t,
    pub system_time: time_value_t,
    pub policy: policy_t,
    pub suspend_count: integer_t,
}

#[cfg(target_os = "macos")]
pub const KERN_SUCCESS: c_int = 0;
#[cfg(target_os = "macos")]
pub const MACH_TASK_BASIC_INFO: u32 = 20;
// The count is the record size in `natural_t` words, as in the `libc` crate.
#[cfg(target_os = "macos")]
pub const MACH_TASK_BASIC_INFO_COUNT: u32 =
    (core::mem::size_of::<mach_task_basic_info>() / core::mem::size_of::<natural_t>()) as u32;

#[cfg(target_os = "macos")]
unsafe extern "C" {
    pub static mut mach_task_self_: mach_port_t;
    pub fn task_info(
        host: mach_port_t,
        flavor: task_flavor_t,
        task_info_out: task_info_t,
        task_info_count: *mut mach_msg_type_number_t,
    ) -> kern_return_t;
}
