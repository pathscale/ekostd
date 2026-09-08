//! Running another program and collecting what it said.
//!
//! One caller: `rustc_metadata` shells out to a compiler to build an sdylib interface, and the
//! FIXME above that call says it should be a session in this process instead. So this is
//! deliberately the smallest thing that serves it - spawn, wait, collect both streams - and not a
//! general `std::process::Command`. There is no `spawn` returning a live `Child`, no stdin, no
//! environment manipulation, because nothing here wants them and each would be a way to get the
//! fork/exec wrong.

use alloc::vec::Vec;
use alloc::vec;

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
}

impl Pipe {
    fn new() -> Option<Pipe> {
        let mut fds = [0 as libc::c_int; 2];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return None;
        }
        Some(Pipe { read: fds[0], write: fds[1] })
    }

    fn close_write(&mut self) {
        if self.write >= 0 {
            unsafe { libc::close(self.write) };
            self.write = -1;
        }
    }

    fn drain(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let n = unsafe { libc::read(self.read, chunk.as_mut_ptr().cast(), chunk.len()) };
            if n < 0 && crate::Errno::current().0 == libc::EINTR {
                continue;
            }
            if n <= 0 {
                return out;
            }
            out.extend_from_slice(&chunk[..n as usize]);
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

impl Command {
    /// Name a program.
    pub fn new(program: impl AsRef<Path>) -> Command {
        Command { program: program.as_ref().as_bytes().to_vec(), args: Vec::new() }
    }

    /// Add one argument.
    pub fn arg(mut self, arg: impl AsRef<[u8]>) -> Command {
        self.args.push(arg.as_ref().to_vec());
        self
    }

    /// Run it to completion and collect both streams.
    ///
    /// Returns `None` if the spawn itself failed. A program that ran and exited non-zero is an
    /// `Output` with a non-zero `code`, not a `None` - the caller distinguishes those, and
    /// collapsing them would lose the child's stderr, which is the useful part.
    pub fn output(self) -> Option<Output> {
        let mut out_pipe = Pipe::new()?;
        let mut err_pipe = Pipe::new()?;

        // argv[0] is the program name by convention; execvp reads the path from argv[0]'s own
        // argument, not from this slot.
        let prog_c = nul_terminated(&self.program);
        let mut owned: Vec<Vec<u8>> = vec![prog_c.clone()];
        for a in &self.args {
            owned.push(nul_terminated(a));
        }
        let mut argv: Vec<*const libc::c_char> =
            owned.iter().map(|a| a.as_ptr() as *const libc::c_char).collect();
        argv.push(core::ptr::null());

        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return None;
        }
        if pid == 0 {
            // Child. Nothing here may allocate or unwind: after `fork` in a process that may
            // have had threads, only async-signal-safe calls are defined.
            unsafe {
                libc::dup2(out_pipe.write, 1);
                libc::dup2(err_pipe.write, 2);
                libc::close(out_pipe.read);
                libc::close(err_pipe.read);
                libc::close(out_pipe.write);
                libc::close(err_pipe.write);
                libc::execvp(prog_c.as_ptr().cast(), argv.as_ptr());
                // Only reached if exec failed. `_exit`, not `exit`: atexit handlers belong to
                // the parent and running them here would flush its buffers twice.
                libc::_exit(127);
            }
        }

        // Parent. Close the write ends first, or the reads never see EOF.
        out_pipe.close_write();
        err_pipe.close_write();
        let stdout = out_pipe.drain();
        let stderr = err_pipe.drain();

        let mut status: libc::c_int = 0;
        loop {
            let r = unsafe { libc::waitpid(pid, &mut status, 0) };
            if r < 0 && crate::Errno::current().0 == libc::EINTR {
                continue;
            }
            break;
        }
        // WEXITSTATUS, spelled out: libc exposes these as macros that the crate does not.
        let code = if status & 0x7f == 0 { (status >> 8) & 0xff } else { 128 + (status & 0x7f) };
        Some(Output { code, stdout, stderr })
    }
}
