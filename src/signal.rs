//! Signals, delivered to a file descriptor so a reactor can await them.
//!
//! # The problem this solves
//!
//! A serving process is stopped by a signal, not by a call. `systemctl stop`,
//! `launchd` unload, `docker stop` and Fly's stop all send **`SIGTERM`**, and
//! the kernel's default disposition for it ends the process where it stands.
//! Anything the process was going to write is lost, and a store that
//! acknowledges a write when it is *queued* rather than when it is on disk is
//! torn by that exit.
//!
//! So the signal has to reach ordinary code that can finish up. Two ways to get
//! there, and this module deliberately takes the second.
//!
//! **A handler is not a place to do work.** It runs between two instructions of
//! whatever thread happened to receive the signal, and only a short list of
//! calls is defined there. Closing a database, draining a queue, taking a mutex
//! and formatting a log line are all reachable from a shutdown and all
//! undefined in a handler.
//!
//! **So the handler does one thing: `write(2)`.** That call is on the
//! async-signal-safe list, the write end is a pipe this module owns, and the
//! signal number is the one byte it carries. Everything else happens on the
//! read side, in ordinary code, at whatever time the reader chooses. This is
//! what every runtime that offers an awaitable signal does underneath, and it
//! is why the interface here is a file descriptor rather than a callback.
//!
//! # Why a descriptor and not a thread
//!
//! `sigwait` is the other standard answer: block the signal in every thread,
//! then dedicate one thread to waiting for it. It is correct, and it costs a
//! thread that spends its life parked, plus an ordering rule that is easy to
//! break. `pthread_sigmask` is **per-thread and inherited**, so the block has
//! to happen on the main thread before anything else spawns one; block late,
//! and the threads that already exist keep the default disposition and the
//! process dies on the signal it meant to catch. Nothing can detect that at
//! runtime, because by the time the mask is read the threads are already there.
//!
//! A descriptor has no such rule. `sigaction` replaces the disposition process
//! wide, before or after any thread exists, and the read end plugs into
//! whatever is already waiting on file descriptors. A reactor registers it and
//! wakes a task; a thread with no reactor reads it directly. No thread is
//! spent, and there is no window to get the ordering wrong in.
//!
//! # Losing a duplicate is not losing the signal
//!
//! The write end is non-blocking and a write that finds the pipe full is
//! dropped. That is deliberate: the reader only asks *whether* a stop arrived,
//! and a full pipe means it has already been told, many times over. Blocking in
//! a handler to deliver a fourteenth copy of the same number would be the one
//! way this design can hang.
//!
//! # What is absent
//!
//! No mask is touched, so nothing has to be restored and there is no `Drop`.
//! `sigaction` returns the previous disposition and this module drops it: a
//! caller that installs a handler here has said which signals it wants, and
//! putting the default back part way through a shutdown is how the process
//! reacquires the disposition it was avoiding.

use crate::Errno;

/// The signals a serving process stops on.
///
/// Named rather than numbered because a caller writes `Signal::Term` once and
/// the platform's number is this module's business. Three, because these are
/// the three a supervisor sends: a process that stops on anything else is
/// stopping on a signal somebody chose for another reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signal {
    /// `SIGTERM`. The ask to stop, and the one a supervisor actually sends:
    /// `systemctl stop`, `launchctl` unload, `docker stop`, Fly's stop and a
    /// plain `kill` all arrive as this.
    Term,
    /// `SIGINT`. The same ask, typed. `Ctrl-C`.
    Int,
    /// `SIGHUP`. The controlling terminal went away.
    ///
    /// Here because a daemonised process has no terminal to lose, so the
    /// terminal's own hang-up cannot mean what it says, and the kernel's
    /// default would otherwise end the process the same way an unhandled
    /// `SIGTERM` does. A stop that arrives on purpose should be able to be one.
    Hup,
}

impl Signal {
    /// Every signal this module knows, which is also what it will install.
    pub const ALL: [Signal; 3] = [Signal::Term, Signal::Int, Signal::Hup];

    /// The number the platform knows it by.
    ///
    /// A match rather than arithmetic: `libc` names each one per target, and
    /// the numbers agree across the platforms here by convention rather than by
    /// contract.
    #[must_use]
    pub fn number(self) -> i32 {
        match self {
            Signal::Term => libc::SIGTERM,
            Signal::Int => libc::SIGINT,
            Signal::Hup => libc::SIGHUP,
        }
    }

    /// The name a log line should use, so a report of what stopped the process
    /// reads the way the man page spells it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Signal::Term => "SIGTERM",
            Signal::Int => "SIGINT",
            Signal::Hup => "SIGHUP",
        }
    }

    /// The signal a number names, if this module knows it.
    #[must_use]
    pub fn from_number(number: i32) -> Option<Signal> {
        Signal::ALL.into_iter().find(|signal| signal.number() == number)
    }
}

impl core::fmt::Display for Signal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name())
    }
}

/// The write end, which is where a handler sends.
///
/// A bare `libc::c_int` rather than a wrapper, because a signal handler is
/// `extern "C"` and can close over nothing that is not plain data. The value is
/// a descriptor this process owns for its whole life, written once by
/// [`Signals::install`] before any handler is installed, so there is nothing to
/// free and no question about which end a handler sees.
static mut WRITE_END: libc::c_int = -1;

/// What a handler does, and the whole of it.
///
/// One `write(2)`, which is async-signal-safe. Every signal number fits in one
/// byte, and the number is the payload: the reader needs to know which one
/// arrived so it can name it in a log line, and it needs nothing else.
extern "C" fn note(signal: libc::c_int) {
    // SAFETY: `WRITE_END` is written once in `install` before any handler is
    // installed, and only read here afterwards. A `libc::c_int` read is atomic
    // on every platform this crate builds for, and the value is a live
    // descriptor for the life of the process.
    let fd = unsafe { WRITE_END };
    if fd < 0 {
        return;
    }
    // Truncation is the intent: the pipe carries one byte per signal and the
    // numbers here are all well under 128.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = [signal as u8];
    // SAFETY: `fd` is a live descriptor this process owns, `byte` is a live
    // one-element array, and `write` retains neither. A full pipe or an
    // interrupted call is fine to ignore: both mean the reader has more to do
    // than it has done, which is the only thing this handler is for.
    unsafe { libc::write(fd, byte.as_ptr().cast(), byte.len()) };
}

/// An installed signal disposition, and the descriptor that reports it.
///
/// Held for the life of the process. Dropping it uninstalls nothing: see the
/// module docs on why putting the default back is the wrong move.
pub struct Signals {
    read_end: libc::c_int,
}

impl Signals {
    /// Install a handler for each of `signals`, and return the read end.
    ///
    /// Replaces the disposition for every signal named, process wide and
    /// immediately, so **no thread needs to exist first** and no mask is
    /// touched. That is the whole reason this is not `sigwait`.
    ///
    /// # Errors
    ///
    /// When the pipe cannot be made, or a disposition cannot be installed.
    /// Either one leaves the process with a signal it cannot catch, which is a
    /// condition to report and not to carry on from: a server that thinks it
    /// catches `SIGTERM` and does not will be torn by the first supervisor that
    /// stops it.
    pub fn install(signals: &[Signal]) -> Result<Signals, Errno> {
        let mut ends = [0 as libc::c_int; 2];
        // SAFETY: `ends` is a live array of exactly the two descriptors `pipe`
        // writes, and it outlives the call.
        checked(unsafe { libc::pipe(ends.as_mut_ptr()) })?;
        let [read_end, write_end] = ends;

        // Non-blocking, on both ends and for two different reasons. The reader
        // gets it so that draining is a loop that stops at `EWOULDBLOCK` rather
        // than one that has to know how many bytes are waiting. The writer gets
        // it so a full pipe drops a duplicate instead of blocking a handler.
        nonblocking(read_end)?;
        nonblocking(write_end)?;

        // Before any handler is installed, so no delivery can ever observe the
        // -1 sentinel and drop a signal on the floor.
        // SAFETY: a single `libc::c_int` static, written once here on the
        // thread that installs, before any handler can run. See `note`.
        unsafe { WRITE_END = write_end };

        let action = Action::noting();
        for signal in signals {
            // SAFETY: `action` is a live, fully initialised `sigaction`, and
            // the third argument is null because the previous disposition is
            // deliberately not kept. See the module docs.
            checked(unsafe {
                libc::sigaction(signal.number(), action.as_ref(), core::ptr::null_mut())
            })?;
        }
        Ok(Signals { read_end })
    }

    /// The read end, non-blocking, for a reactor to register or a thread to read.
    #[must_use]
    pub fn fd(&self) -> libc::c_int {
        self.read_end
    }

    /// Read whatever has arrived, without blocking.
    ///
    /// The primitive underneath both a reactor's readiness wakeup and a plain
    /// blocking read, and the reason it is public is that a reactor needs it:
    /// readiness for a `pipe` means *some* bytes are there, and draining them is
    /// the reader's job rather than the poller's.
    ///
    /// `Ok(None)` is nothing pending right now, which is the ordinary result
    /// when a readiness edge was consumed by another read. A number this module
    /// does not know is skipped rather than returned: a caller has no use for
    /// one, and reporting it would mean inventing a meaning for it.
    pub fn recv(&self) -> Result<Option<Signal>, Errno> {
        let mut bytes = [0_u8; 64];
        // SAFETY: `bytes` is a live array, the length passed is its own, and
        // the descriptor is non-blocking so this returns rather than parks.
        let read = unsafe { libc::read(self.read_end, bytes.as_mut_ptr().cast(), bytes.len()) };
        if read == -1 {
            let error = Errno::current();
            // A non-blocking read with nothing there is not an error, and it is
            // the answer a reactor's speculative read gets most of the time.
            return if error.would_block() { Ok(None) } else { Err(error) };
        }
        // A zero-length read on a pipe means every writer is gone, which here
        // means every handler is gone, which cannot happen while `WRITE_END`
        // holds one open. Reported as nothing pending rather than as a closed
        // pipe, because a caller looping on this has no other way to continue.
        #[allow(clippy::cast_sign_loss)]
        let read = read.max(0) as usize;
        let signalled =
            bytes[..read].iter().find_map(|byte| Signal::from_number(libc::c_int::from(*byte)));
        Ok(signalled)
    }
}

/// A `sigaction`, built once and installed for each signal.
struct Action {
    inner: libc::sigaction,
}

impl Action {
    /// An action that calls [`note`] and asks for interrupted calls to restart.
    ///
    /// **`SA_RESTART` is not an optimisation.** Without it, a signal delivered
    /// to a thread parked in a syscall makes that call return `EINTR`, and every
    /// read, write and `accept` in the process acquires a retry path it did not
    /// have. The signal is not about those calls: it means stop, and cutting an
    /// unrelated write short on its way past is noise the rest of the program
    /// would have to be written to tolerate. macOS does not define `SA_RESTART`
    /// as a separate flag for `read`, where restarting is the default, so this
    /// is a no-op there and the correct request elsewhere.
    fn noting() -> Action {
        // SAFETY: `sigaction` is a plain C struct of integers and a function
        // pointer, so a zeroed one is a valid value; every field that matters
        // is set below.
        let mut inner: libc::sigaction = unsafe { core::mem::zeroed() };
        // Two casts rather than one: `sa_sigaction` is an integer of pointer
        // width in libc's own definition, and casting a function item straight
        // to an integer is not how a function pointer is formed. Going through
        // `*const ()` is that step, stated.
        inner.sa_sigaction = note as *const () as usize;
        inner.sa_flags = libc::SA_RESTART;
        // SAFETY: `sa_mask` is a live `sigset_t` owned by `inner`, borrowed for
        // the call, and `sigemptyset` writes every byte of it.
        unsafe { libc::sigemptyset(&mut inner.sa_mask) };
        Action { inner }
    }

    fn as_ref(&self) -> *const libc::sigaction {
        &self.inner
    }
}

/// Set `O_NONBLOCK` on a descriptor this module owns.
fn nonblocking(fd: libc::c_int) -> Result<(), Errno> {
    // SAFETY: `fd` is a live descriptor and `F_SETFL`/`O_NONBLOCK` are both
    // integers; this call reads no memory through a pointer.
    let flags = checked(unsafe { libc::fcntl(fd, libc::F_GETFL) })?;
    // SAFETY: as above. `fcntl` with `F_SETFL` takes the flags by value.
    checked(unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) })?;
    Ok(())
}

/// Keep the callers of the libc calls here in one shape.
///
/// Every one of them reports failure as `-1` and sets `errno`, so a caller
/// reads one type rather than one per call site.
fn checked(returned: i32) -> Result<i32, Errno> {
    if returned == -1 {
        Err(Errno::current())
    } else {
        Ok(returned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test process has one signal disposition, so these run one at a time.
    ///
    /// Two of them installing `SIGTERM` in parallel would have the second
    /// replace the first's handler while the first is mid-assertion, and the
    /// failure would read as a lost signal rather than as a test collision.
    static ONE_AT_A_TIME: crate::thread::Mutex<()> = crate::thread::Mutex::new(());

    /// **A signal that arrives before anyone reads the descriptor is not lost.**
    ///
    /// This is the whole reason the interface is a pipe rather than a callback:
    /// the handler writes, the byte waits in the pipe, and the read happens
    /// whenever the reader gets to it. The window this covers is real: a
    /// supervisor restarting a service sends `SIGTERM` while the process is
    /// still opening its store, and a design that dropped it would hang until
    /// the supervisor lost patience and sent `SIGKILL`, killing the drain the
    /// signal was supposed to start.
    #[test]
    fn a_signal_raised_before_the_first_read_is_still_there() {
        let _guard = ONE_AT_A_TIME.lock();
        let signals = Signals::install(&Signal::ALL).expect("install");

        // Raised here, read below, with nothing in between. The byte sits in the
        // pipe for the whole gap.
        // SAFETY: `raise` sends a signal to this thread, and the disposition for
        // it is the handler installed above.
        assert_eq!(unsafe { libc::raise(libc::SIGTERM) }, 0, "raise SIGTERM");

        // The handler has run by the time `raise` returns, so the byte is in the
        // pipe already. No polling and no sleep.
        assert_eq!(
            signals.recv().expect("read the pipe"),
            Some(Signal::Term),
            "a raised signal that arrived before the read must still be reported"
        );
        // And the next read is empty rather than reporting the same one twice,
        // because a pipe is a queue and not a flag.
        assert_eq!(signals.recv().expect("read the pipe"), None);
    }

    /// Every signal in the set names itself, so a log line can say which.
    #[test]
    fn the_set_reports_which_signal_arrived() {
        let _guard = ONE_AT_A_TIME.lock();
        let signals = Signals::install(&Signal::ALL).expect("install");
        for signal in Signal::ALL {
            // SAFETY: as above.
            assert_eq!(unsafe { libc::raise(signal.number()) }, 0, "raise {signal}");
            assert_eq!(
                signals.recv().expect("read the pipe"),
                Some(signal),
                "{signal} should be reported by name"
            );
        }
    }

    /// The pipe is non-blocking, so a reader with nothing to read is told so
    /// rather than parked. A reactor depends on this: its poller reports
    /// readiness for a descriptor, and reading is the reactor's own step.
    #[test]
    fn an_empty_pipe_reports_nothing_rather_than_blocking() {
        let _guard = ONE_AT_A_TIME.lock();
        let signals = Signals::install(&Signal::ALL).expect("install");
        assert_eq!(signals.recv().expect("read the pipe"), None);
    }
}
