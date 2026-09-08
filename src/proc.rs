//! The process: its id, and how many threads the machine will usefully run.

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
    if n < 1 { 1 } else { n as usize }
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
