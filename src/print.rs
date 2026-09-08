//! `eprintln!` and friends, over `write(2)`.
//!
//! Forty-eight `eprintln!` calls and four `eprint!` were the single largest category of remaining
//! `std` in the compiler, and they are also the shallowest: every one writes bytes to a file
//! descriptor. `std`'s versions carry a `LineWriter`, a lock, a panic-on-error policy and, on
//! stdout, a flush-at-exit hook. None of that is wanted here - the compiler's output is either a
//! diagnostic or a `-Z` trace, both of which want the bytes out now and neither of which should
//! abort compilation because a pipe closed.
//!
//! So: format into a `String`, write it, ignore a failed write. Ignoring it is deliberate and is
//! what `std` does *not* do - `std::eprintln!` panics on a broken pipe, which turns `ekod | head`
//! into a compiler crash.

use alloc::string::String;
use core::fmt::Write as _;

/// Write bytes to a descriptor, retrying a short write and giving up on any error.
///
/// A partial `write(2)` is not an error and not rare on a pipe, so the loop is required rather
/// than defensive. Errors are dropped: see the module note.
pub fn write_fd(fd: i32, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let n = unsafe { libc::write(fd, bytes.as_ptr().cast(), bytes.len()) };
        if n <= 0 {
            // EINTR is worth retrying; everything else, including EPIPE, is not our business.
            if n < 0 && crate::Errno::current().0 == libc::EINTR {
                continue;
            }
            return;
        }
        bytes = &bytes[n as usize..];
    }
}

/// The formatting half, kept out of the macro so the macro expands to one call.
pub fn emit(fd: i32, args: core::fmt::Arguments<'_>) {
    let mut s = String::new();
    // Formatting into a String cannot fail; the result is discarded to avoid an unwrap.
    let _ = s.write_fmt(args);
    write_fd(fd, s.as_bytes());
}

/// `eprintln!` without `std`.
#[macro_export]
macro_rules! eprintln {
    () => { $crate::print::write_fd(2, b"\n") };
    ($($arg:tt)*) => {{
        $crate::print::emit(2, format_args!($($arg)*));
        $crate::print::write_fd(2, b"\n");
    }};
}

/// `eprint!` without `std`.
#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => { $crate::print::emit(2, format_args!($($arg)*)) };
}

/// `println!` without `std`.
#[macro_export]
macro_rules! println {
    () => { $crate::print::write_fd(1, b"\n") };
    ($($arg:tt)*) => {{
        $crate::print::emit(1, format_args!($($arg)*));
        $crate::print::write_fd(1, b"\n");
    }};
}

/// `print!` without `std`.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => { $crate::print::emit(1, format_args!($($arg)*)) };
}

/// Whether a descriptor is a terminal, for the colour decision in diagnostics.
pub fn is_terminal(fd: i32) -> bool {
    unsafe { libc::isatty(fd) == 1 }
}
