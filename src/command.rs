//! Running another program and collecting what it said.
//!
//! [`Command::output`] runs one to completion. [`Command::spawn`] hands back a [`Child`] for a
//! caller that needs to decide when to stop waiting, because how long is too long is that
//! caller's question and not this crate's.
//!
//! Still no stdin and no working directory: each is a way to get fork and exec wrong and nothing
//! has wanted them. What was added is an environment, and reading both pipes at once with
//! `poll`, which is a fix rather than a feature: draining stdout to end of file and then stderr
//! deadlocks whenever the child fills the stderr buffer first, 16 KB on macOS.

use alloc::vec;
use alloc::vec::Vec;
use core::time::Duration;

use crate::path::Path;

/// What a finished program left behind.
pub struct Output {
    /// The exit status: 0 is success.
    pub code: i32,
    /// Everything it wrote to stdout.
    pub stdout: Vec<u8>,
    /// Everything it wrote to stderr.
    pub stderr: Vec<u8>,
}

impl Output {
    /// Whether it exited zero.
    pub fn success(&self) -> bool {
        self.code == 0
    }
}

/// A program to run.
pub struct Command {
    program: Vec<u8>,
    args: Vec<Vec<u8>>,
    /// Overrides on top of this process's environment, as `KEY=VALUE`.
    overrides: Vec<Vec<u8>>,
}

fn nul_terminated(bytes: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(bytes.len() + 1);
    v.extend_from_slice(bytes);
    v.push(0);
    v
}

/// A pipe, closed on drop so a failure part-way through leaks no descriptors.
struct Pipe {
    read: libc::c_int,
    write: libc::c_int,
    /// Set once the far end is gone and this descriptor has nothing more to say.
    done: bool,
    got: Vec<u8>,
}

impl Pipe {
    fn new() -> Option<Pipe> {
        let mut fds = [0 as libc::c_int; 2];
        // Close-on-exec, or two concurrent callers hang each other: a `fork` copies every
        // descriptor the process has, so another thread's child inherits these write ends and
        // never closes them, and this read never reaches end of file. `dup2` clears the flag on
        // the descriptor it creates, so our own child keeps 0, 1 and 2 across the exec.
        #[cfg(target_os = "linux")]
        let made = unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) };
        #[cfg(not(target_os = "linux"))]
        let made = unsafe { libc::pipe(fds.as_mut_ptr()) };
        if made != 0 {
            return None;
        }
        // macOS has no `pipe2`, so the flag goes on afterwards, leaving a window two syscalls
        // wide rather than the lifetime of a child.
        #[cfg(not(target_os = "linux"))]
        unsafe {
            libc::fcntl(fds[0], libc::F_SETFD, libc::FD_CLOEXEC);
            libc::fcntl(fds[1], libc::F_SETFD, libc::FD_CLOEXEC);
        }
        // Non-blocking, because `poll` says a descriptor is ready and not how much is there.
        // A blocking read after a readable poll can still stall on the second pass round the
        // loop, and one stalled read is the deadlock this module exists to avoid.
        unsafe { libc::fcntl(fds[0], libc::F_SETFL, libc::O_NONBLOCK) };
        Some(Pipe { read: fds[0], write: fds[1], done: false, got: Vec::new() })
    }

    fn close_write(&mut self) {
        if self.write >= 0 {
            unsafe { libc::close(self.write) };
            self.write = -1;
        }
    }

    /// Read whatever is there now. Sets `done` at end of file.
    fn take_ready(&mut self) {
        let mut chunk = [0u8; 8192];
        loop {
            let n = unsafe { libc::read(self.read, chunk.as_mut_ptr().cast(), chunk.len()) };
            if n > 0 {
                self.got.extend_from_slice(&chunk[..n as usize]);
                continue;
            }
            if n == 0 {
                self.done = true;
                return;
            }
            let e = crate::Errno::current().0;
            if e == libc::EINTR {
                continue;
            }
            // Nothing left for now, which is not end of file.
            if e != libc::EAGAIN && e != libc::EWOULDBLOCK {
                self.done = true;
            }
            return;
        }
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        if self.read >= 0 {
            unsafe { libc::close(self.read) };
        }
        if self.write >= 0 {
            unsafe { libc::close(self.write) };
        }
    }
}

/// Where to look for a program named without a slash.
///
/// Built in the parent: `execvp` has no variant taking an environment on macOS, and the
/// alternative is `setenv` in the child, which allocates after a `fork` that may have left
/// another thread's locks held. Candidates computed first make the child's half `execve` alone.
fn candidates(program: &[u8]) -> Vec<Vec<u8>> {
    if program.contains(&b'/') {
        return vec![nul_terminated(program)];
    }
    let path = crate::env::var_os("PATH").unwrap_or_default();
    let mut out = Vec::new();
    for dir in crate::env::split_paths(&path) {
        if dir.is_empty() {
            continue;
        }
        let mut candidate = Vec::with_capacity(dir.len() + program.len() + 2);
        candidate.extend_from_slice(dir);
        if !dir.ends_with(b"/") {
            candidate.push(b'/');
        }
        candidate.extend_from_slice(program);
        out.push(nul_terminated(&candidate));
    }
    out
}

impl Command {
    /// Name a program.
    pub fn new(program: impl AsRef<Path>) -> Command {
        Command {
            program: program.as_ref().as_bytes().to_vec(),
            args: Vec::new(),
            overrides: Vec::new(),
        }
    }

    /// Add one argument.
    pub fn arg(mut self, arg: impl AsRef<[u8]>) -> Command {
        self.args.push(arg.as_ref().to_vec());
        self
    }

    /// Add several arguments, in order.
    pub fn args<I, A>(mut self, args: I) -> Command
    where
        I: IntoIterator<Item = A>,
        A: AsRef<[u8]>,
    {
        for arg in args {
            self.args.push(arg.as_ref().to_vec());
        }
        self
    }

    /// Set a variable in the child's environment, on top of this process's own.
    ///
    /// Inherit-and-override rather than replace, because the callers that need this are setting
    /// one or two things for a toolchain that reads a dozen more. Setting the same key twice
    /// keeps the last, as a shell would.
    pub fn env(mut self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Command {
        let mut entry = key.as_ref().to_vec();
        entry.push(b'=');
        entry.extend_from_slice(value.as_ref());
        self.overrides.push(entry);
        self
    }

    /// Start it, with both output pipes open, and hand back the running child.
    ///
    /// `None` only if the fork or a pipe failed. A program that cannot be found is not that
    /// case: the child starts, every candidate `execve` fails, and it exits 127.
    pub fn spawn(self) -> Option<Child> {
        let mut out = Pipe::new()?;
        let mut err = Pipe::new()?;

        let tries = candidates(&self.program);
        if tries.is_empty() {
            return None;
        }
        // argv[0] is the program name by convention.
        let mut owned: Vec<Vec<u8>> = vec![nul_terminated(&self.program)];
        for a in &self.args {
            owned.push(nul_terminated(a));
        }
        let mut argv: Vec<*const libc::c_char> =
            owned.iter().map(|a| a.as_ptr() as *const libc::c_char).collect();
        argv.push(core::ptr::null());

        let envp_owned = self.environment();
        let mut envp: Vec<*const libc::c_char> =
            envp_owned.iter().map(|e| e.as_ptr() as *const libc::c_char).collect();
        envp.push(core::ptr::null());

        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return None;
        }
        if pid == 0 {
            // Child. Nothing here may allocate or unwind: after `fork` in a process that may
            // have had threads, only async-signal-safe calls are defined. Every buffer this
            // touches was built above, before the fork.
            unsafe {
                // Its own process group, so that killing it kills what it started. A compiler
                // driver spawns real work and a signal sent to the driver alone orphans that
                // work, which keeps running against a deadline nobody is watching any more.
                libc::setpgid(0, 0);

                // **Move the pipe ends clear of 0, 1 and 2 before anything is dup2'd onto
                // those.** Otherwise the shuffle eats itself, in two ways that need only the
                // parent to have closed its own stdout, which is what a daemon does: if
                // `out.write` already is 1 then `dup2(1, 1)` is a no-op and closing the
                // original afterwards closes the stdout just set up, and if the two ends come
                // back crossed, `out.write` 2 and `err.write` 1, then the second `dup2` copies
                // a descriptor the first one has already replaced.
                let out_w = above_stdio(out.write);
                let err_w = above_stdio(err.write);
                if out_w < 0 || err_w < 0 {
                    libc::_exit(127);
                }

                // No stdin, so a program that stops to ask a question does not wait forever on
                // an answer nobody will see.
                let null = libc::open(c_dev_null().as_ptr().cast(), libc::O_RDONLY);
                if null < 0 || libc::dup2(null, 0) < 0 {
                    libc::_exit(127);
                }
                libc::close(null);

                // Checked, because a child that execs with the wrong stdout reports its work to
                // nowhere and the parent waits on a pipe nothing will ever write to.
                if libc::dup2(out_w, 1) < 0 || libc::dup2(err_w, 2) < 0 {
                    libc::_exit(127);
                }

                // Nothing is closed by hand. Every descriptor this child holds carries
                // close-on-exec, so the four pipe ends and the two relocations go by
                // themselves, and `dup2` clears the flag on what it creates, so 1 and 2 are
                // exactly what survives.
                for try_path in &tries {
                    libc::execve(try_path.as_ptr().cast(), argv.as_ptr(), envp.as_ptr());
                }
                // Only reached if every candidate failed. `_exit`, not `exit`: atexit handlers
                // belong to the parent and running them here would flush its buffers twice.
                libc::_exit(127);
            }
        }

        // Parent. Close the write ends first, or the reads never see EOF.
        out.close_write();
        err.close_write();
        Some(Child { pid, out, err, status: None })
    }

    /// Run it to completion and collect both streams.
    ///
    /// Returns `None` if the spawn itself failed. A program that ran and exited non-zero is an
    /// `Output` with a non-zero `code`, not a `None` - the caller distinguishes those, and
    /// collapsing them would lose the child's stderr, which is the useful part.
    ///
    /// Unbounded. A caller that needs a deadline spawns and keeps its own.
    pub fn output(self) -> Option<Output> {
        let mut child = self.spawn()?;
        while !child.drained() {
            child.read_ready(None);
        }
        let code = child.wait();
        Some(child.into_output(code))
    }

    fn environment(&self) -> Vec<Vec<u8>> {
        let mut entries = crate::env::environ();
        for entry in &self.overrides {
            // The key including its `=`, so `CARGO` does not match `CARGO_TARGET_DIR`.
            let key_end = entry.iter().position(|b| *b == b'=').map_or(entry.len(), |i| i + 1);
            let key = &entry[..key_end];
            entries.retain(|e| !e.starts_with(key));
            entries.push(entry.clone());
        }
        entries.iter().map(|e| nul_terminated(e)).collect()
    }
}

/// A running child and its two pipes.
///
/// The parts a caller needs to impose its own policy: read what is there, ask whether it has
/// exited, kill it, wait for it. A deadline is one such policy and is not one of these, because
/// what counts as too long is the caller's question and not this crate's.
pub struct Child {
    pid: libc::pid_t,
    out: Pipe,
    err: Pipe,
    /// Kept once it has been waited on, because `waitpid` answers only once.
    status: Option<i32>,
}

impl Child {
    /// Whether both pipes have reached end of file.
    ///
    /// **Not the same as having exited.** A program can close its output and keep running, which
    /// is why a caller with a deadline still has to watch the clock after this turns true.
    pub fn drained(&self) -> bool {
        self.out.done && self.err.done
    }

    /// Wait until either pipe has something or `timeout` passes, and read whatever arrived.
    ///
    /// **Both at once, which is the whole point.** Reading one to end of file and then the other
    /// deadlocks whenever the child fills the second buffer first.
    pub fn read_ready(&mut self, timeout: Option<Duration>) {
        // **Only the pipes still open.** A descriptor at end of file reports `POLLHUP` every
        // time it is polled, so leaving a finished one in the set makes this return instantly
        // and the caller spin at full speed until the other one closes. A compiler that writes
        // nothing to stdout and takes five minutes over stderr is that case exactly.
        let mut fds = [libc::pollfd { fd: 0, events: libc::POLLIN, revents: 0 }; 2];
        let mut watched: [bool; 2] = [false, false];
        let mut n = 0;
        if !self.out.done {
            fds[n].fd = self.out.read;
            watched[0] = true;
            n += 1;
        }
        if !self.err.done {
            fds[n].fd = self.err.read;
            watched[1] = true;
            n += 1;
        }
        if n == 0 {
            return;
        }

        // Rounded up, never down: `as_millis` of anything under a millisecond is zero, and a
        // zero timeout is a busy wait rather than a short one.
        let ms = timeout.map_or(-1, |t| {
            let whole = t.as_millis().min(i32::MAX as u128) as libc::c_int;
            if whole == 0 && !t.is_zero() {
                1
            } else {
                whole
            }
        });
        let ready = unsafe { libc::poll(fds.as_mut_ptr(), n as libc::nfds_t, ms) };
        if ready == 0 {
            return;
        }
        if ready < 0 {
            if crate::Errno::current().0 == libc::EINTR {
                return;
            }
            // **Say the pipes are finished rather than leaving the caller to spin.** Nothing
            // will become readable after a `poll` that failed for a reason other than a signal,
            // and a caller looping on `drained` would never stop.
            self.out.done = true;
            self.err.done = true;
            return;
        }
        let mut slot = 0;
        if watched[0] {
            if fds[slot].revents != 0 {
                self.out.take_ready();
            }
            slot += 1;
        }
        if watched[1] && fds[slot].revents != 0 {
            self.err.take_ready();
        }
    }

    /// The exit code if it has exited, `None` while it is still running.
    pub fn try_wait(&mut self) -> Option<i32> {
        self.collect(libc::WNOHANG)
    }

    /// Block until it exits.
    pub fn wait(&mut self) -> i32 {
        // `waitpid` with no flags does not return until the child has exited, so the `None` arm
        // is unreachable here and the fallback is only there so this returns a number.
        self.collect(0).unwrap_or(-1)
    }

    /// `waitpid`, with the exit status kept once it arrives.
    ///
    /// **Kept because it can only be asked for once.** A reaped child is gone from the process
    /// table, and a second `waitpid` on its pid fails with `ECHILD` rather than repeating the
    /// answer, so a caller that asked `try_wait` and then `wait` would get a real code and then
    /// a fabricated one.
    ///
    /// `None` means still running, which only `WNOHANG` can produce.
    fn collect(&mut self, flags: libc::c_int) -> Option<i32> {
        if let Some(code) = self.status {
            return Some(code);
        }
        let mut status: libc::c_int = 0;
        loop {
            let r = unsafe { libc::waitpid(self.pid, &mut status, flags) };
            if r == 0 {
                return None;
            }
            if r < 0 && crate::Errno::current().0 == libc::EINTR {
                continue;
            }
            // A negative return here is `ECHILD` or a bad pid: there is no status to report and
            // nothing further to wait for, so -1 is recorded and the caller stops asking.
            let code = if r < 0 { -1 } else { exit_code(status) };
            self.status = Some(code);
            return Some(code);
        }
    }

    /// `SIGTERM` to the child **and everything it started**: asks it to stop.
    ///
    /// **The half to try first.** A build tool that receives this removes its lock file and its
    /// half-written output; the same tool receiving `SIGKILL` cannot, and the next run finds a
    /// directory it has to recover before it can do anything. A program is free to ignore
    /// `SIGTERM` or to be wedged past noticing it, which is why [`Child::kill`] exists and why a
    /// caller that has asked should be prepared to insist.
    pub fn terminate(&self) {
        self.signal(libc::SIGTERM);
    }

    /// `SIGKILL` to the child **and everything it started**, which cannot be caught.
    pub fn kill(&self) {
        self.signal(libc::SIGKILL);
    }

    /// Signal the child's process group, and the child itself in case it has none.
    ///
    /// The negative pid is the group, which the child was put in its own copy of at spawn.
    /// Signalling the child alone leaves its own children running with nobody waiting on them,
    /// and `cargo` is exactly that shape: signal the driver and the `rustc` processes carry on
    /// compiling. The second call covers a `setpgid` that failed.
    fn signal(&self, sig: libc::c_int) {
        unsafe {
            libc::kill(-self.pid, sig);
            libc::kill(self.pid, sig);
        }
    }

    /// What has arrived on stdout so far, for a caller that needs it before the child is done.
    pub fn stdout_so_far(&self) -> &[u8] {
        &self.out.got
    }

    /// What it said, with the code the caller got from `wait`.
    pub fn into_output(self, code: i32) -> Output {
        Output { code, stdout: self.out.got.clone(), stderr: self.err.got.clone() }
    }
}

/// `WEXITSTATUS` and `WTERMSIG`, spelled out: libc exposes these as macros the crate does not.
fn exit_code(status: libc::c_int) -> i32 {
    if status & 0x7f == 0 {
        (status >> 8) & 0xff
    } else {
        128 + (status & 0x7f)
    }
}

/// A copy of `fd` above the three standard descriptors, or `fd` if it is already clear of them.
///
/// `F_DUPFD_CLOEXEC` rather than `F_DUPFD`, because the copy has to carry close-on-exec like the
/// original: this runs in a child that relies on the flag to close everything it is not handing
/// to the program it is about to become.
///
/// Returns negative if the duplication failed, which the caller treats as fatal.
unsafe fn above_stdio(fd: libc::c_int) -> libc::c_int {
    if fd > 2 {
        return fd;
    }
    unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 3) }
}

fn c_dev_null() -> [u8; 10] {
    *b"/dev/null\0"
}
