//! The three file operations this crate makes, and no more.
//!
//! Everything here is about the socket *path* rather than about files: a Unix socket is a name in
//! the filesystem, so binding one means removing whatever stale name is in the way. That is the
//! whole of this crate's business with the filesystem now that the descriptor directory is the
//! only other user.

// `daemon` is `#![no_std]`. These arrive with the standard prelude and have no path to
// match, which is why a `std::` grep cannot see them and the attribute has to be flipped
// to find them at all.

use crate::{Errno, checked};

/// Remove a name, and do not care whether it was there.
///
/// **Deliberately returns nothing.** Every caller is clearing a stale socket file before binding,
/// and "it was not there" is the ordinary case rather than a failure - which is why the `std`
/// version of this line was `let _ = std::fs::remove_file(..)` at all three sites. Removing a
/// *live* socket would be a real hazard, and the guard against that is `ekod`'s liveness probe,
/// not this function.
pub fn unlink(path: &[u8]) {
    unsafe { libc::unlink(path.as_ptr().cast()) };
}

/// Create a directory, succeeding if it already exists.
pub fn mkdir(path: &[u8], mode: libc::mode_t) -> Result<(), Errno> {
    if unsafe { libc::mkdir(path.as_ptr().cast(), mode) } == 0 {
        return Ok(());
    }
    let e = Errno::current();
    if e.0 == libc::EEXIST { Ok(()) } else { Err(e) }
}

/// Whether a name exists at all.
pub fn exists(path: &[u8]) -> bool {
    checked(unsafe { libc::access(path.as_ptr().cast(), libc::F_OK) }).is_ok()
}
