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

use crate::Errno;

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
        loop {
            let wrote = unsafe { libc::write(self.write_fd, core::ptr::from_ref(&byte).cast(), 1) };
            if wrote == 1 || wrote == 0 || Errno::current().0 != libc::EINTR {
                break;
            }
        }
        unsafe { libc::close(self.write_fd) };
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

impl DaemonizeError {
    /// Render this error without going through `core::fmt::write`.
    pub fn message(&self) -> alloc::string::String {
        let mut out = alloc::string::String::new();
        let mut buf = [0u8; 128];
        match self {
            DaemonizeError::Pipe(e) => {
                out.push_str("could not open the readiness pipe: ");
                out.push_str(e.message(&mut buf));
            }
            DaemonizeError::Fork(e) => {
                out.push_str("could not fork: ");
                out.push_str(e.message(&mut buf));
            }
            DaemonizeError::SetSid(e) => {
                out.push_str("could not start a new session: ");
                out.push_str(e.message(&mut buf));
            }
            DaemonizeError::Chdir(e) => {
                out.push_str("could not change to /: ");
                out.push_str(e.message(&mut buf));
            }
            DaemonizeError::DevNull(e) => {
                out.push_str("could not redirect to /dev/null: ");
                out.push_str(e.message(&mut buf));
            }
            DaemonizeError::PidFileLocked => {
                out.push_str("another server holds the pid file; stop it first");
            }
            DaemonizeError::PidFile(e) => {
                out.push_str("could not write the pid file: ");
                out.push_str(e.message(&mut buf));
            }
        }
        out
    }
}

impl core::fmt::Display for DaemonizeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DaemonizeError::Pipe(e) => {
                f.write_str("could not open the readiness pipe: ")?;
                core::fmt::Display::fmt(e, f)
            }
            DaemonizeError::Fork(e) => {
                f.write_str("could not fork: ")?;
                core::fmt::Display::fmt(e, f)
            }
            DaemonizeError::SetSid(e) => {
                f.write_str("could not start a new session: ")?;
                core::fmt::Display::fmt(e, f)
            }
            DaemonizeError::Chdir(e) => {
                f.write_str("could not change to /: ")?;
                core::fmt::Display::fmt(e, f)
            }
            DaemonizeError::DevNull(e) => {
                f.write_str("could not redirect to /dev/null: ")?;
                core::fmt::Display::fmt(e, f)
            }
            DaemonizeError::PidFileLocked => {
                f.write_str("another server holds the pid file; stop it first")
            }
            DaemonizeError::PidFile(e) => {
                f.write_str("could not write the pid file: ")?;
                core::fmt::Display::fmt(e, f)
            }
        }
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
fn wait_parent_ready(read_fd: libc::c_int, ready: Ready) -> ! {
    // The parent must not hold a write end open, or the read below can never see EOF and
    // a failed start hangs instead of reporting. This asymmetry is the whole mechanism.
    drop(ready);
    let mut byte = 0u8;
    let read = loop {
        let read = unsafe { libc::read(read_fd, core::ptr::from_mut(&mut byte).cast(), 1) };
        if read >= 0 || Errno::current().0 != libc::EINTR {
            break read;
        }
    };
    unsafe { libc::close(read_fd) };
    crate::exit(if read == 1 { 0 } else { 1 });
}

pub fn daemonize(pid_file: Option<&[u8]>) -> Result<Started, DaemonizeError> {
    let mut fds = [0 as libc::c_int; 2];
    // Before the fork, so both sides inherit it. Not `O_CLOEXEC`: nothing here execs, and the
    // point is for it to cross two forks intact.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } == -1 {
        return Err(DaemonizeError::Pipe(Errno::current()));
    }
    let (read_fd, write_fd) = (fds[0], fds[1]);
    let ready = Ready { write_fd };

    let first_fork = unsafe { libc::fork() };
    if first_fork == -1 {
        let e = Errno::current();
        unsafe { libc::close(read_fd) };
        return Err(DaemonizeError::Fork(e));
    }
    if first_fork != 0 {
        wait_parent_ready(read_fd, ready);
    }
    unsafe { libc::close(read_fd) };

    // A new session, so this process has no controlling terminal and cannot be killed through one.
    if unsafe { libc::setsid() } == -1 {
        return Err(DaemonizeError::SetSid(Errno::current()));
    }

    // The second fork. The first child leads the new session, and a session leader can acquire a
    // controlling terminal by opening one; its child cannot, ever. This is the whole reason the
    // dance has two forks rather than one.
    let second_fork = unsafe { libc::fork() };
    if second_fork == -1 {
        return Err(DaemonizeError::Fork(Errno::current()));
    }
    if second_fork != 0 {
        crate::exit(0);
    }

    if unsafe { libc::chdir(c"/".as_ptr()) } == -1 {
        return Err(DaemonizeError::Chdir(Errno::current()));
    }
    // File modes stop depending on whoever started this. Not a checked call: `umask` cannot fail
    // and returns the previous mask rather than a status.
    unsafe { libc::umask(0o027) };

    if let Some(path) = pid_file {
        let pid_rc = write_pid_file_raw(path);
        if pid_rc == -1 {
            return Err(DaemonizeError::PidFileLocked);
        }
        if pid_rc != 0 {
            return Err(DaemonizeError::PidFile(Errno(pid_rc)));
        }
    }

    let devnull_rc = redirect_standard_streams_raw();
    if devnull_rc != 0 {
        return Err(DaemonizeError::DevNull(Errno(devnull_rc)));
    }

    Ok(Started::Daemon(ready))
}

/// Claim the pid file, or refuse because someone else holds it.
///
/// Returns `0` on success, `-1` if locked by another process, or a positive errno code.
fn write_pid_file_raw(path: &[u8]) -> i32 {
    let fd = unsafe {
        libc::open(
            path.as_ptr().cast(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_CLOEXEC,
            0o644,
        )
    };
    if fd == -1 {
        return Errno::current().0.max(1);
    }

    if unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) } == -1 {
        unsafe { libc::close(fd) };
        return -1;
    }
    if unsafe { libc::ftruncate(fd, 0) } == -1 {
        let error = Errno::current().0.max(1);
        unsafe { libc::close(fd) };
        return error;
    }

    let pid = unsafe { libc::getpid() };
    let mut buf = [0u8; 24];
    let text = format_pid(pid, &mut buf);
    let mut offset = 0;
    while offset < text.len() {
        let wrote = unsafe { libc::write(fd, text[offset..].as_ptr().cast(), text.len() - offset) };
        if wrote > 0 {
            offset += wrote as usize;
            continue;
        }
        let error = if wrote == 0 { libc::EIO } else { Errno::current().0.max(1) };
        if wrote < 0 && error == libc::EINTR {
            continue;
        }
        unsafe { libc::close(fd) };
        return error;
    }

    // **The descriptor stays open on purpose, and is deliberately leaked.**
    0
}

/// A pid as decimal bytes followed by a newline, without allocating.
pub fn format_pid(pid: libc::pid_t, buf: &mut [u8; 24]) -> &[u8] {
    let mut n = if pid < 0 { 0u64 } else { pid as u64 };
    let mut i = buf.len() - 1;
    buf[i] = b'\n';
    if n == 0 {
        i -= 1;
        buf[i] = b'0';
    }
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }

    &buf[i..]
}

/// Standard in, out and error to `/dev/null`.
///
/// Only those three. The `getrlimit` loop that closes every other descriptor is the one standard
/// step this skips, because the readiness pipe has to survive - and skipping it is stated here
/// rather than merely omitted, so nobody adds it back to be thorough and makes the parent hang.
fn redirect_standard_streams_raw() -> i32 {
    let devnull = unsafe { libc::open(c"/dev/null".as_ptr(), libc::O_RDWR) };
    if devnull == -1 {
        return Errno::current().0.max(1);
    }
    if unsafe { libc::dup2(devnull, libc::STDIN_FILENO) } == -1 {
        let e = Errno::current().0.max(1);
        if devnull > libc::STDERR_FILENO {
            unsafe { libc::close(devnull) };
        }
        return e;
    }
    if unsafe { libc::dup2(devnull, libc::STDOUT_FILENO) } == -1 {
        let e = Errno::current().0.max(1);
        if devnull > libc::STDERR_FILENO {
            unsafe { libc::close(devnull) };
        }
        return e;
    }
    if unsafe { libc::dup2(devnull, libc::STDERR_FILENO) } == -1 {
        let e = Errno::current().0.max(1);
        if devnull > libc::STDERR_FILENO {
            unsafe { libc::close(devnull) };
        }
        return e;
    }
    if devnull > libc::STDERR_FILENO {
        unsafe { libc::close(devnull) };
    }
    0
}

#[cfg(test)]
mod pid_lock_tests {
    extern crate std;
    use super::*;
    use std::os::unix::fs::MetadataExt;
    #[test]
    fn retained_pid_lock_closes_on_exec() {
        let path = std::env::temp_dir().join(std::format!("eko-pid-cloexec-{}", std::process::id()));
        let cpath = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(write_pid_file_raw(cpath.as_bytes_with_nul()), 0);
        let meta = std::fs::metadata(&path).unwrap();
        let mut found = None;
        for entry in std::fs::read_dir("/dev/fd").unwrap() {
            let entry = entry.unwrap();
            let Ok(fd) = entry.file_name().to_string_lossy().parse::<i32>() else { continue };
            let mut stat = core::mem::MaybeUninit::<libc::stat>::uninit();
            if unsafe { libc::fstat(fd, stat.as_mut_ptr()) } == 0 {
                let stat = unsafe { stat.assume_init() };
                if stat.st_ino as u64 == meta.ino() && stat.st_dev as u64 == meta.dev() {
                    found = Some(fd);
                    break;
                }
            }
        }
        let fd = found.expect("retained PID descriptor");
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        unsafe { libc::close(fd); }
        std::fs::remove_file(path).unwrap();
        assert!(flags >= 0 && flags & libc::FD_CLOEXEC != 0);
    }
}
