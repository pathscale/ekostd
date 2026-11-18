//! `poll(2)` wrapper with a backend-independent `PollFd` layout.

use crate::libc_lite as libc;

/// There is data to read.
pub const POLLIN: i16 = libc::POLLIN;
/// Writing now will not block.
pub const POLLOUT: i16 = 0x0004;
/// Error condition.
pub const POLLERR: i16 = 0x0008;
/// Hung up.
pub const POLLHUP: i16 = libc::POLLHUP;

/// A file descriptor and event mask for [`poll`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PollFd {
    /// File descriptor to poll.
    pub fd: i32,
    /// Requested events.
    pub events: i16,
    /// Returned events.
    pub revents: i16,
}

const _: () = {
    assert!(core::mem::size_of::<PollFd>() == core::mem::size_of::<libc::pollfd>());
    assert!(core::mem::align_of::<PollFd>() == core::mem::align_of::<libc::pollfd>());
};

/// Wait for events on `fds` up to `timeout_ms` milliseconds.
pub fn poll(fds: &mut [PollFd], timeout_ms: i32) -> Result<usize, crate::Errno> {
    crate::checked(unsafe {
        libc::poll(fds.as_mut_ptr().cast::<libc::pollfd>(), fds.len() as libc::nfds_t, timeout_ms)
    })
    .map(|n| n as usize)
}
