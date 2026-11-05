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
    if core::str::from_utf8(&bytes).is_ok() {
        Some(unsafe { String::from_utf8_unchecked(bytes) })
    } else {
        None
    }
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

fn zero_bytes(len: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(len);
    let mut i = 0;
    while i < len {
        buf.push(0);
        i += 1;
    }
    buf
}

fn find_byte(mut bytes: &[u8], needle: u8) -> usize {
    let len = bytes.len();
    let mut found = len;
    let mut index = 0;
    while index < len {
        if found == len && bytes[index] == needle {
            found = index;
        }
        index += 1;
    }
    bytes = &bytes[..];
    let _ = bytes.len();
    found
}

fn leading_colons(mut bytes: &[u8]) -> usize {
    let len = bytes.len();
    let mut skip = len;
    let mut index = 0;
    while index < len {
        if skip == len && bytes[index] != b':' {
            skip = index;
        }
        index += 1;
    }
    bytes = &bytes[..];
    let _ = bytes.len();
    skip
}

/// The process working directory, as bytes.
pub fn current_dir() -> Option<Vec<u8>> {
    let mut buf = zero_bytes(4096);
    let p = unsafe { libc::getcwd(buf.as_mut_ptr().cast(), buf.len()) };
    if p.is_null() {
        return None;
    }
    let end = find_byte(buf.as_slice(), 0);
    if end == buf.len() {
        return None;
    }
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
        let mut size: u32 = 0;
        let _ = unsafe { _NSGetExecutablePath(core::ptr::null_mut(), &mut size) };
        let mut buf = zero_bytes(size as usize);
        if unsafe { _NSGetExecutablePath(buf.as_mut_ptr().cast(), &mut size) } != 0 {
            return None;
        }
        let end = find_byte(buf.as_slice(), 0);
        if end == buf.len() {
            return None;
        }
        buf.truncate(end);
        Some(buf)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let mut buf = zero_bytes(4096);
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

#[inline(never)]
pub(crate) unsafe fn cstr_to_vec(p: *const libc::c_char) -> Vec<u8> {
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
    out
}

/// The directory for temporary files.
///
/// `TMPDIR` if set, else `/tmp`. This is what `std::env::temp_dir` does on Unix.
pub fn temp_dir() -> Vec<u8> {
    match var_os("TMPDIR") {
        Some(v) => {
            if v.is_empty() {
                b"/tmp".to_vec()
            } else {
                v
            }
        }
        None => b"/tmp".to_vec(),
    }
}

/// Split a `PATH`-style variable on `:`.
///
/// Empty entries are dropped. `std::env::split_paths` keeps them and treats an empty entry as
/// the current directory, which is a shell convention no caller here relies on and a surprising
/// way to search `.` by accident.
struct SplitPaths<'a> {
    rest: &'a [u8],
}

impl<'a> Iterator for SplitPaths<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let skip = leading_colons(self.rest);
        let (_, trimmed) = self.rest.split_at(skip);
        self.rest = trimmed;
        if self.rest.is_empty() {
            return None;
        }
        let end = find_byte(self.rest, b':');
        let (item, rest) = self.rest.split_at(end);
        self.rest = rest;
        Some(item)
    }
}

pub fn split_paths(value: &[u8]) -> impl Iterator<Item = &[u8]> {
    SplitPaths { rest: value }
}

#[inline(never)]
pub(crate) unsafe fn cstr_table_at(
    table: *const *const libc::c_char,
    index: usize,
) -> *const libc::c_char {
    let byte_offset = index * core::mem::size_of::<*const libc::c_char>();
    let slot = unsafe { table.cast::<u8>().add(byte_offset).cast::<*const libc::c_char>() };
    unsafe { *slot }
}

unsafe fn copy_bytes(pair: (*mut u8, *const u8), len: usize) {
    let (dst, src) = pair;
    let mut copied = 0;
    while copied < len {
        unsafe { *dst.add(copied) = *src.add(copied) };
        copied += 1;
    }
}

unsafe fn step_push_cstr(
    table: *const *const libc::c_char,
    index: usize,
    out: &mut Vec<Vec<u8>>,
) -> bool {
    let p = unsafe { cstr_table_at(table, index) };
    if p.is_null() {
        return false;
    }
    let item = unsafe { cstr_to_vec(p) };
    let len = out.len();
    out.reserve(1);
    let byte_offset = len * core::mem::size_of::<Vec<u8>>();
    let dst = unsafe { out.as_mut_ptr().cast::<u8>().add(byte_offset) };
    let src = core::ptr::from_ref(&item).cast::<u8>();
    unsafe { copy_bytes((dst, src), core::mem::size_of::<Vec<u8>>()) };
    core::mem::forget(item);
    unsafe { out.set_len(len + 1) };
    true
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
            if argv.is_null() || argc <= 0 {
                return out;
            }
            let table = argv as *const *const libc::c_char;
            let count = argc as usize;
            let mut i = 0usize;
            while i < count {
                let _ = step_push_cstr(table, i, &mut out);
                i += 1;
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

pub(crate) unsafe fn raw_environ_ptr() -> *const *const libc::c_char {
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn _NSGetEnviron() -> *mut *const *const libc::c_char;
        }
        unsafe { *_NSGetEnviron() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        unsafe extern "C" {
            static environ: *const *const libc::c_char;
        }
        unsafe { environ }
    }
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
    let envp = unsafe { raw_environ_ptr() };
    if envp.is_null() {
        return out;
    }
    let mut i = 0usize;
    loop {
        if !unsafe { step_push_cstr(envp, i, &mut out) } {
            break;
        }
        i += 1;
    }
    out
}
