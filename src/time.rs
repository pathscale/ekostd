//! A monotonic clock reading.
//!
//! `core::time::Duration` is already `no_std`; only the *reading* is not, because taking one is a
//! syscall. This is `std::time::Instant` and nothing else: the compiler measures elapsed time and
//! never asks the wall clock what day it is.

use core::time::Duration;

/// A point on the monotonic clock.
///
/// Monotonic, so it does not move when the system clock is adjusted - which is the property the
/// measurements need and the reason this is not `gettimeofday`.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Instant {
    nanos: u128,
}

impl Instant {
    /// Read the clock.
    pub fn now() -> Instant {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
        Instant { nanos: (ts.tv_sec as u128) * 1_000_000_000 + (ts.tv_nsec as u128) }
    }

    /// Time since an earlier reading.
    ///
    /// Saturates at zero rather than panicking. `std` panics here when `earlier` is later, which
    /// on a monotonic clock means the caller compared two readings in the wrong order - a bug
    /// worth a wrong number rather than a killed compilation.
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        let d = self.nanos.saturating_sub(earlier.nanos);
        Duration::new((d / 1_000_000_000) as u64, (d % 1_000_000_000) as u32)
    }

    /// Time since this reading.
    pub fn elapsed(&self) -> Duration {
        Instant::now().duration_since(*self)
    }
}

impl core::ops::Sub for Instant {
    type Output = Duration;
    fn sub(self, earlier: Instant) -> Duration {
        self.duration_since(earlier)
    }
}

/// Seconds since the Unix epoch.
///
/// Not an `Instant`: this is the wall clock, it moves when the system clock is adjusted, and the
/// two must not be confused. Whole seconds because both callers - a timestamp stamped into a file
/// and a file-modification comparison - are working at that resolution already.
pub fn epoch_secs() -> i64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts) };
    // As in `file::Metadata::modified_secs`: `time_t` is `i64` here, and a target where it is
    // not gets a type error rather than a cast that quietly loses the top half.
    ts.tv_sec
}
