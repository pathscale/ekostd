//! The operating system, wrapped once, for everything here that needs it.
//!
//! # Why this is a crate and not a module
//!
//! It was `daemon::sys`, written to take `crates/daemon` off the `no_std` exemption list. That
//! worked, and then the real question turned out to be `frontend/`: `ekod` is 66 MB because it
//! links sixty-six `rustc_*` crates, and 537 files across them use `std::` - which is the number
//! the ratchet exists to drive down, since every one is a file the self-hosting stage carries.
//!
//! `frontend/` cannot depend on `crates/daemon`, so the wrapper had to come out. This is that,
//! and it is where it was first asked to be.
//!
//! # Why this and not a trait
//!
//! The alternative was `embedded-io`: make the io a pair of traits and let the caller supply the
//! file or the socket, which is what `~/code/worktable-vec` does and is right there - a table
//! genuinely does not care what it is stored on, and a caller with no operating system is a real
//! caller. This compiler has one platform and will not run without one, so trait-abstracting a
//! Unix socket buys a generality nobody asked for and leaves the socket still to be written
//! somewhere, against `std`. Wrapping it once is the smaller thing.
//!
//! **A `no_std` crate wrapping `libc` is still `no_std`.** The ratchet exists because every file
//! the compiler adds against `std` is one the self-hosting stage has to carry. It does not have to
//! carry `libc`: the platform already has it, and a bootstrap that can link C could link C before
//! this module existed.
//!
//! The alternative considered and rejected was `embedded-io` - make the io a pair of traits and
//! let the caller supply the socket, which is what `~/code/worktable-vec` does. That is right
//! there: a table genuinely does not care what it is stored on, and a caller with no operating
//! system is a real caller. This compiler has one platform and will not run without one, so
//! trait-abstracting a Unix socket buys a generality nobody asked for and leaves the socket still
//! to be written somewhere, against `std`. Wrapping it once is the smaller thing.

// `daemon` is `#![no_std]`. These arrive with the standard prelude and have no path to
// match, which is why a `std::` grep cannot see them and the attribute has to be flipped
// to find them at all.

#![no_std]
// The eyepatch on `thread::Mutex`'s destructor, behind a feature because it is a nightly one.
//
// See the `Drop` impl for why it is load-bearing: without it a `Mutex` anywhere inside a type
// makes that type drop-significant, and rustc's `'tcx` origin-point pattern in
// `rustc_interface::passes` stops compiling. A consumer that is not rustc does not need it, and
// making it unconditional would put every consumer of this crate on nightly for a guarantee only
// one of them uses. `hashbrown` gates its nightly pieces the same way.
#![cfg_attr(feature = "nightly", feature(dropck_eyepatch))]

// ---------------------------------------------------------------------------------------------
// STD IS BANNED IN THIS CRATE.
//
// `#![no_std]` above is the ban and the compiler is the enforcer: without `extern crate std;`
// there is no `std` in the extern prelude, so any `std::` path fails to resolve and the build
// stops. Do not add that line back to make an error go away - the error is the point. Whatever
// needed `std` either has a `core`/`alloc` equivalent, belongs in `crates/libc-wrapper`, or is a
// dependency that has to be replaced.
//
// The prelude is the part a grep cannot see: `Vec`, `String`, `Box`, `format!`, `vec!`,
// `thread_local!` and `println!` name no path. Under `#![no_std]` they resolve through `alloc`
// and `libc_wrapper` instead, which is why those imports appear at the top of every file here.
// ---------------------------------------------------------------------------------------------
extern crate alloc;

pub mod channel;
pub mod command;
pub mod cpu;
pub mod daemonize;
pub mod env;
pub mod file;
pub mod fs;
// `heap`, not `alloc`: this crate says `extern crate alloc;` and the two names collide.
pub mod heap;
pub mod math;
pub mod mmap;
pub mod path;
pub mod print;
pub mod proc;
pub mod socket;
pub mod thread;
pub mod time;

/// A libc call that failed, as the `errno` it set.
///
/// **Not a string, and not `std::io::Error`.** The first loses which error it was; the second is
/// the thing this module exists to stop linking. A caller that wants a sentence asks for one; a
/// caller that wants to branch matches the number, which is what the C header documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Errno(pub i32);

impl Errno {
    /// This thread's `errno`.
    ///
    /// A function returning a pointer to a thread-local, in every modern libc, which is why this
    /// is not a global read. The symbol differs by platform.
    pub fn current() -> Errno {
        #[cfg(target_vendor = "apple")]
        let p = unsafe { libc::__error() };
        #[cfg(all(unix, not(target_vendor = "apple")))]
        let p = unsafe { libc::__errno_location() };
        Errno(unsafe { *p })
    }

    /// Set `errno` back to zero.
    ///
    /// For the one shape of libc call that reports success and failure identically:
    /// `readdir` returns null both at the end of a directory and on an error, and the only way
    /// to tell them apart is to clear `errno` first and read it after.
    pub fn clear() {
        #[cfg(target_vendor = "apple")]
        let p = unsafe { libc::__error() };
        #[cfg(all(unix, not(target_vendor = "apple")))]
        let p = unsafe { libc::__errno_location() };
        unsafe { *p = 0 };
    }

    /// The system's own sentence for this error.
    ///
    /// `strerror_r` rather than `strerror`, which is not thread-safe. The buffer belongs to the
    /// caller because handing back a pointer into a local would be handing back a dangling one.
    pub fn message(self, into: &mut [u8; 128]) -> &str {
        let ok = unsafe { libc::strerror_r(self.0, into.as_mut_ptr().cast(), into.len()) };
        if ok != 0 {
            return "unknown error";
        }
        let end = into.iter().position(|b| *b == 0).unwrap_or(into.len());
        core::str::from_utf8(&into[..end]).unwrap_or("unknown error")
    }
}

impl core::fmt::Display for Errno {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut buf = [0u8; 128];
        write!(f, "{} (errno {})", self.message(&mut buf), self.0)
    }
}

/// Every wrapper here goes through this, so "checked the return value" is a property of the
/// module rather than something to verify one call at a time.
pub(crate) fn checked(returned: i32) -> Result<i32, Errno> {
    if returned == -1 {
        Err(Errno::current())
    } else {
        Ok(returned)
    }
}

/// End this process now, without unwinding and without running destructors.
///
/// The same call `std::process::exit` makes.
pub fn exit(code: i32) -> ! {
    unsafe { libc::exit(code) }
}

/// Sleep for a whole number of milliseconds.
///
/// `nanosleep` rather than `sleep`, because the callers want tens of milliseconds and `sleep`
/// takes seconds. A signal that interrupts it returns early and that is deliberate: both callers
/// are polling loops with their own deadline, so a short sleep costs one extra turn and a
/// restart loop here would swallow a signal the process wants to see.
pub fn sleep_ms(ms: u64) {
    let spec = libc::timespec {
        tv_sec: (ms / 1000) as libc::time_t,
        tv_nsec: ((ms % 1000) * 1_000_000) as _,
    };
    unsafe { libc::nanosleep(&spec, core::ptr::null_mut()) };
}
