//! The process: its id, and how many threads the machine will usefully run.

use crate::libc_lite as libc;

/// This process's id.
pub fn id() -> u32 {
    (unsafe { libc::getpid() }) as u32
}

/// How many threads this machine will usefully run at once.
///
/// `std::thread::available_parallelism` returns a `Result<NonZero<usize>>` because it can fail to
/// ask. This returns 1 in that case, which is what every caller here does with the error anyway,
/// and 1 is the answer that cannot be wrong.
pub fn available_parallelism() -> usize {
    let n = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) };
    if n < 1 {
        1
    } else {
        n as usize
    }
}

/// End the process with `code`.
///
/// `libc::exit`, so atexit handlers and stdio flushing happen. It is the entry point's way of
/// reporting failure: `main` in a `no_std` binary returns `()`, because `Termination` - the trait
/// that turns a `Result` into an exit status - is std's.
pub fn exit(code: i32) -> ! {
    // Safe: `exit` does not return, and the process is being torn down either way.
    unsafe { libc::exit(code) }
}

/// Terminate immediately without unwinding or running Rust destructors.
pub fn abort() -> ! {
    // Safe: abort takes no arguments and never returns.
    unsafe { libc::abort() }
}

/// Real user ID of the calling process.
pub fn getuid() -> u32 {
    unsafe { libc::getuid() }
}

/// Effective user ID of the calling process.
pub fn geteuid() -> u32 {
    unsafe { libc::geteuid() }
}

/// Real group ID of the calling process.
pub fn getgid() -> u32 {
    unsafe { libc::getgid() }
}

/// Effective group ID of the calling process.
pub fn getegid() -> u32 {
    unsafe { libc::getegid() }
}

/// Send signal `sig` to process `pid`.
pub fn kill(pid: i32, sig: i32) -> Result<(), crate::Errno> {
    crate::checked(unsafe { libc::kill(pid, sig) }).map(|_| ())
}

/// Sentinel representing no resource limit.
pub const RLIM_INFINITY: u64 = u64::MAX;

/// Resource kinds supported by [`getrlimit`] and [`setrlimit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resource {
    /// CPU time limit in seconds.
    Cpu,
    /// Maximum number of open file descriptors.
    NoFile,
}

impl Resource {
    fn as_raw(self) -> libc::rlimit_resource_t {
        match self {
            Resource::Cpu => libc::RLIMIT_CPU,
            Resource::NoFile => libc::RLIMIT_NOFILE,
        }
    }
}

/// Soft and hard resource limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rlimit {
    /// Soft limit.
    pub cur: u64,
    /// Hard limit.
    pub max: u64,
}

/// Query the current soft and hard limits for `resource`.
pub fn getrlimit(resource: Resource) -> Result<Rlimit, crate::Errno> {
    let mut raw: libc::rlimit = unsafe { core::mem::zeroed() };
    crate::checked(unsafe { libc::getrlimit(resource.as_raw(), &mut raw) })?;
    let cur = if raw.rlim_cur == libc::RLIM_INFINITY { RLIM_INFINITY } else { raw.rlim_cur as u64 };
    let max = if raw.rlim_max == libc::RLIM_INFINITY { RLIM_INFINITY } else { raw.rlim_max as u64 };
    Ok(Rlimit { cur, max })
}

/// Set the soft and hard limits for `resource`.
pub fn setrlimit(resource: Resource, limit: Rlimit) -> Result<(), crate::Errno> {
    let raw = libc::rlimit {
        rlim_cur: if limit.cur == RLIM_INFINITY {
            libc::RLIM_INFINITY
        } else {
            limit.cur as libc::rlim_t
        },
        rlim_max: if limit.max == RLIM_INFINITY {
            libc::RLIM_INFINITY
        } else {
            limit.max as libc::rlim_t
        },
    };
    crate::checked(unsafe { libc::setrlimit(resource.as_raw(), &raw) }).map(|_| ())
}

/// Process resource usage summary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rusage {
    /// User CPU time in microseconds.
    pub user_micros: u64,
    /// System CPU time in microseconds.
    pub system_micros: u64,
    /// Maximum resident set size in bytes.
    pub max_rss_bytes: u64,
}

/// Replace the current process image with `path`, passing `args` (`argv`) and `envs` (`envp`).
pub fn execve(
    path: &str,
    args: &[&str],
    envs: &[&str],
) -> Result<core::convert::Infallible, crate::Errno> {
    #[cfg(feature = "libc-rs")]
    {
        let mut arg_slices: alloc::vec::Vec<&[u8]> = alloc::vec::Vec::with_capacity(args.len());
        let mut i = 0usize;
        while i < args.len() {
            arg_slices.push(args[i].as_bytes());
            i += 1;
        }
        let mut env_slices: alloc::vec::Vec<&[u8]> = alloc::vec::Vec::with_capacity(envs.len());
        let mut j = 0usize;
        while j < envs.len() {
            env_slices.push(envs[j].as_bytes());
            j += 1;
        }
        ::libc_rs::process::execve_bytes(path.as_bytes(), &arg_slices, &env_slices)
            .map_err(|e| crate::Errno(e.0))
    }
    #[cfg(not(feature = "libc-rs"))]
    {
        if path.is_empty() || path.as_bytes().contains(&0) {
            return Err(crate::Errno(libc::EINVAL));
        }
        let path_c = crate::path::Path::new(path).as_c();
        let mut arg_bufs: alloc::vec::Vec<alloc::vec::Vec<u8>> =
            alloc::vec::Vec::with_capacity(args.len());
        let mut i = 0usize;
        while i < args.len() {
            let b = args[i].as_bytes();
            if b.contains(&0) {
                return Err(crate::Errno(libc::EINVAL));
            }
            let mut v = alloc::vec::Vec::with_capacity(b.len() + 1);
            v.extend_from_slice(b);
            v.push(0);
            arg_bufs.push(v);
            i += 1;
        }
        let mut argv_ptrs: alloc::vec::Vec<*const libc::c_char> =
            alloc::vec::Vec::with_capacity(arg_bufs.len() + 1);
        let mut k = 0usize;
        while k < arg_bufs.len() {
            argv_ptrs.push(arg_bufs[k].as_ptr().cast());
            k += 1;
        }
        argv_ptrs.push(core::ptr::null());

        let mut env_bufs: alloc::vec::Vec<alloc::vec::Vec<u8>> =
            alloc::vec::Vec::with_capacity(envs.len());
        let mut m = 0usize;
        while m < envs.len() {
            let b = envs[m].as_bytes();
            if b.contains(&0) {
                return Err(crate::Errno(libc::EINVAL));
            }
            let mut v = alloc::vec::Vec::with_capacity(b.len() + 1);
            v.extend_from_slice(b);
            v.push(0);
            env_bufs.push(v);
            m += 1;
        }
        let mut envp_ptrs: alloc::vec::Vec<*const libc::c_char> =
            alloc::vec::Vec::with_capacity(env_bufs.len() + 1);
        let mut n = 0usize;
        while n < env_bufs.len() {
            envp_ptrs.push(env_bufs[n].as_ptr().cast());
            n += 1;
        }
        envp_ptrs.push(core::ptr::null());

        unsafe {
            libc::execve(path_c.as_ptr().cast(), argv_ptrs.as_ptr(), envp_ptrs.as_ptr());
        }
        Err(crate::Errno::current())
    }
}

/// Query resource usage for the calling process.
pub fn getrusage_self() -> Result<Rusage, crate::Errno> {
    let mut raw: libc::rusage = unsafe { core::mem::zeroed() };
    crate::checked(unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut raw) })?;
    let user_micros = (raw.ru_utime.tv_sec.max(0) as u64)
        .saturating_mul(1_000_000)
        .saturating_add(raw.ru_utime.tv_usec.max(0) as u64);
    let system_micros = (raw.ru_stime.tv_sec.max(0) as u64)
        .saturating_mul(1_000_000)
        .saturating_add(raw.ru_stime.tv_usec.max(0) as u64);
    #[cfg(all(target_os = "macos", not(feature = "libc-rs")))]
    let max_rss_bytes = raw.ru_maxrss.max(0) as u64;
    #[cfg(not(all(target_os = "macos", not(feature = "libc-rs"))))]
    let max_rss_bytes = (raw.ru_maxrss.max(0) as u64).saturating_mul(1024);
    Ok(Rusage { user_micros, system_micros, max_rss_bytes })
}
