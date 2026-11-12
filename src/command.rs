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

use alloc::vec::Vec;
use core::time::Duration;

use crate::libc_lite as libc;
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
    args_buf: Vec<u8>,
    args_offsets: Vec<usize>,
    /// Overrides on top of this process's environment, as NUL-terminated `KEY=VALUE\0`.
    overrides_buf: Vec<u8>,
    overrides_offsets: Vec<usize>,
    overrides_key_lens: Vec<usize>,
}

fn push_nul_entry(buf: &mut Vec<u8>, offsets: &mut Vec<usize>, bytes: &[u8]) {
    offsets.push(buf.len());
    buf.extend_from_slice(bytes);
    buf.push(0);
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

    fn step_take_ready(&mut self) -> bool {
        let mut chunk = [0u8; 8192];
        let n = unsafe { libc::read(self.read, chunk.as_mut_ptr().cast(), chunk.len()) };
        if n > 0 {
            self.got.extend_from_slice(&chunk[..n as usize]);
            return true;
        }
        if n == 0 {
            self.done = true;
            return false;
        }
        let e = crate::Errno::current().0;
        if e == libc::EINTR {
            return true;
        }
        // Nothing left for now, which is not end of file.
        if e != libc::EAGAIN && e != libc::EWOULDBLOCK {
            self.done = true;
        }
        false
    }

    /// Read whatever is there now. Sets `done` at end of file.
    fn take_ready(&mut self) {
        loop {
            if !self.step_take_ready() {
                break;
            }
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
fn has_slash(bytes: &[u8]) -> bool {
    let len = bytes.len();
    let mut found = false;
    let mut i = 0;
    while i < len {
        if bytes[i] == b'/' {
            found = true;
        }
        i += 1;
    }
    found
}

fn find_colon(bytes: &[u8], start: usize) -> usize {
    let len = bytes.len();
    let mut found = len;
    let mut i = start;
    while i < len {
        if found == len && bytes[i] == b':' {
            found = i;
        }
        i += 1;
    }
    found
}

fn step_path_candidate(
    path: &[u8],
    pos: usize,
    program: &[u8],
    buf: &mut Vec<u8>,
    offsets: &mut Vec<usize>,
) -> usize {
    let end = find_colon(path, pos);
    let dir = &path[pos..end];
    if !dir.is_empty() {
        offsets.push(buf.len());
        buf.extend_from_slice(dir);
        if dir[dir.len() - 1] != b'/' {
            buf.push(b'/');
        }
        buf.extend_from_slice(program);
        buf.push(0);
    }
    if end < path.len() {
        end + 1
    } else {
        path.len()
    }
}

fn candidates(program: &[u8], buf: &mut Vec<u8>, offsets: &mut Vec<usize>) {
    if has_slash(program) {
        push_nul_entry(buf, offsets, program);
        return;
    }
    if let Some(path) = crate::env::var_os("PATH") {
        let len = path.len();
        let mut pos = 0usize;
        while pos < len {
            pos = step_path_candidate(&path, pos, program, buf, offsets);
        }
    }
}

fn build_ptr_table(buf: &[u8], offsets: &[usize], table: &mut Vec<usize>) {
    let count = offsets.len();
    let base = buf.as_ptr() as usize;
    let mut i = 0usize;
    while i < count {
        table.push(base + offsets[i]);
        i += 1;
    }
    table.push(0);
}

unsafe fn exec_one_candidate(tries_buf: &[u8], offset: usize, argv: &[usize], envp: &[usize]) {
    let try_ptr: *const libc::c_char = unsafe { tries_buf.as_ptr().add(offset).cast() };
    unsafe {
        libc::execve(try_ptr, argv.as_ptr().cast(), envp.as_ptr().cast());
    }
}

unsafe fn exec_child(
    out_write: libc::c_int,
    err_write: libc::c_int,
    tries_buf: &[u8],
    tries_offsets: &[usize],
    argv: &[usize],
    envp: &[usize],
) -> libc::c_int {
    unsafe {
        // Its own process group, so that killing it kills what it started. A compiler
        // driver spawns real work and a signal sent to the driver alone orphans that
        // work, which keeps running against a deadline nobody is watching any more.
        libc::setpgid(0, 0);

        // **Move the pipe ends clear of 0, 1 and 2 before anything is dup2'd onto
        // those.**
        let out_w = above_stdio(out_write);
        if out_w < 0 {
            return 127;
        }
        let err_w = above_stdio(err_write);
        if err_w < 0 {
            return 127;
        }

        // No stdin, so a program that stops to ask a question does not wait forever on
        // an answer nobody will see.
        let null = libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY);
        if null < 0 {
            return 127;
        }
        let dup_in = libc::dup2(null, 0);
        libc::close(null);
        if dup_in < 0 {
            return 127;
        }

        // Checked, because a child that execs with the wrong stdout reports its work to
        // nowhere and the parent waits on a pipe nothing will ever write to.
        if libc::dup2(out_w, 1) < 0 {
            return 127;
        }
        if libc::dup2(err_w, 2) < 0 {
            return 127;
        }

        let count = tries_offsets.len();
        let mut i = 0usize;
        while i < count {
            exec_one_candidate(tries_buf, tries_offsets[i], argv, envp);
            i += 1;
        }
        127
    }
}

impl Command {
    /// Name a program.
    pub fn new(program: impl AsRef<Path>) -> Command {
        Command {
            program: program.as_ref().as_bytes().to_vec(),
            args_buf: Vec::new(),
            args_offsets: Vec::new(),
            overrides_buf: Vec::new(),
            overrides_offsets: Vec::new(),
            overrides_key_lens: Vec::new(),
        }
    }

    /// Add one argument.
    pub fn arg(mut self, arg: impl AsRef<[u8]>) -> Command {
        push_nul_entry(&mut self.args_buf, &mut self.args_offsets, arg.as_ref());
        self
    }

    /// Add several arguments, in order.
    pub fn args<I, A>(mut self, args: I) -> Command
    where
        I: IntoIterator<Item = A>,
        A: AsRef<[u8]>,
    {
        for arg in args {
            push_nul_entry(&mut self.args_buf, &mut self.args_offsets, arg.as_ref());
        }
        self
    }

    /// Set a variable in the child's environment, on top of this process's own.
    ///
    /// Inherit-and-override rather than replace, because the callers that need this are setting
    /// one or two things for a toolchain that reads a dozen more. Setting the same key twice
    /// keeps the last, as a shell would.
    pub fn env(mut self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Command {
        let key_bytes = key.as_ref();
        self.overrides_offsets.push(self.overrides_buf.len());
        self.overrides_key_lens.push(key_bytes.len() + 1);
        self.overrides_buf.extend_from_slice(key_bytes);
        self.overrides_buf.push(b'=');
        self.overrides_buf.extend_from_slice(value.as_ref());
        self.overrides_buf.push(0);
        self
    }

    fn build_argv_buf(&self, buf: &mut Vec<u8>, offsets: &mut Vec<usize>) {
        push_nul_entry(buf, offsets, &self.program);
        let base = buf.len();
        buf.extend_from_slice(&self.args_buf);
        let count = self.args_offsets.len();
        let mut i = 0usize;
        while i < count {
            offsets.push(base + self.args_offsets[i]);
            i += 1;
        }
    }

    /// Start it, with both output pipes open, and hand back the running child.
    ///
    /// `None` only if the fork or a pipe failed. A program that cannot be found is not that
    /// case: the child starts, every candidate `execve` fails, and it exits 127.
    pub fn spawn(self) -> Option<Child> {
        let mut out = Pipe::new()?;
        let mut err = Pipe::new()?;

        let mut tries_buf = Vec::new();
        let mut tries_offsets = Vec::new();
        candidates(&self.program, &mut tries_buf, &mut tries_offsets);
        if tries_offsets.is_empty() {
            return None;
        }
        // argv[0] is the program name by convention.
        let mut argv_buf = Vec::new();
        let mut argv_offsets = Vec::new();
        self.build_argv_buf(&mut argv_buf, &mut argv_offsets);
        let mut argv = Vec::new();
        build_ptr_table(&argv_buf, &argv_offsets, &mut argv);

        let mut envp_buf = Vec::new();
        let mut envp_offsets = Vec::new();
        self.environment(&mut envp_buf, &mut envp_offsets);
        let mut envp = Vec::new();
        build_ptr_table(&envp_buf, &envp_offsets, &mut envp);

        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return None;
        }
        if pid == 0 {
            // Child. Nothing here may allocate or unwind: after `fork` in a process that may
            // have had threads, only async-signal-safe calls are defined. Every buffer this
            // touches was built above, before the fork.
            unsafe {
                let code =
                    exec_child(out.write, err.write, &tries_buf, &tries_offsets, &argv, &envp);
                libc::_exit(code);
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

    fn key_matches_at(&self, entry: &[u8], idx: usize) -> bool {
        let start = self.overrides_offsets[idx];
        let key_len = self.overrides_key_lens[idx];
        let key = &self.overrides_buf[start..start + key_len];
        entry.len() >= key_len && crate::path::bytes_equal(&entry[..key_len], key)
    }

    fn is_key_overridden(&self, entry: &[u8], from_idx: usize) -> bool {
        let count = self.overrides_offsets.len();
        let mut matched = false;
        let mut idx = from_idx;
        while idx < count {
            if self.key_matches_at(entry, idx) {
                matched = true;
            }
            idx += 1;
        }
        matched
    }

    unsafe fn step_base_env(
        &self,
        envp: *const *const libc::c_char,
        index: usize,
        buf: &mut Vec<u8>,
        offsets: &mut Vec<usize>,
    ) -> bool {
        let p = unsafe { crate::env::cstr_table_at(envp, index) };
        if p.is_null() {
            return false;
        }
        let existing = unsafe { crate::env::cstr_to_vec(p) };
        if !self.is_key_overridden(&existing, 0) {
            push_nul_entry(buf, offsets, &existing);
        }
        true
    }

    fn append_override_if_active(&self, idx: usize, buf: &mut Vec<u8>, offsets: &mut Vec<usize>) {
        let start = self.overrides_offsets[idx];
        let end = if idx + 1 < self.overrides_offsets.len() {
            self.overrides_offsets[idx + 1]
        } else {
            self.overrides_buf.len()
        };
        let entry_with_nul = &self.overrides_buf[start..end];
        if !self.is_key_overridden(entry_with_nul, idx + 1) {
            offsets.push(buf.len());
            buf.extend_from_slice(entry_with_nul);
        }
    }

    fn environment(&self, buf: &mut Vec<u8>, offsets: &mut Vec<usize>) {
        let envp = unsafe { crate::env::raw_environ_ptr() };
        if !envp.is_null() {
            let mut i = 0usize;
            loop {
                if !unsafe { self.step_base_env(envp, i, buf, offsets) } {
                    break;
                }
                i += 1;
            }
        }
        let count = self.overrides_offsets.len();
        let mut idx = 0usize;
        while idx < count {
            self.append_override_if_active(idx, buf, offsets);
            idx += 1;
        }
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
        let ms = match timeout {
            None => -1,
            Some(t) => {
                let total_ms =
                    t.as_secs().saturating_mul(1000).saturating_add(t.subsec_millis() as u64);
                let whole =
                    if total_ms > i32::MAX as u64 { i32::MAX } else { total_ms as libc::c_int };
                if whole == 0 && !t.is_zero() {
                    1
                } else {
                    whole
                }
            }
        };
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
    pub fn into_output(&self, code: i32) -> Output {
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
        return fd.wrapping_add(0);
    }
    unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 3) }
}
