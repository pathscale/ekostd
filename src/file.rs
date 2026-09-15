//! Files, and the error type the rest of this crate reports them with.
//!
//! `std::io` is 74 uses across the compiler, and what those uses need is narrow: open a file,
//! read it whole, write bytes, ask whether it exists and how big it is. `std::io`'s generality -
//! `BufReader`, `Seek`, the `Read`/`Write` traits over anything - exists so the same code can
//! drive a socket, a cursor and a compressor. This compiler reads source and writes artefacts.
//!
//! `Error` here is an `Errno` and a note, not a boxed `dyn Error`. The boxing in `std::io::Error`
//! exists so a user-supplied `Read` can report a failure that is not an errno; there are no
//! user-supplied readers here.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::path::Path;
use crate::Errno;

/// A file operation that failed.
///
/// `what` is a `Cow`, not a `&'static str`. Most failures name a syscall and nothing else, which
/// a literal covers with no allocation - but callers at the edge of the program build a reason
/// out of a value they were given (`format!("{why}")`), and a borrowed-only field cannot hold
/// that. `std::io::Error` boxes a `dyn Error` for the same reason; this is the same need with
/// two orders of magnitude less machinery.
#[derive(Clone, PartialEq, Eq)]
pub struct Error {
    errno: Errno,
    kind: ErrorKind,
    what: alloc::borrow::Cow<'static, str>,
}

impl Error {
    /// Build one from an errno the caller obtained itself.
    ///
    /// For callers that make a syscall this crate does not wrap - `flock` calls `fcntl` directly
    /// and needs to report the failure in the same type as its `open`.
    pub fn from_errno(errno: Errno, what: &'static str) -> Error {
        Error { errno, kind: ErrorKind::of(errno), what: alloc::borrow::Cow::Borrowed(what) }
    }

    /// The underlying errno.
    pub fn errno(&self) -> Errno {
        self.errno
    }

    /// The file was larger than the caller's limit.
    ///
    /// `EFBIG` because that is what it means, even though no syscall reported it: the caller has
    /// a size limit of its own and this is how it says so in the same type.
    pub fn too_large() -> Error {
        Error::from_errno(Errno(libc::EFBIG), "file too large")
    }

    /// `getcwd` failed.
    pub fn no_cwd() -> Error {
        Error::from_errno(Errno::current(), "getcwd")
    }

    /// The bytes were not what the caller expected.
    ///
    /// `EILSEQ`, and no syscall reported it: a decoder that has read a file and found it
    /// malformed needs to report that in the same type as the read itself.
    pub fn invalid_data(what: &'static str) -> Error {
        Error::from_errno(Errno(libc::EILSEQ), what)
    }

    /// The broad category of failure.
    ///
    /// A handful of cases, not `std::io::ErrorKind`'s forty: these are the ones the compiler
    /// words a different message for. Everything else is `Other` and gets the errno's own text,
    /// which says more than a category name would.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// An error in a named category, carrying a reason the caller built.
    pub fn new(kind: ErrorKind, what: impl Into<alloc::borrow::Cow<'static, str>>) -> Error {
        Error { errno: Errno(0), kind, what: what.into() }
    }

    /// An error that fits no category, carrying a reason.
    pub fn other(what: impl Into<alloc::borrow::Cow<'static, str>>) -> Error {
        Error::new(ErrorKind::Other, what)
    }

    /// Whether the file was not there.
    ///
    /// Its own method because it is the one distinction callers actually make: a missing file is
    /// usually a decision, and every other failure is a report.
    pub fn is_not_found(&self) -> bool {
        self.errno.0 == libc::ENOENT
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.errno.0 == 0 {
            // Built by `new`/`other` rather than by a failed syscall: there is no errno to
            // render, and printing "Undefined error: 0" beside the reason would be noise.
            return f.write_str(&self.what);
        }
        let mut buf = [0u8; 128];
        write!(f, "{}: {}", self.what, self.errno.message(&mut buf))
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl core::error::Error for Error {}

/// The failure categories callers word a message for.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ErrorKind {
    /// `ENOENT`.
    NotFound,
    /// `EACCES` or `EPERM`.
    PermissionDenied,
    /// `EISDIR`.
    IsADirectory,
    /// The caller was handed something it could not use. No errno behind it.
    InvalidInput,
    /// Bytes that were read but did not mean what they had to. No errno behind it.
    InvalidData,
    /// Anything else; the errno's own message is the description.
    Other,
}

impl ErrorKind {
    /// The category an errno falls into.
    fn of(errno: Errno) -> ErrorKind {
        match errno.0 {
            libc::ENOENT => ErrorKind::NotFound,
            libc::EACCES | libc::EPERM => ErrorKind::PermissionDenied,
            libc::EISDIR => ErrorKind::IsADirectory,
            libc::EILSEQ => ErrorKind::InvalidData,
            _ => ErrorKind::Other,
        }
    }
}

/// The result of a file operation.
pub type Result<T> = core::result::Result<T, Error>;

fn err(what: &'static str) -> Error {
    let errno = Errno::current();
    Error { errno, kind: ErrorKind::of(errno), what: alloc::borrow::Cow::Borrowed(what) }
}

/// An open file.
///
/// Closes on drop. There is no `sync_all`: nothing here writes anything a crash must not lose,
/// and adding one would imply a durability guarantee the compiler does not make.
pub struct File {
    fd: libc::c_int,
}

impl File {
    /// Open for reading.
    pub fn open(path: impl AsRef<Path>) -> Result<File> {
        let c = path.as_ref().as_c();
        let fd = unsafe { libc::open(c.as_ptr().cast(), libc::O_RDONLY) };
        if fd < 0 {
            return Err(err("open"));
        }
        Ok(File { fd })
    }

    /// Create or truncate for writing.
    pub fn create(path: impl AsRef<Path>) -> Result<File> {
        let c = path.as_ref().as_c();
        let fd = unsafe {
            libc::open(c.as_ptr().cast(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644)
        };
        if fd < 0 {
            return Err(err("create"));
        }
        Ok(File { fd })
    }

    /// Open for appending, creating if absent.
    pub fn append(path: impl AsRef<Path>) -> Result<File> {
        let c = path.as_ref().as_c();
        let fd = unsafe {
            libc::open(c.as_ptr().cast(), libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND, 0o644)
        };
        if fd < 0 {
            return Err(err("append"));
        }
        Ok(File { fd })
    }

    /// Open an existing file for both reading and writing.
    pub fn open_rw(path: impl AsRef<Path>) -> Result<File> {
        let c = path.as_ref().as_c();
        let fd = unsafe { libc::open(c.as_ptr().cast(), libc::O_RDWR) };
        if fd < 0 {
            return Err(err("open"));
        }
        Ok(File { fd })
    }

    /// Create or truncate, opened for both reading and writing.
    ///
    /// `create` alone is write-only, which is what a file being written wants - but the metadata
    /// encoder rewinds its own output to measure what it wrote when `-Zmeta-stats` is on, so it
    /// needs to read back a file it is writing.
    pub fn create_rw(path: impl AsRef<Path>) -> Result<File> {
        let c = path.as_ref().as_c();
        let fd = unsafe {
            libc::open(c.as_ptr().cast(), libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC, 0o644)
        };
        if fd < 0 {
            return Err(err("create"));
        }
        Ok(File { fd })
    }

    /// Move the read/write position to an absolute offset, returning where it landed.
    pub fn seek(&mut self, offset: u64) -> Result<u64> {
        let n = unsafe { libc::lseek(self.fd, offset as libc::off_t, libc::SEEK_SET) };
        if n < 0 {
            return Err(err("lseek"));
        }
        Ok(n as u64)
    }

    /// Where the read/write position currently is.
    pub fn stream_position(&self) -> Result<u64> {
        let n = unsafe { libc::lseek(self.fd, 0, libc::SEEK_CUR) };
        if n < 0 {
            return Err(err("lseek"));
        }
        Ok(n as u64)
    }

    /// Move the position back to the start.
    pub fn rewind(&mut self) -> Result<()> {
        self.seek(0).map(|_| ())
    }

    /// `fstat(2)` on the open descriptor.
    pub fn metadata(&self) -> Result<Metadata> {
        let mut st: libc::stat = unsafe { core::mem::zeroed() };
        if unsafe { libc::fstat(self.fd, &mut st) } != 0 {
            return Err(err("fstat"));
        }
        Ok(Metadata { st })
    }

    /// Read into a buffer, returning how much arrived. Zero means end of file.
    pub fn read(&mut self, out: &mut [u8]) -> Result<usize> {
        loop {
            let n = unsafe { libc::read(self.fd, out.as_mut_ptr().cast(), out.len()) };
            if n < 0 {
                if Errno::current().0 == libc::EINTR {
                    continue;
                }
                return Err(err("read"));
            }
            return Ok(n as usize);
        }
    }

    /// The raw descriptor, for the few callers that mmap or `isatty`.
    pub fn raw(&self) -> libc::c_int {
        self.fd
    }

    /// Read the rest of the file.
    pub fn read_to_end(&mut self, out: &mut Vec<u8>) -> Result<usize> {
        let start = out.len();
        let mut chunk = [0u8; 64 * 1024];
        loop {
            let n = unsafe { libc::read(self.fd, chunk.as_mut_ptr().cast(), chunk.len()) };
            if n < 0 {
                if Errno::current().0 == libc::EINTR {
                    continue;
                }
                return Err(err("read"));
            }
            if n == 0 {
                return Ok(out.len() - start);
            }
            out.extend_from_slice(&chunk[..n as usize]);
        }
    }

    /// Write all of `bytes`, looping over short writes.
    pub fn write_all(&mut self, mut bytes: &[u8]) -> Result<()> {
        while !bytes.is_empty() {
            let n = unsafe { libc::write(self.fd, bytes.as_ptr().cast(), bytes.len()) };
            if n < 0 {
                if Errno::current().0 == libc::EINTR {
                    continue;
                }
                return Err(err("write"));
            }
            bytes = &bytes[n as usize..];
        }
        Ok(())
    }

    /// The file's size in bytes.
    pub fn len(&self) -> Result<u64> {
        let mut st: libc::stat = unsafe { core::mem::zeroed() };
        if unsafe { libc::fstat(self.fd, &mut st) } != 0 {
            return Err(err("fstat"));
        }
        Ok(st.st_size as u64)
    }

    /// Whether the file has no bytes in it.
    ///
    /// A `Result`, like `len`, because the question costs an `fstat` and a caller that cannot
    /// stat the file does not get a `false` that looks like an answer.
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }
}

// `write!` on a `File`, so a caller that was formatting into `std::io::Write` keeps doing so.
//
// `core::fmt::Write` is the only writing trait `core` has, and it takes `&str` rather than
// bytes - which is exactly right here, because `write!` produces text. The `fmt::Error` it
// returns carries nothing, so a failed write is reported as "it failed"; every caller in this
// compiler writes `_ = write!(..)` and discards it anyway, these being ICE notes appended to a
// file that already exists because the compilation is going down.
impl fmt::Write for File {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "File({})", self.fd)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

/// Read a whole file.
pub fn read(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let mut f = File::open(path)?;
    // Size the buffer up front where we can: the common case is one `read` and no realloc.
    let hint = f.len().unwrap_or(0) as usize;
    let mut out = Vec::with_capacity(hint);
    f.read_to_end(&mut out)?;
    Ok(out)
}

/// Read a whole file as UTF-8.
pub fn read_to_string(path: impl AsRef<Path>) -> Result<String> {
    let bytes = read(path)?;
    String::from_utf8(bytes).map_err(|_| Error::from_errno(Errno(libc::EILSEQ), "not utf-8"))
}

/// Write a whole file, replacing it.
pub fn write(path: impl AsRef<Path>, bytes: &[u8]) -> Result<()> {
    File::create(path)?.write_all(bytes)
}

/// Whether a name exists.
pub fn exists(path: impl AsRef<Path>) -> bool {
    let c = path.as_ref().as_c();
    unsafe { libc::access(c.as_ptr().cast(), libc::F_OK) == 0 }
}

/// What `stat(2)` says about a name.
pub struct Metadata {
    st: libc::stat,
}

impl Metadata {
    /// Size in bytes.
    pub fn len(&self) -> u64 {
        self.st.st_size as u64
    }

    /// Whether the file has no bytes in it.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether it is a directory.
    pub fn is_dir(&self) -> bool {
        self.st.st_mode & libc::S_IFMT == libc::S_IFDIR
    }

    /// Whether it is a regular file.
    pub fn is_file(&self) -> bool {
        self.st.st_mode & libc::S_IFMT == libc::S_IFREG
    }

    /// The permission bits.
    ///
    /// The raw mode, not a `Permissions` type. The one caller checks whether the output file is
    /// writable, which is a bit test.
    pub fn mode(&self) -> u32 {
        self.st.st_mode as u32
    }

    /// Whether the file is read-only to this process.
    pub fn readonly(&self) -> bool {
        // The owner-write bit, which is what `std::fs::Permissions::readonly` reports on Unix.
        self.st.st_mode & 0o200 == 0
    }

    /// Modification time, as whole seconds since the epoch.
    ///
    /// Seconds, not a `SystemTime`: every caller here compares two of these for staleness, and
    /// carrying a calendar type to answer "is this newer" is the generality this crate avoids.
    pub fn modified_secs(&self) -> i64 {
        // `time_t` is already `i64` on every target this crate supports. A 32-bit port gets a
        // type error here rather than a silent narrowing, which is the right way to find out.
        self.st.st_mtime
    }
}

/// `stat(2)`.
pub fn metadata(path: impl AsRef<Path>) -> Result<Metadata> {
    let c = path.as_ref().as_c();
    let mut st: libc::stat = unsafe { core::mem::zeroed() };
    if unsafe { libc::stat(c.as_ptr().cast(), &mut st) } != 0 {
        return Err(err("stat"));
    }
    Ok(Metadata { st })
}

/// `lstat(2)`: what the **name** is, rather than what it points at.
///
/// The difference is the whole of the hazard in a recursive delete. A symlink to a directory
/// answers `is_dir` under `stat` and is not one, so a walker that descended into it would leave
/// the tree it was given and delete somewhere else.
pub fn symlink_metadata(path: impl AsRef<Path>) -> Result<Metadata> {
    let c = path.as_ref().as_c();
    let mut st: libc::stat = unsafe { core::mem::zeroed() };
    if unsafe { libc::lstat(c.as_ptr().cast(), &mut st) } != 0 {
        return Err(err("lstat"));
    }
    Ok(Metadata { st })
}

/// Resolve a path to an absolute one with symlinks followed.
pub fn canonicalize(path: impl AsRef<Path>) -> Result<crate::path::PathBuf> {
    let c = path.as_ref().as_c();
    // PATH_MAX is the contract of `realpath` with a null second argument on the platforms this
    // targets; the allocation is sized to it rather than guessed.
    let mut buf = alloc::vec![0u8; libc::PATH_MAX as usize];
    let p = unsafe { libc::realpath(c.as_ptr().cast(), buf.as_mut_ptr().cast()) };
    if p.is_null() {
        return Err(err("realpath"));
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    buf.truncate(end);
    Ok(crate::path::PathBuf::from_bytes(buf))
}

/// Create a directory, succeeding if it is already there.
pub fn create_dir_all(path: impl AsRef<Path>) -> Result<()> {
    let p = path.as_ref();
    // Walk down from the root creating each prefix. `mkdir -p` semantics, and the reason this
    // is not one `mkdir` call.
    let bytes = p.as_bytes();
    let mut at = Vec::new();
    if p.is_absolute() {
        at.push(b'/');
    }
    for part in bytes.split(|&b| b == b'/').filter(|s| !s.is_empty()) {
        if at.len() > 1 || (at.len() == 1 && at[0] != b'/') {
            at.push(b'/');
        }
        at.extend_from_slice(part);
        let mut c = at.clone();
        c.push(0);
        if unsafe { libc::mkdir(c.as_ptr().cast(), 0o755) } != 0 {
            let e = Errno::current();
            if e.0 != libc::EEXIST {
                return Err(Error::from_errno(e, "mkdir"));
            }
        }
    }
    Ok(())
}

/// Remove a name, and do not care whether it was there.
pub fn remove_file(path: impl AsRef<Path>) {
    let c = path.as_ref().as_c();
    unsafe { libc::unlink(c.as_ptr().cast()) };
}

/// Create a hard link.
pub fn hard_link(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    let a = from.as_ref().as_c();
    let b = to.as_ref().as_c();
    if unsafe { libc::link(a.as_ptr().cast(), b.as_ptr().cast()) } != 0 {
        return Err(err("link"));
    }
    Ok(())
}

/// Whether an error was "the destination is already there".
pub fn is_already_exists(e: &Error) -> bool {
    e.errno().0 == libc::EEXIST
}

/// Copy a file's contents, creating or truncating the destination.
///
/// Contents only: this does not carry permissions, times or extended attributes, because the one
/// caller is falling back from a failed hard link and wants the bytes.
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<u64> {
    let bytes = read(from)?;
    let n = bytes.len() as u64;
    write(to, &bytes)?;
    Ok(n)
}

/// Make a path absolute without touching the filesystem.
///
/// Unlike [`canonicalize`], this does not resolve symlinks and does not require the path to
/// exist - it prepends the working directory to a relative path and otherwise leaves it alone.
/// `.` and `..` are left in place for the same reason [`crate::path::Path::parts`] does not
/// resolve them: doing so without following symlinks gives a different file.
pub fn absolute(path: impl AsRef<Path>) -> Result<crate::path::PathBuf> {
    let p = path.as_ref();
    if p.is_absolute() {
        return Ok(p.to_path_buf());
    }
    let cwd = crate::env::current_dir().ok_or(Error::from_errno(Errno::current(), "getcwd"))?;
    let mut out = crate::path::PathBuf::from_bytes(cwd);
    out.push(p);
    Ok(out)
}

/// A directory that removes itself and its contents when dropped.
///
/// The name is `<prefix><pid>-<counter><suffix>` under [`crate::env::temp_dir`]. That is not
/// cryptographically unguessable the way `mkdtemp` is; it does not need to be, because the one
/// caller creates a scratch directory for its own compiler output and never puts a secret in it.
/// `O_EXCL` on the create is what makes a collision an error rather than a silent share.
#[derive(Debug)]
pub struct TempDir {
    path: crate::path::PathBuf,
}

impl TempDir {
    /// Create one under the system temporary directory.
    pub fn new(prefix: &str, suffix: &str) -> Result<TempDir> {
        TempDir::new_in(crate::path::Path::new(&crate::env::temp_dir()), prefix, suffix)
    }

    /// Create one under a directory of the caller's choosing.
    ///
    /// Not a convenience: `rustc_metadata` puts its scratch directory beside the output file, so
    /// that moving the finished metadata into place is a rename within one filesystem rather
    /// than a copy across two.
    pub fn new_in(dir: &Path, prefix: &str, suffix: &str) -> Result<TempDir> {
        use core::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let base = dir.to_path_buf();
        for _ in 0..64 {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let name = alloc::format!("{prefix}{}-{n}{suffix}", crate::proc::id());
            let at = base.join(name.as_str());
            let c = at.as_c();
            if unsafe { libc::mkdir(c.as_ptr().cast(), 0o700) } == 0 {
                return Ok(TempDir { path: at });
            }
            let e = Errno::current();
            if e.0 != libc::EEXIST {
                return Err(Error::from_errno(e, "mkdir"));
            }
        }
        Err(Error::from_errno(Errno(libc::EEXIST), "tempdir"))
    }

    /// Where it is.
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        remove_dir_all(self.path.as_path());
    }
}

/// Remove a directory and everything under it, ignoring failures.
///
/// Ignoring them is deliberate: the caller is a `Drop`, which cannot report, and a scratch
/// directory that outlives the process is a wasted inode rather than a correctness problem.
pub fn remove_dir_all(path: &Path) {
    let c = path.as_c();
    let dir = unsafe { libc::opendir(c.as_ptr().cast()) };
    if dir.is_null() {
        return;
    }
    loop {
        let ent = unsafe { libc::readdir(dir) };
        if ent.is_null() {
            break;
        }
        let name_ptr = unsafe { (*ent).d_name.as_ptr() };
        let mut name = Vec::new();
        let mut i = 0isize;
        loop {
            let b = unsafe { *name_ptr.offset(i) } as u8;
            if b == 0 {
                break;
            }
            name.push(b);
            i += 1;
        }
        if name == b"." || name == b".." {
            continue;
        }
        let child = path.join(&name[..]);
        // Recurse into directories, unlink everything else. `lstat`, not `stat`: a symlink to a
        // directory must be removed, not descended into.
        let cc = child.as_c();
        let mut st: libc::stat = unsafe { core::mem::zeroed() };
        if unsafe { libc::lstat(cc.as_ptr().cast(), &mut st) } == 0
            && st.st_mode & libc::S_IFMT == libc::S_IFDIR
        {
            remove_dir_all(child.as_path());
        } else {
            unsafe { libc::unlink(cc.as_ptr().cast()) };
        }
    }
    unsafe { libc::closedir(dir) };
    unsafe { libc::rmdir(c.as_ptr().cast()) };
}

/// The names in a directory, excluding `.` and `..`.
///
/// Returns the whole listing rather than a lazy iterator. `std::fs::read_dir` is lazy because a
/// directory can be enormous and each entry can be a syscall; here the one caller collects the
/// listing immediately to sort it, so laziness would buy an iterator type and no work saved.
pub fn read_dir(path: impl AsRef<Path>) -> Result<Vec<crate::path::PathBuf>> {
    let dir_path = path.as_ref();
    let c = dir_path.as_c();
    let dir = unsafe { libc::opendir(c.as_ptr().cast()) };
    if dir.is_null() {
        return Err(err("opendir"));
    }
    let mut out = Vec::new();
    loop {
        // `readdir` returns NULL both at end-of-directory and on error, distinguished by errno.
        // The distinction is not made here: a truncated listing and a complete one are treated
        // alike, which matches what the caller does with a directory it cannot read at all.
        let ent = unsafe { libc::readdir(dir) };
        if ent.is_null() {
            break;
        }
        let name_ptr = unsafe { (*ent).d_name.as_ptr() };
        let mut name = Vec::new();
        let mut i = 0isize;
        loop {
            let b = unsafe { *name_ptr.offset(i) } as u8;
            if b == 0 {
                break;
            }
            name.push(b);
            i += 1;
        }
        if name == b"." || name == b".." {
            continue;
        }
        out.push(dir_path.join(&name[..]));
    }
    unsafe { libc::closedir(dir) };
    Ok(out)
}

/// Where a symbolic link points.
pub fn read_link(path: impl AsRef<Path>) -> Result<crate::path::PathBuf> {
    let c = path.as_ref().as_c();
    let mut buf = alloc::vec![0u8; libc::PATH_MAX as usize];
    let n = unsafe { libc::readlink(c.as_ptr().cast(), buf.as_mut_ptr().cast(), buf.len()) };
    if n < 0 {
        return Err(err("readlink"));
    }
    buf.truncate(n as usize);
    Ok(crate::path::PathBuf::from_bytes(buf))
}

/// Rename a file, replacing the destination.
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    let a = from.as_ref().as_c();
    let b = to.as_ref().as_c();
    if unsafe { libc::rename(a.as_ptr().cast(), b.as_ptr().cast()) } != 0 {
        return Err(err("rename"));
    }
    Ok(())
}

/// Standard output, as a writer.
///
/// Not a locked handle: `std::io::stdout()` returns one because `println!` and a direct write can
/// interleave, and a global lock keeps a line whole. Everything here writes one formatted buffer
/// per call, and a single `write(2)` to a pipe under `PIPE_BUF` is already atomic.
pub fn stdout() -> Stream {
    Stream { fd: 1 }
}

/// Standard error, as a writer.
pub fn stderr() -> Stream {
    Stream { fd: 2 }
}

/// A borrowed standard stream.
pub struct Stream {
    fd: i32,
}

impl Stream {
    /// Write all of `bytes`.
    pub fn write_all(&mut self, bytes: &[u8]) -> Result<()> {
        crate::print::write_fd(self.fd, bytes);
        Ok(())
    }

    /// A no-op: nothing here buffers.
    pub fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    /// Whether this stream is a terminal.
    pub fn is_terminal(&self) -> bool {
        crate::print::is_terminal(self.fd)
    }
}

impl fmt::Write for Stream {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        crate::print::write_fd(self.fd, s.as_bytes());
        Ok(())
    }
}

/// Standard input.
pub fn stdin() -> Stdin {
    Stdin { buf: Vec::new(), at: 0, eof: false }
}

/// A line reader over file descriptor 0.
///
/// `ekors` is a line protocol on both of its faces - the interactive prompt and the MCP
/// transport - so this reads lines and nothing else. It buffers, because reading a line one
/// `read(2)` per byte off a pipe is what `BufRead` existed to avoid.
pub struct Stdin {
    buf: Vec<u8>,
    at: usize,
    eof: bool,
}

impl Stdin {
    /// The next line, without its newline. `None` at end of input.
    pub fn read_line(&mut self) -> Result<Option<String>> {
        loop {
            if let Some(nl) = self.buf[self.at..].iter().position(|&b| b == b'\n') {
                let line = self.buf[self.at..self.at + nl].to_vec();
                self.at += nl + 1;
                return Ok(Some(String::from_utf8_lossy(&line).into_owned()));
            }
            if self.eof {
                // A final line with no trailing newline is still a line.
                if self.at < self.buf.len() {
                    let line = self.buf[self.at..].to_vec();
                    self.at = self.buf.len();
                    return Ok(Some(String::from_utf8_lossy(&line).into_owned()));
                }
                return Ok(None);
            }
            // Compact what has been consumed before growing, so a long session does not hold
            // every line it has already handed out.
            if self.at > 0 {
                self.buf.drain(..self.at);
                self.at = 0;
            }
            let mut chunk = [0u8; 8192];
            let n = unsafe { libc::read(0, chunk.as_mut_ptr().cast(), chunk.len()) };
            if n < 0 {
                if Errno::current().0 == libc::EINTR {
                    continue;
                }
                return Err(err("read"));
            }
            if n == 0 {
                self.eof = true;
                continue;
            }
            self.buf.extend_from_slice(&chunk[..n as usize]);
        }
    }

    /// The lines, as an iterator.
    pub fn lines(self) -> Lines {
        Lines { inner: self }
    }
}

/// The iterator returned by [`Stdin::lines`].
pub struct Lines {
    inner: Stdin,
}

impl Iterator for Lines {
    type Item = Result<String>;
    fn next(&mut self) -> Option<Result<String>> {
        self.inner.read_line().transpose()
    }
}

/// Accumulates writes and sends them in one go.
///
/// `std::io::BufWriter` exists because a `write(2)` per formatting call is slow. That is still
/// true here, and `JsonEmitter` formats a diagnostic in many small pieces, so the buffer earns
/// its place. It flushes on drop, as `std`'s does - and like `std`'s, a failure during that drop
/// is not reported, which is why anything that must be durable should call `flush` itself.
pub struct BufWriter<W: fmt::Write> {
    // `Option`, because `Drop` flushes and therefore forbids moving the writer out of the
    // struct. `take` in `into_inner` is how the writer leaves; every other path sees `Some`.
    inner: Option<W>,
    buf: String,
}

impl<W: fmt::Write> BufWriter<W> {
    /// Wrap a writer.
    pub fn new(inner: W) -> BufWriter<W> {
        BufWriter { inner: Some(inner), buf: String::new() }
    }

    /// Push everything buffered into the writer.
    pub fn flush(&mut self) -> fmt::Result {
        if self.buf.is_empty() {
            return Ok(());
        }
        let r = match self.inner.as_mut() {
            Some(w) => w.write_str(&self.buf),
            None => Ok(()),
        };
        self.buf.clear();
        r
    }

    /// Give back the writer, flushing first.
    pub fn into_inner(mut self) -> W {
        let _ = self.flush();
        self.inner.take().expect("the writer is taken exactly once")
    }
}

impl<W: fmt::Write> fmt::Write for BufWriter<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.push_str(s);
        // 64 KiB before a syscall, which is `std`'s order of magnitude and well past the size of
        // any single diagnostic.
        if self.buf.len() >= 64 * 1024 {
            return self.flush();
        }
        Ok(())
    }
}

impl<W: fmt::Write> Drop for BufWriter<W> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
