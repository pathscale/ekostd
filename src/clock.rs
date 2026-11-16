//! A monotonic nanosecond counter that needs no operating system.
//!
//! # Why this module exists at all
//!
//! Timing points belong in `loopir` and `fjb`, and neither may have them: the no-std ratchet
//! forbids a new file reaching for `std::`, so `Instant` is not available where the time is
//! actually spent. `core` has no clock, because a clock is a property of the machine rather
//! than of the language.
//!
//! The machine has one. On aarch64 the virtual counter `CNTVCT_EL0` is readable from user space
//! in a single instruction, with no syscall, no vDSO and no allocation, which is what makes it
//! usable at the granularity a compiler's phases actually run at.
//!
//! # Why it is the instruction and not a crate around it
//!
//! It used to be `no_std_time::read_time_counter`, which is that same instruction plus x86, arm
//! and riscv. That one call brought `no_std_time` and the `rustversion` proc macro into every
//! build of the compiler, and the compiler has to be able to build everything it depends on.
//! The clock already carried one audited `mrs` read, for the cpu id; the counter is a second.
//!
//! What this module adds is the part a general counter cannot: **a stated unit**, and a test that
//! holds it to it.
//!
//! # The unit, and the trap, stated once so nobody re-derives it
//!
//! **On aarch64 `now_ns` is nanoseconds: the tick count scaled by `CNTFRQ_EL0`.** It advances in steps of 41.67 ns
//! because the timer runs at 24 MHz, but that is its *resolution*, not its unit. Multiplying a
//! delta by the step size to "convert to nanoseconds" inflates every number by 41.67x. That
//! mistake was made once already, in the `ps-st3` benchmark harness, and it made a queue's
//! latency look three orders of magnitude worse than it was.
//!
//! **Off aarch64 there is no counter here and `now_ns` returns zero.** [`is_real`] says so, and
//! the reports print it, so a table of zeros is never read as a fast compiler.
//!
//! The consequence for callers: a region shorter than ~42 ns reads as either 0 or 42, so time
//! regions coarser than that, or count them instead.
//!
//! Until 16 November 2026 this was ekopathrs's `clock` crate. It is general, not compiler
//! specific, so it lives here and every consumer (ekod, ekors, the door transport in
//! endpoint-libs, PathDB) reads it as `eko::clock`.

/// A monotonic count of nanoseconds from an unspecified origin.
///
/// Only differences are meaningful. The origin is whatever the machine booted with.
#[inline]
pub fn now_ns() -> u64 {
    #[cfg(target_arch = "aarch64")]
    {
        let (ticks, hz): (u64, u64);
        // SAFETY: `CNTVCT_EL0` and `CNTFRQ_EL0` are readable from user space and the reads have
        // no side effects.
        unsafe {
            core::arch::asm!("mrs {}, cntvct_el0", out(reg) ticks, options(nomem, nostack));
            core::arch::asm!("mrs {}, cntfrq_el0", out(reg) hz, options(nomem, nostack));
        }
        // The counter counts ticks of `hz`. 24 MHz here, so a tick is 41.67 ns; the test in
        // `tests/counts_nanoseconds.rs` fails if this scaling is dropped or applied twice.
        ((ticks as u128 * 1_000_000_000) / hz.max(1) as u128) as u64
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        0
    }
}

/// Whether [`now_ns`] is nanoseconds on this target rather than cycles or a constant.
///
/// The reports print this, so a table in the wrong unit is never read as a fast compiler.
pub const fn is_real() -> bool {
    cfg!(target_arch = "aarch64")
}

/// Which CPU this code is running on, as the kernel last set it.
///
/// On Darwin aarch64 `TPIDR_EL0` carries the cpu id in bits 0-11 and the cluster id in bits
/// 12-19. It is a hint, not a pin: the thread can migrate between the read and the next
/// instruction. That is fine for the thing it is for - counting how many distinct cores were
/// inside a phase at once - and useless for anything that needs to be true afterwards.
#[inline]
pub fn cpu_id() -> u16 {
    #[cfg(target_arch = "aarch64")]
    {
        current_cpu()
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        0
    }
}

#[cfg(target_arch = "aarch64")]
#[inline]
fn current_cpu() -> u16 {
    let value: u64;
    // SAFETY: `TPIDR_EL0` is readable from user space on Darwin aarch64 and the read has no
    // side effects. The value is advisory: the thread may migrate immediately afterwards.
    unsafe {
        core::arch::asm!("mrs {}, tpidr_el0", out(reg) value, options(nomem, nostack));
    }
    (value & 0xfff) as u16
}
