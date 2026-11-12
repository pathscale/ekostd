//! `std`-shaped names over this crate's own functions.
//!
//! Some functions in `env`, `file` and `fs` have signatures that differ from `std`'s by accident:
//! `env::var` returns an `Option`, `fs::read_dir` a `Vec`, `env::temp_dir` bytes, `file::remove_file`
//! nothing. A crate moving off `std` then has to change every call site instead of one path.
//!
//! This module gives the `std` shape at the `std` position (`compat::env::var`,
//! `compat::fs::read_dir`, `compat::io::Error`), so that move is a rename. It is duplication on
//! purpose and it is temporary: when the primary functions take the same shape, `compat::x` becomes
//! `x` and this file goes. Nothing here changes what the primary functions do.

/// `std::env`, as far as callers have needed it.
pub mod env {
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::fmt;

    use crate::file::{Error, Result};
    use crate::path::PathBuf;

    pub use crate::env::{args, var_os};

    /// Why `var` has no text to give. `std::env::VarError`.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum VarError {
        /// The variable is not set.
        NotPresent,
        /// The variable is set to bytes that are not UTF-8; the bytes are carried.
        NotUnicode(Vec<u8>),
    }

    impl fmt::Display for VarError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                VarError::NotPresent => f.write_str("environment variable not found"),
                VarError::NotUnicode(_) => f.write_str("environment variable was not valid unicode"),
            }
        }
    }

    impl core::error::Error for VarError {}

    /// The value of an environment variable as UTF-8, telling unset from not-text.
    pub fn var(name: impl AsRef<str>) -> core::result::Result<String, VarError> {
        let bytes = var_os(name.as_ref()).ok_or(VarError::NotPresent)?;
        String::from_utf8(bytes).map_err(|error| VarError::NotUnicode(error.into_bytes()))
    }

    /// The directory for temporary files, as a path.
    pub fn temp_dir() -> PathBuf {
        PathBuf::from(crate::env::temp_dir())
    }

    /// The working directory, as a path, or why it could not be read.
    pub fn current_dir() -> Result<PathBuf> {
        crate::env::current_dir().map(PathBuf::from).ok_or_else(Error::no_cwd)
    }

    /// The path of the running executable, or that it could not be found.
    pub fn current_exe() -> Result<PathBuf> {
        crate::env::current_exe()
            .map(PathBuf::from)
            .ok_or_else(|| Error::other("the path of the running executable is not available"))
    }
}

/// `std::fs`, as far as callers have needed it.
pub mod fs {
    use crate::file::{Error, Result};
    use crate::libc_lite as libc;
    use crate::path::{Path, PathBuf};
    use crate::Errno;

    pub use crate::file::{
        canonicalize, copy, hard_link, metadata, read, read_link, read_to_string, rename,
        symlink_metadata, write, File, Metadata,
    };
    pub use crate::fs::{create_dir_all, remove_dir_all};

    /// Remove a name, reporting a failure. `file::remove_file` does not report.
    pub fn remove_file(path: impl AsRef<Path>) -> Result<()> {
        let c = path.as_ref().as_c();
        if unsafe { libc::unlink(c.as_ptr().cast()) } == 0 {
            Ok(())
        } else {
            Err(Error::from_errno(Errno::current(), "unlink"))
        }
    }

    /// One name found in a directory.
    #[derive(Clone, Debug)]
    pub struct DirEntry {
        path: PathBuf,
    }

    impl DirEntry {
        /// The full path of the entry: the directory joined with the name.
        pub fn path(&self) -> PathBuf {
            self.path.clone()
        }

        /// The name alone.
        pub fn file_name(&self) -> PathBuf {
            self.path.file_name().map(Path::to_path_buf).unwrap_or_else(PathBuf::new)
        }

        /// What the name is, without following a link, as `std`'s does.
        pub fn metadata(&self) -> Result<Metadata> {
            symlink_metadata(&self.path)
        }

        /// The same answer as `metadata`; `std` has a lighter type here, and its `is_dir` and
        /// `is_file` are the two questions callers ask of it.
        pub fn file_type(&self) -> Result<Metadata> {
            symlink_metadata(&self.path)
        }
    }

    /// The entries of a directory, in the shape `std::fs::ReadDir` has: an iterator of results.
    /// Every item is `Ok`, because the directory is read in full before the first is returned.
    pub struct ReadDir {
        entries: alloc::vec::IntoIter<PathBuf>,
    }

    impl Iterator for ReadDir {
        type Item = Result<DirEntry>;

        fn next(&mut self) -> Option<Result<DirEntry>> {
            self.entries.next().map(|path| Ok(DirEntry { path }))
        }
    }

    /// The entries of a directory, without `.` and `..`, in the filesystem's order.
    pub fn read_dir(path: impl AsRef<Path>) -> Result<ReadDir> {
        Ok(ReadDir { entries: crate::fs::read_dir(path)?.into_iter() })
    }
}

/// `std::io`'s error names. The types are this crate's own.
pub mod io {
    pub use crate::file::{Error, ErrorKind, Result};
}

/// `std::path`. The types are this crate's own.
pub mod path {
    pub use crate::path::{Component, Components, Path, PathBuf};
}
