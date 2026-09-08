//! Unix domain sockets, and reading lines off one.
//!
//! # What this replaces
//!
//! `std::os::unix::net::{UnixListener, UnixStream}` and `std::io::{BufReader, BufWriter}`. The
//! socket calls are a direct mapping and need no explanation; the line reading does, because
//! `BufReader::lines` is the single convenience that kept this crate on `std` and it is not
//! difficult, only fiddly.
//!
//! # Why the buffer is here and not at each caller
//!
//! A line is not a unit the kernel knows about. `read` returns whatever arrived, which may be
//! half a line, three lines, or a line split across two calls - so something has to hold the
//! remainder between reads. `std::io::BufReader` is that something, and writing it once here is
//! the alternative to `client.rs` and `server.rs` each growing their own and drifting.
//!
//! # The end of a stream is not an error
//!
//! [`Lines::next_line`] returns `Ok(None)` when the peer closed cleanly with nothing buffered,
//! and that is the only case where a short read is not a problem. A caller that has been promised
//! `n` lines and gets `None` at line three knows the stream was truncated; a caller reading until
//! the peer stops knows it stopped. Collapsing those into one error is what `std::io::Error` did
//! here, and `protocol.rs`'s own header argues against it: "a reply that ends early is a
//! truncated stream; a reply that never ends is a hung server."

// `daemon` is `#![no_std]`. These arrive with the standard prelude and have no path to
// match, which is why a `std::` grep cannot see them and the attribute has to be flipped
// to find them at all.
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;


use crate::{Errno, checked};

/// An open file descriptor, closed when dropped.
///
/// Every socket here is one of these, so "was it closed" is a property of ownership rather than
/// of remembering. A descriptor is duplicated with [`Fd::duplicate`] rather than copied, because
/// two owners closing one number is how a live connection gets torn down under someone.
#[derive(Debug)]
pub struct Fd(libc::c_int);

impl Fd {
    /// The raw number, for a call that needs it. Does not transfer ownership.
    pub fn raw(&self) -> libc::c_int {
        self.0
    }

    /// A second descriptor for the same open file, with its own lifetime.
    ///
    /// `std::os::unix::net::UnixStream::try_clone` in one call. The reader and the writer of one
    /// connection each own one, which is what lets them be held separately without either
    /// closing the other's.
    pub fn duplicate(&self) -> Result<Fd, Errno> {
        checked(unsafe { libc::dup(self.0) }).map(Fd)
    }
}

impl Drop for Fd {
    fn drop(&mut self) {
        unsafe { libc::close(self.0) };
    }
}

/// Fill a `sockaddr_un` from a path, or say it will not fit.
///
/// **The length limit is real and is not ours.** `sun_path` is 104 bytes on this platform and
/// 108 on Linux, and a path that does not fit is silently truncated by `bind` - which binds a
/// *different* socket than the caller named, and the failure surfaces later as a client that
/// cannot connect. Refusing here is the difference between a clear error and a mystery.
fn address(path: &[u8]) -> Result<libc::sockaddr_un, Errno> {
    let mut addr: libc::sockaddr_un = unsafe { core::mem::zeroed() };
    addr.sun_family = libc::AF_UNIX as _;
    // One byte reserved for the NUL, which `bind` needs and the path does not carry.
    if path.len() >= addr.sun_path.len() {
        return Err(Errno(libc::ENAMETOOLONG));
    }
    for (slot, byte) in addr.sun_path.iter_mut().zip(path.iter()) {
        *slot = *byte as _;
    }
    Ok(addr)
}

impl Errno {
    /// Would this call have blocked?
    ///
    /// Two numbers, because POSIX allows `EAGAIN` and `EWOULDBLOCK` to differ and does not say
    /// which a given call returns. They are equal on this platform and checking both costs
    /// nothing; assuming it is a portability bug waiting for a different libc.
    pub fn would_block(self) -> bool {
        self.0 == libc::EAGAIN || self.0 == libc::EWOULDBLOCK
    }
}

/// Set or clear `O_NONBLOCK`, keeping every other flag.
///
/// Read-modify-write rather than a bare `F_SETFL`: overwriting the flags would clear whatever
/// else the descriptor carries, and on an accepted connection that is not ours to decide.
fn set_nonblocking(fd: libc::c_int, on: bool) -> Result<(), Errno> {
    let flags = checked(unsafe { libc::fcntl(fd, libc::F_GETFL) })?;
    let next = if on { flags | libc::O_NONBLOCK } else { flags & !libc::O_NONBLOCK };
    checked(unsafe { libc::fcntl(fd, libc::F_SETFL, next) })?;
    Ok(())
}

fn socket_fd() -> Result<Fd, Errno> {
    checked(unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) }).map(Fd)
}

/// A listening Unix socket.
pub struct Listener(Fd);

impl Listener {
    /// Bind and listen. The caller unlinks any stale socket file first.
    ///
    /// Not done here on purpose: removing a path is destructive and this function should not
    /// decide that a file in the way is stale. `server::bind` probes for a live server before
    /// unlinking, which is the check that separates replacing a dead socket from stealing a live
    /// one's clients.
    pub fn bind(path: &[u8], backlog: i32) -> Result<Listener, Errno> {
        let fd = socket_fd()?;
        let addr = address(path)?;
        checked(unsafe {
            libc::bind(
                fd.raw(),
                core::ptr::from_ref(&addr).cast(),
                core::mem::size_of::<libc::sockaddr_un>() as libc::socklen_t,
            )
        })?;
        checked(unsafe { libc::listen(fd.raw(), backlog) })?;
        Ok(Listener(fd))
    }

    /// Whether `accept` returns immediately when nothing is waiting.
    pub fn set_nonblocking(&self, on: bool) -> Result<(), Errno> {
        set_nonblocking(self.0.raw(), on)
    }

    /// Wait for a connection.
    ///
    /// `EINTR` is retried rather than returned. A signal arriving while blocked in `accept` is
    /// not a failure of the socket, and every caller here would have looped anyway - so the loop
    /// is in one place instead of three.
    pub fn accept(&self) -> Result<Stream, Errno> {
        loop {
            let fd = unsafe { libc::accept(self.0.raw(), core::ptr::null_mut(), core::ptr::null_mut()) };
            if fd == -1 {
                let e = Errno::current();
                if e.0 == libc::EINTR {
                    continue;
                }
                return Err(e);
            }
            return Ok(Stream(Fd(fd)));
        }
    }
}

/// One connection.
pub struct Stream(Fd);

impl Stream {
    /// Connect to a listening socket.
    pub fn connect(path: &[u8]) -> Result<Stream, Errno> {
        let fd = socket_fd()?;
        let addr = address(path)?;
        checked(unsafe {
            libc::connect(
                fd.raw(),
                core::ptr::from_ref(&addr).cast(),
                core::mem::size_of::<libc::sockaddr_un>() as libc::socklen_t,
            )
        })?;
        Ok(Stream(fd))
    }

    /// Whether reads and writes return immediately rather than waiting.
    ///
    /// A connection accepted on a non-blocking listener inherits that on some platforms, and a
    /// non-blocking read of a client that has not typed yet looks like end of stream - which
    /// ends the session. The session wants to block; only the accept did not.
    pub fn set_nonblocking(&self, on: bool) -> Result<(), Errno> {
        set_nonblocking(self.0.raw(), on)
    }

    /// A second handle on the same connection, for reading while the other writes.
    pub fn duplicate(&self) -> Result<Stream, Errno> {
        self.0.duplicate().map(Stream)
    }

    /// Write all of it, or say why not.
    ///
    /// `write` is allowed to accept less than it was given and every caller here means "all of
    /// it", so the loop belongs in the wrapper. A zero return with bytes outstanding is a peer
    /// that went away, which is `EPIPE` and is reported as one rather than as a silent stall.
    pub fn write_all(&mut self, mut bytes: &[u8]) -> Result<(), Errno> {
        while !bytes.is_empty() {
            let wrote = unsafe { libc::write(self.0.raw(), bytes.as_ptr().cast(), bytes.len()) };
            if wrote < 0 {
                let e = Errno::current();
                if e.0 == libc::EINTR {
                    continue;
                }
                return Err(e);
            }
            if wrote == 0 {
                return Err(Errno(libc::EPIPE));
            }
            bytes = &bytes[wrote as usize..];
        }
        Ok(())
    }

    /// Read once into `buf`, returning how much arrived. Zero is a clean end of stream.
    fn read(&self, buf: &mut [u8]) -> Result<usize, Errno> {
        loop {
            let got = unsafe { libc::read(self.0.raw(), buf.as_mut_ptr().cast(), buf.len()) };
            if got < 0 {
                let e = Errno::current();
                if e.0 == libc::EINTR {
                    continue;
                }
                return Err(e);
            }
            return Ok(got as usize);
        }
    }

    /// This connection as a line reader. Consumes the handle: the buffer it grows holds bytes
    /// already taken off the socket, so reading around it afterwards would lose them.
    pub fn lines(self) -> Lines {
        Lines { stream: self, buf: vec![0u8; 8192], filled: 0, at: 0 }
    }
}

/// A line reader over a connection: `std::io::BufReader::lines`, without `std`.
pub struct Lines {
    stream: Stream,
    buf: Vec<u8>,
    /// How much of `buf` holds bytes read from the socket.
    filled: usize,
    /// How much of that has been handed out.
    at: usize,
}

impl Lines {
    /// The next line, without its newline, or `None` at a clean end of stream.
    ///
    /// A final line with no trailing newline is returned, because a peer that wrote one and
    /// closed said something and dropping it would lose a request. `None` therefore means the
    /// stream ended with nothing pending, which is the only unambiguous end.
    pub fn next_line(&mut self) -> Result<Option<String>, Errno> {
        let mut line: Vec<u8> = Vec::new();
        loop {
            // Anything already buffered, up to a newline.
            if let Some(n) = self.buf[self.at..self.filled].iter().position(|b| *b == b'\n') {
                line.extend_from_slice(&self.buf[self.at..self.at + n]);
                self.at += n + 1;
                return Ok(Some(String::from_utf8_lossy(&line).into_owned()));
            }
            line.extend_from_slice(&self.buf[self.at..self.filled]);
            self.at = 0;
            self.filled = 0;

            let got = self.stream.read(&mut self.buf)?;
            if got == 0 {
                return Ok(if line.is_empty() {
                    None
                } else {
                    Some(String::from_utf8_lossy(&line).into_owned())
                });
            }
            self.filled = got;
        }
    }

    /// The write half, for a reply on the same connection.
    pub fn stream(&mut self) -> &mut Stream {
        &mut self.stream
    }
}
