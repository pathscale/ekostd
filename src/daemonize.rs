//! Going into the background, and not lying about when.
//!
//! # What this replaced
//!
//! The `daemonize` crate, 781 lines over `libc`, plus 150 lines in `coupled::background` doing
//! the half it deliberately leaves out. Taking the crate was the right call the day it was made -
//! the twelve steps are easy to get subtly wrong and it had them - but it links `std`, and this
//! crate is trying not to. What is left here is the same twelve steps against the same libc, plus
//! the readiness pipe that was already ours.
//!
//! The steps, in the order the C write-up everyone links gives them: fork so the parent can
//! return, `setsid` so the child leads a new session with no controlling terminal, fork again so
//! the daemon is not a session leader and can never acquire one, `chdir("/")` so a directory
//! cannot be held busy, `umask` so file modes do not depend on whoever started it, `dup2` the
//! standard streams to `/dev/null`, and a pid file opened `O_CREAT | O_EXCL` and flocked so a
//! second daemon refuses instead of both believing they own it.
//!
//! # The race, and why the pipe is not decoration
//!
//! A fork returning says nothing about whether the daemon is serving. `ekod` makes that window
//! wide on purpose: it binds, compiles every path on the start-up line, then writes its
//! descriptor. A preloaded corpus is seconds, and a client that started on the fork would find no
//! socket on a start that was going to succeed.
//!
//! So a pipe is opened before the first fork and inherited across both. The parent blocks reading
//! it and exits only when one of two things happens:
//!
//! * **a byte arrives** - the daemon called [`Ready::signal`], having bound and preloaded. The
//!   parent exits 0, and a script whose next line is `ekors ping` reaches a server that is there.
//! * **the pipe closes with nothing on it** - the daemon exited before that point. Every copy of
//!   the write end is gone, the read returns 0, the parent exits nonzero. There is no path where a
//!   broken start-up leaves a parent waiting, because [`Ready`]'s `Drop` closes the write end.
//!
//! The standard step this deliberately does **not** do is the `getrlimit` loop that closes every
//! non-standard descriptor. The pipe has to survive both forks, and closing it is exactly what
//! would make the parent hang forever. Only 0, 1 and 2 are redirected.

// `daemon` is `#![no_std]`. These arrive with the standard prelude and have no path to
// match, which is why a `std::` grep cannot see them and the attribute has to be flipped
// to find them at all.

use crate::{checked, Errno};

/// The daemon's end of the readiness pipe.
///
/// Dropping it without signalling closes the write end, which is what tells the parent the
/// start-up failed. That is why this is a guard and not a bare descriptor.
pub struct Ready {
    write_fd: libc::c_int,
}

impl Ready {
    /// Tell the parent this process is serving, and let it exit.
    ///
    /// Called once, where the next thing a client does would succeed. Everything before it -
    /// binding, preloading, writing the descriptor - is inside the window the parent waits on.
    ///
    /// A failed write is ignored. The only ways it fails are the parent having died or been
    /// killed, and in both the daemon is running correctly and has nobody to tell. Turning that
    /// into a start-up error would stop a good server because whoever launched it walked away.
    pub fn signal(self) {
        let byte = 1u8;
        unsafe {
            libc::write(self.write_fd, core::ptr::from_ref(&byte).cast(), 1);
            libc::close(self.write_fd);
        }
        core::mem::forget(self);
    }
}

impl Drop for Ready {
    fn drop(&mut self) {
        // Closing without writing is the failure signal, and the parent is already reading.
        unsafe { libc::close(self.write_fd) };
    }
}

/// What this process is, after asking to go into the background.
pub enum Started {
    /// Not asked to. Carry on, on the terminal that started this.
    Foreground,
    /// This is the daemon. Signal when serving.
    Daemon(Ready),
}

/// How this can fail, as the step that failed.
///
/// Named steps rather than one `Errno`, because they are fixed differently: a pid file that is
/// locked means another daemon is running and the fix is to stop it, while a fork that failed
/// means the machine is out of processes.
#[derive(Debug, Clone, Copy)]
pub enum DaemonizeError {
    Pipe(Errno),
    Fork(Errno),
    SetSid(Errno),
    Chdir(Errno),
    DevNull(Errno),
    /// The pid file exists and another process holds its lock.
    PidFileLocked,
    PidFile(Errno),
}

impl core::fmt::Display for DaemonizeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DaemonizeError::Pipe(e) => write!(f, "could not open the readiness pipe: {e}"),
            DaemonizeError::Fork(e) => write!(f, "could not fork: {e}"),
            DaemonizeError::SetSid(e) => write!(f, "could not start a new session: {e}"),
            DaemonizeError::Chdir(e) => write!(f, "could not change to /: {e}"),
            DaemonizeError::DevNull(e) => write!(f, "could not redirect to /dev/null: {e}"),
            DaemonizeError::PidFileLocked => {
                write!(f, "another server holds the pid file; stop it first")
            }
            DaemonizeError::PidFile(e) => write!(f, "could not write the pid file: {e}"),
        }
    }
}

/// `fork`, as the two answers a caller here acts on.
///
/// The child's pid is not carried: both parents in this file exit immediately - the first to wait
/// on the readiness pipe, the second because a session leader must not be the daemon - so neither
/// has anything to do with it. `fork` failing is the third answer and is the `Err`.
enum Forked {
    Parent,
    Child,
}

fn fork() -> Result<Forked, DaemonizeError> {
    match unsafe { libc::fork() } {
        -1 => Err(DaemonizeError::Fork(Errno::current())),
        0 => Ok(Forked::Child),
        _ => Ok(Forked::Parent),
    }
}

/// Fork into the background, with the parent held until the daemon says it is serving.
///
/// The parent does not return: it exits 0 once the byte arrives, or nonzero when the pipe closes
/// first. Only the daemon returns, holding the [`Ready`] it must signal.
///
/// `pid_file` must be a NUL-terminated absolute path, or `None`. Absolute because this changes the
/// working directory to `/`, and NUL-terminated because that is what `open` takes and building the
/// C string here would mean allocating in a function that has just forked.
///
/// # Safety of forking a threaded process
///
/// Only the calling thread survives `fork`, so a caller that has already started threads gets a
/// daemon holding handles to threads that do not exist. `ekod` calls this before `Compiler::start`
/// for exactly that reason, and the comment there says so.
pub fn daemonize(pid_file: Option<&[u8]>) -> Result<Started, DaemonizeError> {
    let mut fds = [0 as libc::c_int; 2];
    // Before the fork, so both sides inherit it. Not `O_CLOEXEC`: nothing here execs, and the
    // point is for it to cross two forks intact.
    checked(unsafe { libc::pipe(fds.as_mut_ptr()) }).map_err(DaemonizeError::Pipe)?;
    let (read_fd, write_fd) = (fds[0], fds[1]);

    match fork()? {
        Forked::Parent => {
            // The parent must not hold a write end open, or the read below can never see EOF and
            // a failed start hangs instead of reporting. This asymmetry is the whole mechanism.
            unsafe { libc::close(write_fd) };
            let mut byte = 0u8;
            let read = unsafe { libc::read(read_fd, core::ptr::from_mut(&mut byte).cast(), 1) };
            unsafe { libc::close(read_fd) };
            crate::exit(if read == 1 { 0 } else { 1 });
        }
        Forked::Child => {
            unsafe { libc::close(read_fd) };
        }
    }

    // A new session, so this process has no controlling terminal and cannot be killed through one.
    checked(unsafe { libc::setsid() }).map_err(DaemonizeError::SetSid)?;

    // The second fork. The first child leads the new session, and a session leader can acquire a
    // controlling terminal by opening one; its child cannot, ever. This is the whole reason the
    // dance has two forks rather than one.
    if let Forked::Parent = fork()? {
        crate::exit(0);
    }

    checked(unsafe { libc::chdir(c"/".as_ptr()) }).map_err(DaemonizeError::Chdir)?;
    // File modes stop depending on whoever started this. Not a checked call: `umask` cannot fail
    // and returns the previous mask rather than a status.
    unsafe { libc::umask(0o027) };

    if let Some(path) = pid_file {
        write_pid_file(path)?;
    }

    redirect_standard_streams()?;

    Ok(Started::Daemon(Ready { write_fd }))
}

/// Claim the pid file, or refuse because someone else holds it.
///
/// `O_EXCL` is not enough on its own: it refuses when the file *exists*, and a daemon that was
/// killed leaves one behind, so a machine that crashed once would never start a daemon again. The
/// lock is what makes the answer "is another process alive and holding this", which is the actual
/// question. A stale file is unlocked and gets truncated and rewritten.
fn write_pid_file(path: &[u8]) -> Result<(), DaemonizeError> {
    let fd = unsafe { libc::open(path.as_ptr().cast(), libc::O_WRONLY | libc::O_CREAT, 0o644) };
    let fd = checked(fd).map_err(DaemonizeError::PidFile)?;

    if unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) } == -1 {
        unsafe { libc::close(fd) };
        return Err(DaemonizeError::PidFileLocked);
    }
    checked(unsafe { libc::ftruncate(fd, 0) }).map_err(DaemonizeError::PidFile)?;

    let pid = unsafe { libc::getpid() };
    let mut buf = [0u8; 24];
    let text = format_pid(pid, &mut buf);
    let wrote = unsafe { libc::write(fd, text.as_ptr().cast(), text.len()) };
    if wrote < 0 {
        return Err(DaemonizeError::PidFile(Errno::current()));
    }

    // **The descriptor stays open on purpose, and is deliberately leaked.** `flock` is released
    // when the last descriptor for the file closes, so closing it here would drop the lock and
    // let a second daemon in. It lives as long as the process, which is exactly the lifetime the
    // lock should have, and the kernel closes it at exit. Not marked cloexec for the same reason
    // this module does not close inherited descriptors.
    Ok(())
}

/// A pid as decimal bytes, without allocating.
fn format_pid(pid: libc::pid_t, buf: &mut [u8; 24]) -> &[u8] {
    let mut n = if pid < 0 { 0u64 } else { pid as u64 };
    let mut i = buf.len();
    if n == 0 {
        i -= 1;
        buf[i] = b'0';
    }
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    buf[buf.len() - 1] = buf[buf.len() - 1];
    &buf[i..]
}

/// Standard in, out and error to `/dev/null`.
///
/// Only those three. The `getrlimit` loop that closes every other descriptor is the one standard
/// step this skips, because the readiness pipe has to survive - and skipping it is stated here
/// rather than merely omitted, so nobody adds it back to be thorough and makes the parent hang.
fn redirect_standard_streams() -> Result<(), DaemonizeError> {
    let devnull = unsafe { libc::open(c"/dev/null".as_ptr(), libc::O_RDWR) };
    let devnull = checked(devnull).map_err(DaemonizeError::DevNull)?;
    for fd in [libc::STDIN_FILENO, libc::STDOUT_FILENO, libc::STDERR_FILENO] {
        checked(unsafe { libc::dup2(devnull, fd) }).map_err(DaemonizeError::DevNull)?;
    }
    if devnull > libc::STDERR_FILENO {
        unsafe { libc::close(devnull) };
    }
    Ok(())
}
