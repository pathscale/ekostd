//! Environment variables and the working directory.
//!
//! Paths and environment values are bytes on this platform, not text. `std` models them as
//! `OsString` to stay honest about that on Windows too; here they are `Vec<u8>`, and the
//! `String` conversions are offered separately so a caller that genuinely needs text has to say
//! so and handle the failure.

use alloc::string::String;
use alloc::vec::Vec;

/// The raw bytes of an environment variable, or `None` if it is unset.
///
/// This is `std::env::var_os`. The name is the same because the semantics are: unset and
/// "set to nothing" are different, and callers deciding on a `-Z` flag rely on that.
pub fn var_os(name: &str) -> Option<Vec<u8>> {
    let mut c = Vec::with_capacity(name.len() + 1);
    c.extend_from_slice(name.as_bytes());
    c.push(0);
    let p = unsafe { libc::getenv(c.as_ptr().cast()) };
    if p.is_null() {
        return None;
    }
    let mut out = Vec::new();
    let mut i = 0isize;
    loop {
        let b = unsafe { *p.offset(i) } as u8;
        if b == 0 {
            break;
        }
        out.push(b);
        i += 1;
    }
    Some(out)
}

/// The value of an environment variable as UTF-8.
///
/// `None` covers both "unset" and "set to bytes that are not UTF-8". `std::env::var`
/// distinguishes those with `VarError`; no caller here does anything different for the second
/// case, so the distinction is dropped rather than carried.
pub fn var(name: &str) -> Option<String> {
    let bytes = var_os(name)?;
    String::from_utf8(bytes).ok()
}

/// Set a variable. Unsafe for the same reason `std::env::set_var` is: it races other threads.
///
/// # Safety
/// No other thread may be reading the environment concurrently.
pub unsafe fn set_var(name: &str, value: &str) {
    let mut n = Vec::with_capacity(name.len() + 1);
    n.extend_from_slice(name.as_bytes());
    n.push(0);
    let mut v = Vec::with_capacity(value.len() + 1);
    v.extend_from_slice(value.as_bytes());
    v.push(0);
    unsafe { libc::setenv(n.as_ptr().cast(), v.as_ptr().cast(), 1) };
}

/// The process working directory, as bytes.
pub fn current_dir() -> Option<Vec<u8>> {
    let mut buf = alloc::vec![0u8; 4096];
    let p = unsafe { libc::getcwd(buf.as_mut_ptr().cast(), buf.len()) };
    if p.is_null() {
        return None;
    }
    let end = buf.iter().position(|&b| b == 0)?;
    buf.truncate(end);
    Some(buf)
}

/// The path of the running executable.
///
/// Platform-specific because there is no portable call for it: macOS has
/// `_NSGetExecutablePath`, Linux has `/proc/self/exe`. `std` hides that behind one function and
/// so does this, but the two implementations are genuinely different and neither is a fallback
/// for the other.
pub fn current_exe() -> Option<Vec<u8>> {
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn _NSGetExecutablePath(buf: *mut libc::c_char, size: *mut u32) -> libc::c_int;
        }
        let mut size: u32 = 4096;
        let mut buf = alloc::vec![0u8; size as usize];
        // Returns -1 and writes the needed size when the buffer is too small; one retry is
        // enough because the second size is authoritative.
        if unsafe { _NSGetExecutablePath(buf.as_mut_ptr().cast(), &mut size) } != 0 {
            buf = alloc::vec![0u8; size as usize];
            if unsafe { _NSGetExecutablePath(buf.as_mut_ptr().cast(), &mut size) } != 0 {
                return None;
            }
        }
        let end = buf.iter().position(|&b| b == 0)?;
        buf.truncate(end);
        Some(buf)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let mut buf = alloc::vec![0u8; 4096];
        let n = unsafe {
            libc::readlink(c"/proc/self/exe".as_ptr(), buf.as_mut_ptr().cast(), buf.len())
        };
        if n <= 0 {
            return None;
        }
        buf.truncate(n as usize);
        Some(buf)
    }
}

/// The directory for temporary files.
///
/// `TMPDIR` if set, else `/tmp`. This is what `std::env::temp_dir` does on Unix.
pub fn temp_dir() -> Vec<u8> {
    match var_os("TMPDIR") {
        Some(v) if !v.is_empty() => v,
        _ => b"/tmp".to_vec(),
    }
}

/// Split a `PATH`-style variable on `:`.
///
/// Empty entries are dropped. `std::env::split_paths` keeps them and treats an empty entry as
/// the current directory, which is a shell convention no caller here relies on and a surprising
/// way to search `.` by accident.
pub fn split_paths(value: &[u8]) -> impl Iterator<Item = &[u8]> {
    value.split(|&b| b == b':').filter(|s| !s.is_empty())
}

/// The process's command-line arguments.
///
/// Platform-specific, like [`current_exe`], and for the same reason: there is no portable call.
/// macOS exposes the real `argv` through `_NSGetArgv`; Linux has it NUL-separated in
/// `/proc/self/cmdline`.
///
/// This is what a `main` would have been handed. It is read from the OS rather than captured at
/// startup, so a caller deep in the compiler can ask without an entry point having stashed it.
pub fn args() -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn _NSGetArgc() -> *mut libc::c_int;
            fn _NSGetArgv() -> *mut *mut *mut libc::c_char;
        }
        unsafe {
            let argc = *_NSGetArgc();
            let argv = *_NSGetArgv();
            if argv.is_null() {
                return out;
            }
            for i in 0..argc as isize {
                let p = *argv.offset(i);
                if p.is_null() {
                    break;
                }
                let mut arg = Vec::new();
                let mut j = 0isize;
                loop {
                    let b = *p.offset(j) as u8;
                    if b == 0 {
                        break;
                    }
                    arg.push(b);
                    j += 1;
                }
                out.push(arg);
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(raw) = crate::file::read("/proc/self/cmdline") {
            // Trailing NUL means a trailing empty split, which is not an argument.
            for arg in raw.split(|&b| b == 0) {
                if !arg.is_empty() {
                    out.push(arg.to_vec());
                }
            }
        }
    }
    out
}

/// Set a variable to raw bytes.
///
/// # Safety
/// Races any other thread reading the environment, exactly as `setenv` does.
pub unsafe fn set_var_bytes(name: &[u8], value: &[u8]) {
    let mut n = Vec::with_capacity(name.len() + 1);
    n.extend_from_slice(name);
    n.push(0);
    let mut v = Vec::with_capacity(value.len() + 1);
    v.extend_from_slice(value);
    v.push(0);
    unsafe { libc::setenv(n.as_ptr().cast(), v.as_ptr().cast(), 1) };
}

/// Every environment variable as a raw `KEY=VALUE` entry.
///
/// For building a child's environment: [`crate::command::Command::env`] starts from this and
/// overrides entries in it, because a child that inherits nothing loses `PATH`, `HOME` and
/// everything a toolchain reads, and a child that inherits blindly cannot be given anything.
///
/// Read from `environ` rather than assembled, so it is what the process actually has.
pub fn environ() -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    #[cfg(target_os = "macos")]
    let envp = unsafe {
        unsafe extern "C" {
            fn _NSGetEnviron() -> *mut *const *const libc::c_char;
        }
        *_NSGetEnviron()
    };
    #[cfg(not(target_os = "macos"))]
    let envp = unsafe { libc::environ as *const *const libc::c_char };
    if envp.is_null() {
        return out;
    }
    let mut i = 0isize;
    loop {
        let p = unsafe { *envp.offset(i) };
        if p.is_null() {
            return out;
        }
        let mut entry = Vec::new();
        let mut j = 0isize;
        loop {
            let b = unsafe { *p.offset(j) } as u8;
            if b == 0 {
                break;
            }
            entry.push(b);
            j += 1;
        }
        out.push(entry);
        i += 1;
    }
}
