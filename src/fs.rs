//! Names in the filesystem: making them, listing them, and removing them.
//!
//! It began as three calls about the socket *path* rather than about files, because a Unix
//! socket is a name in the filesystem and binding one means removing whatever stale name is in
//! the way. A caller that walks a tree of inputs wanted four more, and they are here rather
//! than in `file` because they are about the directory rather than the contents: `file` opens
//! and reads, this makes, lists and unmakes.
//!
//! One of them is dangerous and says so at its own doc comment. [`remove_dir_all`] is the only
//! thing in this crate that can destroy something the caller did not name, and the property
//! that stops it is `lstat` rather than `stat`.

// `daemon` is `#![no_std]`. These arrive with the standard prelude and have no path to
// match, which is why a `std::` grep cannot see them and the attribute has to be flipped
// to find them at all.

use alloc::vec::Vec;

use crate::path::{Path, PathBuf};
use crate::{checked, Errno};

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
    if e.0 == libc::EEXIST {
        Ok(())
    } else {
        Err(e)
    }
}

/// Whether a name exists at all.
pub fn exists(path: &[u8]) -> bool {
    checked(unsafe { libc::access(path.as_ptr().cast(), libc::F_OK) }).is_ok()
}

/// Remove an empty directory.
///
/// Empty, because `rmdir(2)` is. [`remove_dir_all`] is the recursive one, and keeping them
/// apart means a caller that meant to delete one directory cannot delete a tree by typo.
pub fn rmdir(path: &[u8]) -> Result<(), Errno> {
    checked(unsafe { libc::rmdir(path.as_ptr().cast()) }).map(|_| ())
}

/// Every name directly inside a directory, as full paths, without `.` and `..`.
///
/// **Full paths rather than file names**, because every caller here goes on to `stat` or open
/// what it found, and rebuilding the path at each call site is where a join goes wrong.
///
/// **No file type.** `dirent` carries a `d_type` and it is `DT_UNKNOWN` on filesystems that do
/// not keep one, so a caller that branched on it would work on one machine and not another.
/// `Path::is_dir` asks the kernel and is right everywhere.
///
/// The order is the filesystem's, which is to say arbitrary. A caller that needs a stable
/// order sorts, and any caller walking a tree of inputs should, because two runs over one
/// directory reading files in different orders is a reproducibility bug waiting for a busy day.
pub fn read_dir(path: impl AsRef<Path>) -> crate::file::Result<Vec<PathBuf>> {
    let c = path.as_ref().as_c();
    let dir = unsafe { libc::opendir(c.as_ptr().cast()) };
    if dir.is_null() {
        return Err(crate::file::Error::from_errno(Errno::current(), "opendir"));
    }
    let mut out = Vec::new();
    loop {
        // `readdir` reports end-of-directory and failure the same way, with a null return, and
        // the difference is whether it set errno. Clearing it first is the only way to ask.
        Errno::clear();
        let entry = unsafe { libc::readdir(dir) };
        if entry.is_null() {
            let e = Errno::current();
            unsafe { libc::closedir(dir) };
            if e.0 != 0 {
                return Err(crate::file::Error::from_errno(e, "readdir"));
            }
            return Ok(out);
        }
        let name = unsafe { (*entry).d_name };
        let bytes: Vec<u8> = name.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        out.push(path.as_ref().join(Path::new(&bytes)));
    }
}

/// Create a directory and every missing parent, succeeding if it is already there.
///
/// Walks down rather than up, so the first `mkdir` that is not `EEXIST` is reported against the
/// component that actually failed. A caller told only that the leaf failed has to guess which
/// ancestor was unwritable.
pub fn create_dir_all(path: impl AsRef<Path>) -> crate::file::Result<()> {
    let path = path.as_ref();
    let mut so_far = PathBuf::new();
    if path.is_absolute() {
        so_far.push(Path::new("/"));
    }
    for part in path.parts() {
        if part.is_empty() {
            continue;
        }
        so_far.push(Path::new(part));
        match mkdir(&so_far.as_path().as_c(), 0o755) {
            Ok(()) => {}
            Err(e) => return Err(crate::file::Error::from_errno(e, "mkdir")),
        }
    }
    Ok(())
}

/// Remove a directory and everything under it.
///
/// **Does not follow symlinks**, which is the whole hazard in a recursive delete: a link
/// pointing out of the tree would take the deletion with it. A link is a name, `unlink` removes
/// the name, and only a real directory is descended into.
pub fn remove_dir_all(path: impl AsRef<Path>) -> crate::file::Result<()> {
    let path = path.as_ref();
    for child in read_dir(path)? {
        // `symlink_metadata` rather than `metadata`: the question is what the *name* is, not
        // what it points at.
        let link = crate::file::symlink_metadata(child.as_path())?;
        if link.is_dir() {
            remove_dir_all(child.as_path())?;
        } else {
            unlink(&child.as_path().as_c());
        }
    }
    rmdir(&path.as_c()).map_err(|e| crate::file::Error::from_errno(e, "rmdir"))
}

/// Copy a file's contents to a new name.
///
/// The destination is truncated if it exists, as `std::fs::copy` does. Permissions are not
/// copied: every caller here is writing into a directory it just made, and carrying a mode
/// across is how a copy of a source file comes out executable.
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> crate::file::Result<u64> {
    let mut source = crate::file::File::open(from)?;
    let mut bytes = Vec::new();
    source.read_to_end(&mut bytes)?;
    let mut sink = crate::file::File::create(to)?;
    sink.write_all(&bytes)?;
    Ok(bytes.len() as u64)
}
