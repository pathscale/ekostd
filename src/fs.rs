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

use alloc::string::String;
use alloc::vec::Vec;

use crate::libc_lite as libc;
use crate::path::{Path, PathBuf};
use crate::{checked, Errno};

/// Mount read-only.
pub const MS_RDONLY: u64 = 1;
/// Ignore suid and sgid bits.
pub const MS_NOSUID: u64 = 2;
/// Disallow access to device special files.
pub const MS_NODEV: u64 = 4;
/// Disallow program execution.
pub const MS_NOEXEC: u64 = 8;
/// Perform a bind mount.
pub const MS_BIND: u64 = 4096;
/// Move a subtree.
pub const MS_MOVE: u64 = 8192;
/// Lazy unmount (`MNT_DETACH`).
pub const MNT_DETACH: i32 = 2;

/// Mount a filesystem at `target`.
pub fn mount(
    source: Option<&str>,
    target: &str,
    fstype: Option<&str>,
    flags: u64,
    data: Option<&[u8]>,
) -> Result<(), Errno> {
    #[cfg(target_os = "none")]
    {
        ::libc_rs::fs::mount(
            source.map(str::as_bytes),
            target.as_bytes(),
            fstype.map(str::as_bytes),
            flags,
            data,
        )
        .map_err(|e| Errno(e.0))
    }
    #[cfg(all(not(target_os = "none"), target_os = "linux"))]
    {
        let src_c = source.map(|s| Path::new(s).as_c());
        let tgt_c = Path::new(target).as_c();
        let fs_c = fstype.map(|s| Path::new(s).as_c());
        let data_c = data.map(|d| Path::new(d).as_c());
        let src_ptr = src_c.as_ref().map_or(core::ptr::null(), |v| v.as_ptr().cast());
        let fs_ptr = fs_c.as_ref().map_or(core::ptr::null(), |v| v.as_ptr().cast());
        let data_ptr = data_c.as_ref().map_or(core::ptr::null(), |v| v.as_ptr().cast());
        checked(unsafe {
            libc::mount(src_ptr, tgt_c.as_ptr().cast(), fs_ptr, flags as libc::c_ulong, data_ptr)
        })
        .map(|_| ())
    }
    #[cfg(all(not(target_os = "none"), not(target_os = "linux")))]
    {
        let _ = (source, target, fstype, flags, data);
        Err(Errno(libc::ENOSYS))
    }
}

/// Unmount `target` with `flags` (such as [`MNT_DETACH`]).
pub fn umount2(target: &str, flags: i32) -> Result<(), Errno> {
    #[cfg(target_os = "none")]
    {
        ::libc_rs::fs::umount2(target.as_bytes(), flags).map_err(|e| Errno(e.0))
    }
    #[cfg(all(not(target_os = "none"), target_os = "linux"))]
    {
        checked(with_c_path_ptr(target.as_bytes(), |p| unsafe { libc::umount2(p, flags) }))
            .map(|_| ())
    }
    #[cfg(all(not(target_os = "none"), not(target_os = "linux")))]
    {
        let _ = (target, flags);
        Err(Errno(libc::ENOSYS))
    }
}

/// Change the root directory of the calling process to `path`.
pub fn chroot(path: &str) -> Result<(), Errno> {
    #[cfg(target_os = "none")]
    {
        ::libc_rs::fs::chroot(path.as_bytes()).map_err(|e| Errno(e.0))
    }
    #[cfg(not(target_os = "none"))]
    {
        checked(with_c_path_ptr(path.as_bytes(), |p| unsafe { libc::chroot(p) })).map(|_| ())
    }
}

/// Flush all filesystem caches to storage (`sync(2)`).
pub fn sync_filesystems() {
    #[cfg(target_os = "none")]
    {
        ::libc_rs::fs::sync_all_filesystems();
    }
    #[cfg(not(target_os = "none"))]
    {
        unsafe { libc::sync() };
    }
}

/// Return entry names directly inside `path` (excluding `.` and `..`).
pub fn read_dir_names(path: &str) -> Result<Vec<String>, Errno> {
    #[cfg(target_os = "none")]
    {
        let fd = ::libc_rs::fs::open(
            path.as_bytes(),
            ::libc_rs::fs::O_RDONLY | ::libc_rs::fs::O_DIRECTORY | ::libc_rs::fs::O_CLOEXEC,
            0,
        )
        .map_err(|e| Errno(e.0))?;
        let mut buf = [0u8; 4096];
        let mut out = Vec::new();
        loop {
            let n = match ::libc_rs::fs::getdents64(fd, &mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    let _ = ::libc_rs::io::close(fd);
                    return Err(Errno(e.0));
                }
            };
            for entry in ::libc_rs::fs::parse_dirents(&buf[..n]) {
                let bytes = entry.name;
                if (bytes.len() == 1 && bytes[0] == b'.')
                    || (bytes.len() == 2 && bytes[0] == b'.' && bytes[1] == b'.')
                {
                    continue;
                }
                if let Ok(s) = core::str::from_utf8(bytes) {
                    let mut name = String::new();
                    name.push_str(s);
                    out.push(name);
                }
            }
        }
        let _ = ::libc_rs::io::close(fd);
        Ok(out)
    }
    #[cfg(not(target_os = "none"))]
    {
        let c = Path::new(path).as_c();
        let dir = unsafe { libc::opendir(c.as_ptr().cast()) };
        if dir.is_null() {
            return Err(Errno::current());
        }
        let mut out = Vec::new();
        loop {
            Errno::clear();
            let entry = unsafe { libc::readdir(dir) };
            if entry.is_null() {
                let e = Errno::current();
                unsafe { libc::closedir(dir) };
                if e.0 != 0 {
                    return Err(e);
                }
                return Ok(out);
            }
            let name = unsafe { (*entry).d_name };
            let mut bytes = Vec::new();
            let mut index = 0;
            while index < name.len() && name[index] != 0 {
                bytes.push(name[index] as u8);
                index += 1;
            }
            if (bytes.len() == 1 && bytes[0] == b'.')
                || (bytes.len() == 2 && bytes[0] == b'.' && bytes[1] == b'.')
            {
                continue;
            }
            if let Ok(s) = core::str::from_utf8(&bytes) {
                let mut entry_name = String::new();
                entry_name.push_str(s);
                out.push(entry_name);
            }
        }
    }
}

fn with_c_path_ptr<R>(path: &[u8], f: impl FnOnce(*const libc::c_char) -> R) -> R {
    if path.last() == Some(&0) {
        return f(path.as_ptr().cast());
    }
    if path.len() < 256 {
        let mut buf = [0u8; 256];
        buf[..path.len()].copy_from_slice(path);
        return f(buf.as_ptr().cast());
    }
    let c = Path::new(path).as_c();
    f(c.as_ptr().cast())
}

/// Remove a name, and do not care whether it was there.
///
/// **Deliberately returns nothing.** Every caller is clearing a stale socket file before binding,
/// and "it was not there" is the ordinary case rather than a failure - which is why the `std`
/// version of this line was `let _ = std::fs::remove_file(..)` at all three sites. Removing a
/// *live* socket would be a real hazard, and the guard against that is `ekod`'s liveness probe,
/// not this function.
pub fn unlink(path: &[u8]) {
    with_c_path_ptr(path, |p| {
        unsafe { libc::unlink(p) };
    });
}

/// Create a directory, succeeding if it already exists.
pub fn mkdir(path: &[u8], mode: u32) -> Result<(), Errno> {
    if with_c_path_ptr(path, |p| unsafe { libc::mkdir(p, mode as libc::mode_t) }) == 0 {
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
    checked(with_c_path_ptr(path, |p| unsafe { libc::access(p, libc::F_OK) })).is_ok()
}

/// Remove an empty directory.
///
/// Empty, because `rmdir(2)` is. [`remove_dir_all`] is the recursive one, and keeping them
/// apart means a caller that meant to delete one directory cannot delete a tree by typo.
pub fn rmdir(path: &[u8]) -> Result<(), Errno> {
    checked(with_c_path_ptr(path, |p| unsafe { libc::rmdir(p) })).map(|_| ())
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
    #[cfg(target_os = "none")]
    {
        let dir_path = path.as_ref();
        let fd = ::libc_rs::fs::open(
            dir_path.as_bytes(),
            ::libc_rs::fs::O_RDONLY | ::libc_rs::fs::O_DIRECTORY | ::libc_rs::fs::O_CLOEXEC,
            0,
        )
        .map_err(|e| crate::file::Error::from_errno(Errno(e.0), "opendir"))?;
        let mut buf = [0u8; 4096];
        let mut out = Vec::new();
        loop {
            let n = match ::libc_rs::fs::getdents64(fd, &mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    let _ = ::libc_rs::io::close(fd);
                    return Err(crate::file::Error::from_errno(Errno(e.0), "readdir"));
                }
            };
            for entry in ::libc_rs::fs::parse_dirents(&buf[..n]) {
                let bytes = entry.name;
                if (bytes.len() == 1 && bytes[0] == b'.')
                    || (bytes.len() == 2 && bytes[0] == b'.' && bytes[1] == b'.')
                {
                    continue;
                }
                out.push(dir_path.join(Path::new(bytes)));
            }
        }
        let _ = ::libc_rs::io::close(fd);
        Ok(out)
    }
    #[cfg(not(target_os = "none"))]
    {
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
            let mut bytes = Vec::new();
            let mut index = 0;
            while index < name.len() && name[index] != 0 {
                bytes.push(name[index] as u8);
                index += 1;
            }
            if (bytes.len() == 1 && bytes[0] == b'.')
                || (bytes.len() == 2 && bytes[0] == b'.' && bytes[1] == b'.')
            {
                continue;
            }
            out.push(path.as_ref().join(Path::new(&bytes)));
        }
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
    let children = read_dir(path)?;
    let mut i = 0;
    while i < children.len() {
        let child = &children[i];
        i += 1;
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
