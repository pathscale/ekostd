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

/// A borrowed path.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Path {
    inner: [u8],
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

    /// The final component, if there is one.
    pub fn file_name(&self) -> Option<&Path> {
        if self.inner.is_empty() {
            return None;
        }
        match self.inner.iter().rposition(|&b| b == b'/') {
            Some(i) if i + 1 == self.inner.len() => None,
            Some(i) => Some(Path::new(&self.inner[i + 1..])),
            None => Some(self),
        }
    }

    /// Everything before the final component.
    pub fn parent(&self) -> Option<&Path> {
        let i = self.inner.iter().rposition(|&b| b == b'/')?;
        if i == 0 {
            return Some(Path::new(b"/"));
        }
        Some(Path::new(&self.inner[..i]))
    }

    /// The extension of the final component, without the dot.
    ///
    /// A leading dot is a hidden file, not an extension - `.bashrc` has no extension - which is
    /// the one rule here that is not obvious from the name.
    pub fn extension(&self) -> Option<&Path> {
        let name = self.file_name()?.as_bytes();
        let i = name.iter().rposition(|&b| b == b'.')?;
        if i == 0 { None } else { Some(Path::new(&name[i + 1..])) }
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
            && &self.inner[..p.len()] == p
            && (self.inner.len() == p.len() || self.inner[p.len()] == b'/' || p.ends_with(b"/"))
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
        Ok(Path::new(rest.strip_prefix(b"/").unwrap_or(rest)))
    }

    /// The `/`-separated parts, with empties dropped.
    ///
    /// Not `std`'s `components()`: this does not resolve `.` or `..`, because resolving them
    /// without touching the filesystem is wrong in the presence of symlinks and every caller here
    /// that needs a real answer calls `canonicalize` instead.
    pub fn parts(&self) -> impl Iterator<Item = &[u8]> {
        self.inner.split(|&b| b == b'/').filter(|s| !s.is_empty())
    }

    /// The final component without its extension.
    pub fn file_stem(&self) -> Option<&Path> {
        let name = self.file_name()?.as_bytes();
        match name.iter().rposition(|&b| b == b'.') {
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
        match self.to_str() {
            Some(s) => Cow::Borrowed(s),
            None => Cow::Owned({
                let mut out = String::new();
                for chunk in self.inner.utf8_chunks() {
                    out.push_str(chunk.valid());
                    if !chunk.invalid().is_empty() {
                        out.push('\u{fffd}');
                    }
                }
                out
            }),
        }
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

    /// This path and each of its parents, longest first.
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
        match self.path.to_str() {
            Some(s) => f.write_str(s),
            None => {
                for chunk in self.path.as_bytes().utf8_chunks() {
                    f.write_str(chunk.valid())?;
                    if !chunk.invalid().is_empty() {
                        f.write_str("\u{fffd}")?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl fmt::Debug for Display<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.path.to_str().unwrap_or("<non-utf8 path>"))
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
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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
        let mut rest = self.rest;
        while rest.first() == Some(&b'/') {
            rest = &rest[1..];
        }
        Path::new(rest)
    }
}

impl<'a> Iterator for Components<'a> {
    type Item = Component<'a>;
    fn next(&mut self) -> Option<Component<'a>> {
        if self.root {
            self.root = false;
            self.rest = self.rest.strip_prefix(b"/").unwrap_or(self.rest);
            return Some(Component::RootDir);
        }
        while self.rest.first() == Some(&b'/') {
            self.rest = &self.rest[1..];
        }
        if self.rest.is_empty() {
            return None;
        }
        let end = self.rest.iter().position(|&b| b == b'/').unwrap_or(self.rest.len());
        let (part, rest) = self.rest.split_at(end);
        self.rest = rest;
        Some(match part {
            b"." => Component::CurDir,
            b".." => Component::ParentDir,
            _ => Component::Normal(part),
        })
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
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathBuf {
    inner: Vec<u8>,
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
        if !self.inner.is_empty() && !self.inner.ends_with(b"/") {
            self.inner.push(b'/');
        }
        self.inner.extend_from_slice(o);
    }

    /// Drop the final component. `false` if there was nothing to drop.
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

    /// Replace the final component.
    pub fn set_file_name(&mut self, name: impl AsRef<Path>) {
        self.pop();
        self.push(name);
    }

    /// Replace the extension of the final component.
    pub fn set_extension(&mut self, ext: &str) {
        let name_start = self.inner.iter().rposition(|&b| b == b'/').map_or(0, |i| i + 1);
        if let Some(dot) = self.inner[name_start..].iter().rposition(|&b| b == b'.') {
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
        let bytes: alloc::sync::Arc<[u8]> = alloc::sync::Arc::from(p.into_bytes());
        // Safe: `Path` is `repr(transparent)` over `[u8]`, so the pointer and its length
        // metadata are identical and the cast preserves both.
        unsafe { alloc::sync::Arc::from_raw(alloc::sync::Arc::into_raw(bytes) as *const Path) }
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
        let bytes: Box<[u8]> = p.into_bytes().into_boxed_slice();
        // Safe: `Path` is `repr(transparent)` over `[u8]`, so pointer and length metadata match.
        unsafe { Box::from_raw(Box::into_raw(bytes) as *mut Path) }
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
