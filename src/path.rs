//! Paths, as what they are on this platform: bytes.
//!
//! `std::path` is 86 uses across the compiler and the largest single category of remaining `std`.
//! Almost all of that surface exists for a reason this compiler does not have. `std::Path` is
//! generic over `OsStr` so the same type can be a UTF-16 path on Windows and a byte path on Unix;
//! it carries prefix parsing for `C:\` and UNC names; `components()` normalises `.` and `..` under
//! rules that differ per platform.
//!
//! Here a path is a sequence of bytes with `/` between the parts, because that is what the kernel
//! this runs on takes. `Path` borrows those bytes and `PathBuf` owns them, mirroring `str` and
//! `String` - which is the same shape `std` has, so call sites read unchanged.
//!
//! **`as_c` is why `PathBuf` is not just `Vec<u8>`.** Every syscall wants a NUL-terminated
//! pointer, and the bug this type exists to prevent is passing a non-terminated buffer to `open`.
//! The terminator is added on the way out, once, rather than being an invariant every caller has
//! to remember.

use alloc::borrow::{Cow, ToOwned};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// The character between path components.
pub const MAIN_SEPARATOR: char = '/';

/// The byte between path components.
pub const MAIN_SEPARATOR_BYTE: u8 = b'/';

/// Compare byte slices without relying on slice equality lowering.
pub(crate) fn bytes_equal<'a>(mut left: &'a [u8], right: &'a [u8]) -> bool {
    let len = left.len();
    let right_len = right.len();
    if len != right_len {
        left = right;
        return left.len() != right_len;
    }
    let mut equal = true;
    let mut index = 0;
    while index < len {
        if left[index] != right[index] {
            equal = false;
        }
        index += 1;
    }
    equal
}

/// Order byte slices lexicographically without relying on slice comparison specialization.
fn bytes_lt<'a>(mut left: &'a [u8], right: &'a [u8]) -> bool {
    let left_len = left.len();
    let right_len = right.len();
    let mut decided = false;
    let mut less = false;
    let mut index = 0;
    while index < left_len && index < right_len {
        if !decided && left[index] != right[index] {
            less = left[index] < right[index];
            decided = true;
        }
        index += 1;
    }
    if !decided {
        left = right;
        return left_len < left.len();
    }
    less
}

fn bytes_cmp(left: &[u8], right: &[u8]) -> core::cmp::Ordering {
    if bytes_lt(left, right) {
        core::cmp::Ordering::Less
    } else if bytes_lt(right, left) {
        core::cmp::Ordering::Greater
    } else {
        core::cmp::Ordering::Equal
    }
}

fn bytes_rposition(bytes: &[u8], needle: u8) -> Option<usize> {
    let len = bytes.len();
    let mut found = usize::MAX;
    let mut index = 0;
    while index < len {
        if bytes[index] == needle {
            found = index;
        }
        index += 1;
    }
    if found == usize::MAX {
        None
    } else {
        Some(found)
    }
}

fn bytes_position(bytes: &[u8], needle: u8) -> Option<usize> {
    let len = bytes.len();
    let mut found = usize::MAX;
    let mut index = 0;
    while index < len {
        if found == usize::MAX && bytes[index] == needle {
            found = index;
        }
        index += 1;
    }
    if found == usize::MAX {
        None
    } else {
        Some(found)
    }
}

fn bytes_non_slash(bytes: &[u8]) -> Option<usize> {
    let len = bytes.len();
    let mut found = usize::MAX;
    let mut index = 0;
    while index < len {
        if found == usize::MAX && bytes[index] != b'/' {
            found = index;
        }
        index += 1;
    }
    if found == usize::MAX {
        None
    } else {
        Some(found)
    }
}

/// A borrowed path.
#[derive(Hash)]
#[repr(transparent)]
pub struct Path {
    inner: [u8],
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        bytes_equal(&self.inner, &other.inner)
    }
}

impl Eq for Path {}

impl PartialOrd for Path {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        bytes_cmp(&self.inner, &other.inner)
    }
}

impl Path {
    /// Borrow bytes as a path. No validation: the kernel is the authority on what it accepts.
    pub fn new<S: AsRef<[u8]> + ?Sized>(s: &S) -> &Path {
        // Safe: `Path` is `repr(transparent)` over `[u8]`, so the layouts are identical.
        unsafe { &*(s.as_ref() as *const [u8] as *const Path) }
    }

    /// The bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    /// The path as text, when it happens to be UTF-8.
    ///
    /// `None` rather than a lossy conversion: a path that is not UTF-8 still names a real file,
    /// and silently replacing bytes would produce a name that opens nothing.
    pub fn to_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.inner).ok()
    }

    /// Whether the path starts at the root.
    pub fn is_absolute(&self) -> bool {
        self.inner.first() == Some(&b'/')
    }

    /// The final component, if it is a name.
    ///
    /// `std`'s answer, byte for byte: trailing separators and `.` components are skipped, so
    /// `a/b/` and `a/b/.` both name `b`, and a path that ends in `..`, or is only the root or `.`,
    /// has no file name.
    pub fn file_name(&self) -> Option<&Path> {
        let (start, end) = self.last_body_component()?;
        let name = &self.inner[start..end];
        if bytes_equal(name, b"..") {
            None
        } else {
            Some(Path::new(name))
        }
    }

    /// Everything before the final component, or `None` when there is no final component to drop.
    ///
    /// **The root has no parent.** `/` answers `None`, as `std` does, so a walk written as
    /// `while let Some(p) = here.parent()` ends there. It answered `/` once, and every such walk
    /// that reached the root without finding what it wanted spun on it forever.
    ///
    /// The rest is `std`'s answer too. `a` has the empty path as its parent and `/a` has `/`.
    /// Trailing and repeated separators and `.` components are skipped, so `/a/b/` has `/a` and
    /// `a//./b` has `a`. A lone `.` has the empty path, and the empty path has `None`.
    pub fn parent(&self) -> Option<&Path> {
        match self.last_body_component() {
            Some((start, _)) => Some(Path::new(&self.inner[..self.trim_back(start)])),
            None if self.leading_cur_dir() => Some(Path::new(b"")),
            None => None,
        }
    }

    /// Whether the path begins with a `.` that `std` keeps as a component of its own: a relative
    /// path that is `.` or starts `./`. A `.` anywhere else is skipped.
    fn leading_cur_dir(&self) -> bool {
        self.inner.first() == Some(&b'.') && matches!(self.inner.get(1), None | Some(&b'/'))
    }

    /// Where the removable components begin: after the root `/`, or after a leading `.`.
    fn body_start(&self) -> usize {
        if self.is_absolute() || self.leading_cur_dir() {
            1
        } else {
            0
        }
    }

    fn step_trim_back(&self, floor: usize, end: usize) -> (usize, bool) {
        let body = &self.inner[floor..end];
        match bytes_rposition(body, b'/') {
            Some(i) => {
                let last = &body[i + 1..];
                if !last.is_empty() && !bytes_equal(last, b".") {
                    (end, false)
                } else {
                    (floor + i, true)
                }
            }
            None => {
                if !bytes_equal(body, b".") {
                    (end, false)
                } else {
                    (floor, false)
                }
            }
        }
    }

    /// `end` moved back past any trailing separators and `.` components, never into the root or
    /// a leading `.`.
    fn trim_back(&self, mut end: usize) -> usize {
        let floor = self.body_start();
        loop {
            if end <= floor {
                break;
            }
            let (next_end, keep_going) = self.step_trim_back(floor, end);
            end = next_end;
            if !keep_going {
                break;
            }
        }
        end
    }

    /// The byte range of the final component that is a name or `..`, or `None` when the path is
    /// only its root, a leading `.`, or nothing.
    fn last_body_component(&self) -> Option<(usize, usize)> {
        let floor = self.body_start();
        let end = self.trim_back(self.inner.len());
        if end == floor {
            return None;
        }
        let body = &self.inner[floor..end];
        let start = match bytes_rposition(body, b'/') {
            Some(i) => floor + i + 1,
            None => floor,
        };
        Some((start, end))
    }

    /// The extension of the final component, without the dot.
    ///
    /// A leading dot is a hidden file, not an extension - `.bashrc` has no extension - which is
    /// the one rule here that is not obvious from the name.
    pub fn extension(&self) -> Option<&Path> {
        let name = self.file_name()?.as_bytes();
        let i = bytes_rposition(name, b'.')?;
        if i == 0 {
            None
        } else {
            Some(Path::new(&name[i + 1..]))
        }
    }

    /// Join a component onto this path.
    pub fn join(&self, other: impl AsRef<Path>) -> PathBuf {
        let mut out = self.to_owned();
        out.push(other);
        out
    }

    /// Own a copy.
    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf { inner: self.inner.to_vec() }
    }

    /// The path with a NUL appended, ready for a syscall.
    pub fn as_c(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.inner.len() + 1);
        v.extend_from_slice(&self.inner);
        v.push(0);
        v
    }

    /// Whether the leading components match.
    pub fn starts_with(&self, prefix: impl AsRef<Path>) -> bool {
        let p = prefix.as_ref().as_bytes();
        self.inner.len() >= p.len()
            && bytes_equal(&self.inner[..p.len()], p)
            && (self.inner.len() == p.len()
                || self.inner[p.len()] == b'/'
                || (!p.is_empty() && p[p.len() - 1] == b'/'))
    }

    /// The remainder after a matching prefix.
    ///
    /// `Result`, not `Option`, to match `std` - every call site here is a `let Ok(rest) = ..`
    /// or a `?`, and the failure carries no information beyond "it did not match", which is
    /// why the error type is empty.
    pub fn strip_prefix(&self, prefix: impl AsRef<Path>) -> Result<&Path, StripPrefixError> {
        let p = prefix.as_ref().as_bytes();
        if !self.starts_with(Path::new(p)) {
            return Err(StripPrefixError);
        }
        let rest = &self.inner[p.len()..];
        let rest = if !rest.is_empty() && rest[0] == b'/' { &rest[1..] } else { rest };
        Ok(Path::new(rest))
    }

    /// The `/`-separated parts, with empties dropped.
    ///
    /// Not `std`'s `components()`: this does not resolve `.` or `..`, because resolving them
    /// without touching the filesystem is wrong in the presence of symlinks and every caller here
    /// that needs a real answer calls `canonicalize` instead.
    pub fn parts(&self) -> impl Iterator<Item = &[u8]> {
        PathParts { rest: &self.inner }
    }

    /// The final component without its extension.
    pub fn file_stem(&self) -> Option<&Path> {
        let name = self.file_name()?.as_bytes();
        match bytes_rposition(name, b'.') {
            // A leading dot is a hidden file, not an extension, so the whole name is the stem.
            Some(0) | None => Some(Path::new(name)),
            Some(i) => Some(Path::new(&name[..i])),
        }
    }

    /// This path with a different extension.
    pub fn with_extension(&self, ext: &str) -> PathBuf {
        let mut out = self.to_path_buf();
        out.set_extension(ext);
        out
    }

    /// Whether the path does not start at the root.
    pub fn is_relative(&self) -> bool {
        !self.is_absolute()
    }

    /// The path as text, replacing invalid UTF-8.
    ///
    /// For messages. A path that is not UTF-8 still names a real file, and the replaced bytes
    /// name a different one - so never hand the result back to a syscall.
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.to_str().unwrap_or("<non-utf8 path>"))
    }

    /// The path as an opaque byte string.
    ///
    /// `std` returns an `OsStr` here to stay honest about Windows. On this platform a path is
    /// bytes, so this is the bytes - the name is kept because it is what call sites say.
    pub fn as_os_str(&self) -> &[u8] {
        &self.inner
    }

    /// The `/`-separated components.
    ///
    /// Yields [`Component`], not raw slices, because callers match on whether a component is a
    /// normal name or the root - `rustc_span` skips a mapping entry whose target has no normal
    /// component at all.
    pub fn components(&self) -> Components<'_> {
        Components { rest: &self.inner, root: self.is_absolute() }
    }

    /// This path and each of its parents, longest first. Ends after the root, because the root
    /// has no parent.
    pub fn ancestors(&self) -> Ancestors<'_> {
        Ancestors { next: Some(self) }
    }

    /// Whether a file exists at this path.
    pub fn exists(&self) -> bool {
        crate::file::exists(self)
    }

    /// Whether this names a directory.
    pub fn is_dir(&self) -> bool {
        crate::file::metadata(self).map(|m| m.is_dir()).unwrap_or(false)
    }

    /// Whether this names a regular file.
    pub fn is_file(&self) -> bool {
        crate::file::metadata(self).map(|m| m.is_file()).unwrap_or(false)
    }

    /// `stat(2)` on this path.
    pub fn metadata(&self) -> crate::file::Result<crate::file::Metadata> {
        crate::file::metadata(self)
    }

    /// Resolve to an absolute path with symlinks followed.
    pub fn canonicalize(&self) -> crate::file::Result<PathBuf> {
        crate::file::canonicalize(self)
    }

    /// How many bytes the path is.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the path is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Whether the path ends with these bytes.
    ///
    /// Byte-wise, not component-wise: `std::Path::ends_with` compares whole components, but the
    /// callers here are testing for a suffix like `.rlib`, which is not one.
    pub fn ends_with(&self, suffix: impl AsRef<[u8]>) -> bool {
        self.inner.ends_with(suffix.as_ref())
    }

    /// A displayable form, replacing invalid UTF-8 rather than failing.
    ///
    /// For messages only. Never feed the result back to a syscall: the replacement characters
    /// name a different file, or none.
    pub fn display(&self) -> Display<'_> {
        Display { path: self }
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.display(), f)
    }
}

/// The result of [`Path::display`].
pub struct Display<'a> {
    path: &'a Path,
}

impl fmt::Display for Display<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.path.to_str().unwrap_or("<non-utf8 path>"))
    }
}

impl fmt::Debug for Display<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.path.to_str().unwrap_or("<non-utf8 path>"), f)
    }
}

// `serde` is behind a feature so the wrapper does not force it on a crate that has no serialiser.
// `rustc_errors`'s `--error-format=json` emits an artifact path, and a JSON consumer wants text,
// so this serialises lossily rather than as a byte array - which is what `std::path::Path`'s own
// `Serialize` does, and what the existing output already looked like.
#[cfg(feature = "serde")]
impl serde::Serialize for Path {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string_lossy())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for PathBuf {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.as_path().serialize(s)
    }
}

/// The prefix did not match. Carries nothing, because there is nothing else to say.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct StripPrefixError;

impl fmt::Display for StripPrefixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("prefix not found")
    }
}

impl core::error::Error for StripPrefixError {}

/// One `/`-separated part of a path.
#[derive(Copy, Clone, Debug)]
pub enum Component<'a> {
    /// The leading `/`.
    RootDir,
    /// `.`
    CurDir,
    /// `..`
    ParentDir,
    /// Anything else.
    Normal(&'a [u8]),
}

impl<'a> PartialEq for Component<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (Component::RootDir, Component::RootDir) => true,
            (Component::CurDir, Component::CurDir) => true,
            (Component::ParentDir, Component::ParentDir) => true,
            (Component::Normal(a), Component::Normal(b)) => bytes_equal(a, b),
            _ => false,
        }
    }
}

impl<'a> Eq for Component<'a> {}

/// The iterator returned by [`Path::components`].
pub struct Components<'a> {
    rest: &'a [u8],
    root: bool,
}

impl<'a> Components<'a> {
    /// What is left, as a path.
    pub fn as_path(&self) -> &'a Path {
        // The root has already been yielded if it was there, so the remainder is what is left of
        // the byte slice with any leading separators trimmed.
        let start = match bytes_non_slash(self.rest) {
            Some(i) => i,
            None => self.rest.len(),
        };
        let (_, rest) = self.rest.split_at(start);
        Path::new(rest)
    }
}

impl<'a> Iterator for Components<'a> {
    type Item = Component<'a>;
    fn next(&mut self) -> Option<Component<'a>> {
        if self.root {
            self.root = false;
            self.rest = if !self.rest.is_empty() && self.rest[0] == b'/' {
                &self.rest[1..]
            } else {
                self.rest
            };
            return Some(Component::RootDir);
        }
        let mut parts = PathParts { rest: self.rest };
        let Some(part) = parts.next() else {
            self.rest = parts.rest;
            return None;
        };
        self.rest = parts.rest;
        Some(if bytes_equal(part, b".") {
            Component::CurDir
        } else if bytes_equal(part, b"..") {
            Component::ParentDir
        } else {
            Component::Normal(part)
        })
    }
}

struct PathParts<'a> {
    rest: &'a [u8],
}

impl<'a> Iterator for PathParts<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let Some(start) = bytes_non_slash(self.rest) else {
            let (_, rest) = self.rest.split_at(self.rest.len());
            self.rest = rest;
            return None;
        };
        let (_, trimmed) = self.rest.split_at(start);
        let end = match bytes_position(trimmed, b'/') {
            Some(i) => i,
            None => trimmed.len(),
        };
        let (item, rest) = trimmed.split_at(end);
        self.rest = rest;
        Some(item)
    }
}

impl<'a> From<&'a Path> for Cow<'a, Path> {
    fn from(p: &'a Path) -> Cow<'a, Path> {
        Cow::Borrowed(p)
    }
}

impl<'a> From<PathBuf> for Cow<'a, Path> {
    fn from(p: PathBuf) -> Cow<'a, Path> {
        Cow::Owned(p)
    }
}

impl<'a> From<&'a PathBuf> for Cow<'a, Path> {
    fn from(p: &'a PathBuf) -> Cow<'a, Path> {
        Cow::Borrowed(p.as_path())
    }
}

/// The iterator returned by [`Path::ancestors`].
pub struct Ancestors<'a> {
    next: Option<&'a Path>,
}

impl<'a> Iterator for Ancestors<'a> {
    type Item = &'a Path;
    fn next(&mut self) -> Option<&'a Path> {
        let here = self.next?;
        self.next = here.parent();
        Some(here)
    }
}

/// An owned path.
#[derive(Clone, Default, Hash)]
pub struct PathBuf {
    inner: Vec<u8>,
}

impl PartialEq for PathBuf {
    fn eq(&self, other: &Self) -> bool {
        bytes_equal(&self.inner, &other.inner)
    }
}

impl Eq for PathBuf {}

impl PartialOrd for PathBuf {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PathBuf {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        bytes_cmp(&self.inner, &other.inner)
    }
}

impl PathBuf {
    /// An empty path.
    pub fn new() -> PathBuf {
        PathBuf { inner: Vec::new() }
    }

    /// Take ownership of bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> PathBuf {
        PathBuf { inner: bytes }
    }

    /// Borrow as a `Path`.
    pub fn as_path(&self) -> &Path {
        Path::new(&self.inner)
    }

    /// Give up the bytes.
    ///
    /// `into_os_string` is the same call under `std`'s name for it, kept because call sites say
    /// that.
    pub fn into_os_string(self) -> Vec<u8> {
        self.inner
    }

    /// Give up the bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.inner
    }

    /// Append a component.
    ///
    /// An absolute component replaces the whole path, as `std` does: `join("/etc")` on any base
    /// is `/etc`, because that is what the caller asked for and silently making it relative would
    /// open the wrong file.
    pub fn push(&mut self, other: impl AsRef<Path>) {
        let o = other.as_ref().as_bytes();
        if o.first() == Some(&b'/') {
            self.inner.clear();
            self.inner.extend_from_slice(o);
            return;
        }
        if !self.inner.is_empty() && self.inner[self.inner.len() - 1] != b'/' {
            self.inner.push(b'/');
        }
        self.inner.extend_from_slice(o);
    }

    /// Drop the final component. `false` if there was nothing to drop, which includes the root:
    /// `/` stays `/`.
    pub fn pop(&mut self) -> bool {
        match self.as_path().parent() {
            Some(p) => {
                let n = p.as_bytes().len();
                self.inner.truncate(n);
                true
            }
            None => false,
        }
    }

    /// Replace the final component, or append one when there is no file name to replace.
    ///
    /// Pops only a real file name, as `std` does: `a/..` becomes `a/../x`, not `a/x`, because
    /// `..` is a step, not a name, and `.` becomes `./x`.
    pub fn set_file_name(&mut self, name: impl AsRef<Path>) {
        if self.file_name().is_some() {
            self.pop();
        }
        self.push(name);
    }

    /// Replace the extension of the final component.
    pub fn set_extension(&mut self, ext: &str) {
        let name_start = match bytes_rposition(&self.inner, b'/') {
            Some(i) => i + 1,
            None => 0,
        };
        if let Some(dot) = bytes_rposition(&self.inner[name_start..], b'.') {
            if dot != 0 {
                self.inner.truncate(name_start + dot);
            }
        }
        if !ext.is_empty() {
            self.inner.push(b'.');
            self.inner.extend_from_slice(ext.as_bytes());
        }
    }
}

impl<P: AsRef<Path>> FromIterator<P> for PathBuf {
    fn from_iter<I: IntoIterator<Item = P>>(iter: I) -> PathBuf {
        let mut out = PathBuf::new();
        for part in iter {
            out.push(part);
        }
        out
    }
}

impl<P: AsRef<Path>> Extend<P> for PathBuf {
    fn extend<I: IntoIterator<Item = P>>(&mut self, iter: I) {
        for part in iter {
            self.push(part);
        }
    }
}

impl From<Vec<u8>> for PathBuf {
    fn from(v: Vec<u8>) -> PathBuf {
        PathBuf { inner: v }
    }
}

impl From<&str> for PathBuf {
    fn from(s: &str) -> PathBuf {
        PathBuf { inner: s.as_bytes().to_vec() }
    }
}

impl From<String> for PathBuf {
    fn from(s: String) -> PathBuf {
        PathBuf { inner: s.into_bytes() }
    }
}

/// The borrowed form, which `std` also has. Without it `args.iter().map(PathBuf::from)`
/// does not compile and the caller writes a closure that says nothing.
impl From<&String> for PathBuf {
    fn from(s: &String) -> PathBuf {
        PathBuf { inner: s.as_bytes().to_vec() }
    }
}

impl From<&Path> for PathBuf {
    fn from(p: &Path) -> PathBuf {
        p.to_path_buf()
    }
}

impl core::ops::Deref for PathBuf {
    type Target = Path;
    fn deref(&self) -> &Path {
        self.as_path()
    }
}

impl ToOwned for Path {
    type Owned = PathBuf;
    fn to_owned(&self) -> PathBuf {
        self.to_path_buf()
    }
}

impl core::borrow::Borrow<Path> for PathBuf {
    fn borrow(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<Path> for Path {
    fn as_ref(&self) -> &Path {
        self
    }
}

impl AsRef<Path> for PathBuf {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<Path> for str {
    fn as_ref(&self) -> &Path {
        Path::new(self.as_bytes())
    }
}

impl AsRef<Path> for String {
    fn as_ref(&self) -> &Path {
        Path::new(self.as_bytes())
    }
}

impl AsRef<Path> for [u8] {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl AsRef<[u8]> for Path {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl AsRef<[u8]> for PathBuf {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl AsRef<Path> for Vec<u8> {
    fn as_ref(&self) -> &Path {
        Path::new(&self[..])
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.display(), f)
    }
}

impl fmt::Display for PathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_path(), f)
    }
}

impl From<PathBuf> for alloc::sync::Arc<Path> {
    fn from(p: PathBuf) -> alloc::sync::Arc<Path> {
        let bytes: alloc::sync::Arc<[u8]> = alloc::sync::Arc::from(p.as_bytes());
        // Safe: `Path` is `repr(transparent)` over `[u8]`, so `Arc<[u8]>` and `Arc<Path>` have
        // identical fat-pointer layout and pointee representation.
        unsafe { core::mem::transmute::<alloc::sync::Arc<[u8]>, alloc::sync::Arc<Path>>(bytes) }
    }
}

impl<'a> AsRef<Path> for Component<'a> {
    fn as_ref(&self) -> &Path {
        match self {
            Component::RootDir => Path::new(b"/"),
            Component::CurDir => Path::new(b"."),
            Component::ParentDir => Path::new(b".."),
            Component::Normal(s) => Path::new(*s),
        }
    }
}

impl<'a> Component<'a> {
    /// The component as bytes.
    pub fn as_os_str(&self) -> &'a [u8] {
        match self {
            Component::RootDir => b"/",
            Component::CurDir => b".",
            Component::ParentDir => b"..",
            Component::Normal(s) => s,
        }
    }
}

impl From<PathBuf> for Box<Path> {
    fn from(p: PathBuf) -> Box<Path> {
        let src = p.as_bytes();
        let mut exact = Vec::with_capacity(src.len());
        exact.extend_from_slice(src);
        let len = exact.len();
        let ptr = exact.as_mut_ptr();
        core::mem::forget(exact);
        // Safe: `exact` was allocated with `capacity == len`, and `Path` is `repr(transparent)`
        // over `[u8]`.
        unsafe {
            let slice = core::ptr::slice_from_raw_parts_mut(ptr, len) as *mut Path;
            Box::from_raw(slice)
        }
    }
}

impl PartialEq<Path> for PathBuf {
    fn eq(&self, other: &Path) -> bool {
        self.as_path() == other
    }
}

impl PartialEq<&Path> for PathBuf {
    fn eq(&self, other: &&Path) -> bool {
        self.as_path() == *other
    }
}

impl PartialEq<PathBuf> for &Path {
    fn eq(&self, other: &PathBuf) -> bool {
        *self == other.as_path()
    }
}

impl PartialEq<PathBuf> for Path {
    fn eq(&self, other: &PathBuf) -> bool {
        self == other.as_path()
    }
}

impl Default for &Path {
    fn default() -> &'static Path {
        Path::new(b"")
    }
}

impl fmt::Debug for PathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_path(), f)
    }
}
