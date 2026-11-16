use crate::vec::Vec;
use core::str::{self, Utf8Error};

pub struct String {
    vec: Vec<u8>,
}

pub struct FromUtf8Error {
    bytes: Vec<u8>,
    error: Utf8Error,
}

impl String {
    pub const fn new() -> Self {
        Self { vec: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self { vec: Vec::with_capacity(capacity) }
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    pub fn as_bytes(&self) -> &[u8] { self.vec.as_slice() }

    /// Removes the last character and returns it; `None` when empty.
    pub fn pop(&mut self) -> Option<char> {
        let ch = self.as_str().chars().next_back()?;
        let length = self.vec.len() - ch.len_utf8();
        // SAFETY: the characters before `ch` are whole, so the bytes stay valid UTF-8.
        unsafe { self.vec.set_len(length); }
        Some(ch)
    }

    pub fn as_str(&self) -> &str {
        // String only admits bytes through validated &str inputs or checked UTF-8 conversion.
        unsafe { str::from_utf8_unchecked(&self.vec) }
    }

    pub unsafe fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        &mut self.vec
    }

    pub fn push_str(&mut self, value: &str) {
        self.vec.extend_from_slice(value.as_bytes());
    }

    pub fn push(&mut self, value: char) {
        let mut bytes = [0; 4];
        self.push_str(value.encode_utf8(&mut bytes));
    }

    pub fn clear(&mut self) {
        self.vec.clear();
    }

    pub fn truncate(&mut self, new_len: usize) {
        if new_len < self.len() {
            if !self.as_str().is_char_boundary(new_len) {
                panic!("String::truncate: new_len does not lie on a char boundary");
            }
            self.vec.truncate(new_len);
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.vec.reserve(additional);
    }

    pub fn from_utf8(vec: Vec<u8>) -> Result<Self, FromUtf8Error> {
        match str::from_utf8(&vec) {
            Ok(_) => Ok(Self { vec }),
            Err(error) => Err(FromUtf8Error { bytes: vec, error }),
        }
    }

    pub unsafe fn from_utf8_unchecked(vec: Vec<u8>) -> Self {
        Self { vec }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.vec
    }

    pub fn into_boxed_str(self) -> crate::boxed::Box<str> {
        let boxed = self.vec.into_boxed_slice();
        let raw = crate::boxed::Box::into_raw(boxed);
        // The allocation is unchanged; String's invariant guarantees valid UTF-8 bytes.
        unsafe { crate::boxed::Box::from_raw(raw as *mut str) }
    }
}

impl FromUtf8Error {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn utf8_error(&self) -> Utf8Error {
        self.error
    }
}

impl Default for String {
    fn default() -> Self {
        Self::new()
    }
}

impl core::ops::Deref for String {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl core::convert::From<&str> for String {
    fn from(value: &str) -> Self {
        let mut result = Self::with_capacity(value.len());
        result.push_str(value);
        result
    }
}

pub trait ToString { fn to_string(&self) -> String; }
impl<T: core::fmt::Display + ?Sized> ToString for T {
    fn to_string(&self) -> String {
        use core::fmt::Write;
        let mut value=String::new();
        let _=write!(&mut value, "{}", self);
        value
    }
}
impl core::fmt::Write for String { fn write_str(&mut self, s:&str)->core::fmt::Result { self.push_str(s); Ok(()) } }
impl core::fmt::Display for String { fn fmt(&self,f:&mut core::fmt::Formatter<'_>)->core::fmt::Result { core::fmt::Display::fmt(self.as_str(), f) } }
impl core::fmt::Debug for String { fn fmt(&self,f:&mut core::fmt::Formatter<'_>)->core::fmt::Result { core::fmt::Debug::fmt(self.as_str(),f) } }
impl PartialEq for String { fn eq(&self, other:&Self)->bool { self.as_str()==other.as_str() } }
impl Eq for String {}
impl PartialOrd for String { fn partial_cmp(&self,other:&Self)->Option<core::cmp::Ordering>{Some(self.cmp(other))} }
impl Ord for String { fn cmp(&self,other:&Self)->core::cmp::Ordering { self.as_str().cmp(other.as_str()) } }
impl core::hash::Hash for String { fn hash<H:core::hash::Hasher>(&self,state:&mut H){self.as_str().hash(state)} }
impl Clone for String { fn clone(&self)->Self { Self::from(self.as_str()) } }
impl AsRef<str> for String { fn as_ref(&self)->&str { self.as_str() } }
impl AsRef<[u8]> for String { fn as_ref(&self)->&[u8] { &self.vec } }
impl core::borrow::Borrow<str> for String { fn borrow(&self)->&str { self.as_str() } }
impl core::ops::Add<&str> for String { type Output=String; fn add(mut self,s:&str)->String { self.push_str(s); self } }
impl core::ops::AddAssign<&str> for String { fn add_assign(&mut self,s:&str){self.push_str(s)} }
impl core::iter::FromIterator<char> for String { fn from_iter<I:IntoIterator<Item=char>>(iter:I)->Self { let mut s=Self::new(); for c in iter{s.push(c)} s } }
impl<'a> core::iter::FromIterator<&'a str> for String { fn from_iter<I:IntoIterator<Item=&'a str>>(iter:I)->Self { let mut s=Self::new(); for p in iter{s.push_str(p)} s } }
impl core::iter::FromIterator<String> for String { fn from_iter<I:IntoIterator<Item=String>>(iter:I)->Self { let mut s=Self::new(); for p in iter{s.push_str(p.as_str())} s } }

impl String {
    /// The text of `bytes`, with each run that is not UTF-8 replaced by U+FFFD. Borrows when
    /// nothing had to be replaced.
    pub fn from_utf8_lossy(bytes: &[u8]) -> crate::borrow::Cow<'_, str> {
        let mut error = match core::str::from_utf8(bytes) {
            Ok(text) => return crate::borrow::Cow::Borrowed(text),
            Err(error) => error,
        };
        let mut rest = bytes;
        let mut out = String::with_capacity(bytes.len());
        loop {
            let valid = error.valid_up_to();
            // SAFETY: The decoder reported this prefix as valid.
            out.push_str(unsafe { core::str::from_utf8_unchecked(&rest[..valid]) });
            out.push('\u{fffd}');
            // No length means the input ends inside a character: all that is left is invalid.
            let invalid = match error.error_len() {
                Some(len) => len,
                None => rest.len() - valid,
            };
            rest = &rest[valid + invalid..];
            match core::str::from_utf8(rest) {
                Ok(text) => {
                    out.push_str(text);
                    return crate::borrow::Cow::Owned(out);
                }
                Err(next) => error = next,
            }
        }
    }
}

impl PartialEq<str> for String { fn eq(&self, other: &str) -> bool { self.as_str() == other } }
impl PartialEq<&str> for String { fn eq(&self, other: &&str) -> bool { self.as_str() == *other } }
impl PartialEq<String> for str { fn eq(&self, other: &String) -> bool { self == other.as_str() } }
impl PartialEq<String> for &str { fn eq(&self, other: &String) -> bool { *self == other.as_str() } }

impl String {
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        self.vec.try_reserve_exact(additional)
    }
}
impl<'a> core::iter::FromIterator<&'a char> for String { fn from_iter<I: IntoIterator<Item = &'a char>>(iter: I) -> Self { let mut s = Self::new(); for c in iter { s.push(*c) } s } }
impl core::iter::Extend<char> for String { fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) { for c in iter { self.push(c) } } }
impl From<&String> for String { fn from(s: &String) -> String { s.clone() } }

impl core::str::pattern::Pattern for &String {
    fn find_in(&mut self, haystack: &str, from: usize) -> Option<(usize, usize)> { self.as_str().find_in(haystack, from) }
    fn prefix_len(&mut self, haystack: &str) -> Option<usize> { self.as_str().prefix_len(haystack) }
    fn suffix_len(&mut self, haystack: &str) -> Option<usize> { self.as_str().suffix_len(haystack) }
}

impl String {
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), crate::collections::TryReserveError> {
        self.vec.try_reserve(additional)
    }
}

impl String {
    /// Lowers the capacity to `max(len, min_capacity)` when that is below the current one.
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.vec.shrink_to(min_capacity)
    }
}

/// The characters `String::drain` removed (taken out when it is called).
pub struct Drain { removed: String, front: usize, back: usize }
impl Iterator for Drain {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        let ch = self.removed[self.front..self.back].chars().next()?;
        self.front += ch.len_utf8();
        Some(ch)
    }
}
impl core::iter::DoubleEndedIterator for Drain {
    fn next_back(&mut self) -> Option<char> {
        let ch = self.removed[self.front..self.back].chars().next_back()?;
        self.back -= ch.len_utf8();
        Some(ch)
    }
}

impl String {
    /// Removes the characters in `range`, which the iterator yields.
    pub fn drain<R: core::ops::RangeBounds<usize>>(&mut self, range: R) -> Drain {
        let start = match range.start_bound() {
            core::ops::Bound::Included(&n) => n,
            core::ops::Bound::Excluded(&n) => n + 1,
            core::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            core::ops::Bound::Included(&n) => n + 1,
            core::ops::Bound::Excluded(&n) => n,
            core::ops::Bound::Unbounded => self.len(),
        };
        assert!(self.as_str().is_char_boundary(start));
        assert!(self.as_str().is_char_boundary(end));
        let removed = String::from(&self[start..end]);
        self.vec.drain(start..end);
        let len = removed.len();
        Drain { removed, front: 0, back: len }
    }

    pub fn insert(&mut self, idx: usize, ch: char) {
        let mut buffer = [0u8; 4];
        self.insert_str(idx, ch.encode_utf8(&mut buffer));
    }

    pub fn remove(&mut self, idx: usize) -> char {
        let ch = match self[idx..].chars().next() {
            Some(ch) => ch,
            None => panic!("cannot remove a char from the end of a string"),
        };
        self.vec.drain(idx..idx + ch.len_utf8());
        ch
    }

    pub fn insert_str(&mut self, idx: usize, string: &str) {
        assert!(self.as_str().is_char_boundary(idx));
        self.vec.splice(idx..idx, string.bytes());
    }

    pub fn replace_range<R: core::ops::RangeBounds<usize>>(&mut self, range: R, replace_with: &str) {
        let start = match range.start_bound() {
            core::ops::Bound::Included(&n) => n,
            core::ops::Bound::Excluded(&n) => n + 1,
            core::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            core::ops::Bound::Included(&n) => n + 1,
            core::ops::Bound::Excluded(&n) => n,
            core::ops::Bound::Unbounded => self.len(),
        };
        assert!(self.as_str().is_char_boundary(start));
        assert!(self.as_str().is_char_boundary(end));
        self.vec.splice(start..end, replace_with.bytes());
    }
}
impl core::ops::DerefMut for String {
    fn deref_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}
impl AsMut<str> for String { fn as_mut(&mut self) -> &mut str { self.as_mut_str() } }
impl core::borrow::BorrowMut<str> for String { fn borrow_mut(&mut self) -> &mut str { self.as_mut_str() } }

impl String {
    /// The text as a mutable `str`.
    pub fn as_mut_str(&mut self) -> &mut str {
        let length = self.vec.len();
        // SAFETY: the first `length` bytes of the buffer are the string, valid UTF-8 by String's invariant, and
        // `&mut str` only allows changes that keep them valid UTF-8.
        unsafe { str::from_utf8_unchecked_mut(core::slice::from_raw_parts_mut(self.vec.as_mut_ptr(), length)) }
    }

    /// Keeps only the characters for which `keep` returns true, in order. One pass moves the kept bytes forward
    /// in place; nothing is allocated.
    pub fn retain<F: FnMut(char) -> bool>(&mut self, mut keep: F) {
        let length = self.vec.len();
        let base = self.vec.as_mut_ptr();
        // A panic in `keep` leaves the string empty instead of half moved.
        // SAFETY: zero bytes are always valid UTF-8.
        unsafe { self.vec.set_len(0); }
        let mut read = 0;
        let mut write = 0;
        while read < length {
            // SAFETY: bytes from `read` on have not been moved yet, so they are still whole UTF-8 characters.
            let rest = unsafe { str::from_utf8_unchecked(core::slice::from_raw_parts(base.add(read), length - read)) };
            let ch = match rest.chars().next() {
                Some(ch) => ch,
                None => break,
            };
            let width = ch.len_utf8();
            if keep(ch) {
                if write != read {
                    // SAFETY: both ranges lie inside the buffer; `copy` allows them to overlap.
                    unsafe { core::ptr::copy(base.add(read), base.add(write), width); }
                }
                write += width;
            }
            read += width;
        }
        // SAFETY: the first `write` bytes are the kept characters, whole and in their order.
        unsafe { self.vec.set_len(write); }
    }

    /// Splits the string at byte `at`: `self` keeps `[0, at)` and the rest is returned. Panics if `at` is past the
    /// end or not on a character boundary.
    pub fn split_off(&mut self, at: usize) -> String {
        if !self.as_str().is_char_boundary(at) {
            panic!("String::split_off: at does not lie on a char boundary");
        }
        let other = String::from(&self.as_str()[at..]);
        self.vec.truncate(at);
        other
    }

    /// Shrinks the capacity to the length.
    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit();
    }

    /// Gives up ownership and returns the text, which stays allocated for the rest of the program.
    pub fn leak<'a>(self) -> &'a mut str {
        let bytes = self.vec.leak();
        // SAFETY: the bytes are the string's, valid UTF-8 by String's invariant.
        unsafe { str::from_utf8_unchecked_mut(bytes) }
    }

    /// Decodes UTF-16 `v`; a lone surrogate is an error.
    pub fn from_utf16(v: &[u16]) -> Result<String, FromUtf16Error> {
        let mut out = String::with_capacity(v.len());
        let mut i = 0;
        while i < v.len() {
            let (ch, units) = decode_utf16_at(v, i);
            match ch {
                Some(ch) => out.push(ch),
                None => return Err(FromUtf16Error(())),
            }
            i += units;
        }
        Ok(out)
    }

    /// Decodes UTF-16 `v`, with each lone surrogate replaced by U+FFFD.
    pub fn from_utf16_lossy(v: &[u16]) -> String {
        let mut out = String::with_capacity(v.len());
        let mut i = 0;
        while i < v.len() {
            let (ch, units) = decode_utf16_at(v, i);
            out.push(match ch {
                Some(ch) => ch,
                None => '\u{fffd}',
            });
            i += units;
        }
        out
    }
}

/// The character that starts at `v[i]` and how many units it takes: two for a surrogate pair, else one. `None`
/// for a lone surrogate.
fn decode_utf16_at(v: &[u16], i: usize) -> (Option<char>, usize) {
    let unit = v[i] as u32;
    if unit < 0xD800 || unit >= 0xE000 {
        return (core::char::from_u32(unit), 1);
    }
    if unit < 0xDC00 {
        if let Some(&next) = v.get(i + 1) {
            let low = next as u32;
            if low >= 0xDC00 && low < 0xE000 {
                return (core::char::from_u32(0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00)), 2);
            }
        }
    }
    (None, 1)
}

/// The error `String::from_utf16` returns: the input held a lone surrogate.
pub struct FromUtf16Error(());

impl core::fmt::Debug for FromUtf16Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("FromUtf16Error(())")
    }
}
impl core::fmt::Display for FromUtf16Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("invalid utf-16: lone surrogate found")
    }
}

impl core::fmt::Debug for FromUtf8Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FromUtf8Error").field("bytes", &self.bytes).field("error", &self.error).finish()
    }
}
impl core::fmt::Display for FromUtf8Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.error, f)
    }
}
impl Clone for FromUtf8Error {
    fn clone(&self) -> Self {
        FromUtf8Error { bytes: self.bytes.clone(), error: self.error }
    }
}
impl PartialEq for FromUtf8Error {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes && self.error == other.error
    }
}
impl Eq for FromUtf8Error {}

impl core::str::FromStr for String {
    type Err = core::convert::Infallible;
    fn from_str(s: &str) -> Result<String, Self::Err> {
        Ok(String::from(s))
    }
}

impl From<char> for String {
    fn from(c: char) -> String {
        let mut s = String::new();
        s.push(c);
        s
    }
}
impl From<crate::boxed::Box<str>> for String {
    fn from(s: crate::boxed::Box<str>) -> String {
        let raw = crate::boxed::Box::into_raw(s) as *mut [u8];
        // SAFETY: `str` and `[u8]` have the same layout, so the allocation is handed over unchanged; its bytes are
        // valid UTF-8 because they were a `str`.
        let bytes = unsafe { crate::boxed::Box::from_raw(raw) };
        String { vec: crate::boxed::Box::into_vec(bytes) }
    }
}
impl<'a> From<crate::borrow::Cow<'a, str>> for String {
    fn from(s: crate::borrow::Cow<'a, str>) -> String {
        s.into_owned()
    }
}
impl From<&mut str> for String {
    fn from(s: &mut str) -> String {
        String::from(&*s)
    }
}
impl From<String> for crate::boxed::Box<str> {
    fn from(s: String) -> crate::boxed::Box<str> {
        s.into_boxed_str()
    }
}

impl<'a> core::iter::Extend<&'a str> for String {
    fn extend<I: IntoIterator<Item = &'a str>>(&mut self, iter: I) {
        for s in iter { self.push_str(s) }
    }
}
impl core::iter::Extend<String> for String {
    fn extend<I: IntoIterator<Item = String>>(&mut self, iter: I) {
        for s in iter { self.push_str(&s) }
    }
}
impl<'a> core::iter::Extend<&'a char> for String {
    fn extend<I: IntoIterator<Item = &'a char>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        for c in iter { self.push(*c) }
    }
}
impl<'a> core::iter::Extend<crate::borrow::Cow<'a, str>> for String {
    fn extend<I: IntoIterator<Item = crate::borrow::Cow<'a, str>>>(&mut self, iter: I) {
        for s in iter { self.push_str(&s) }
    }
}
impl core::iter::Extend<crate::boxed::Box<str>> for String {
    fn extend<I: IntoIterator<Item = crate::boxed::Box<str>>>(&mut self, iter: I) {
        for s in iter { self.push_str(&s) }
    }
}

impl<'a> PartialEq<crate::borrow::Cow<'a, str>> for String {
    fn eq(&self, other: &crate::borrow::Cow<'a, str>) -> bool { self.as_str() == &**other }
}
impl<'a> PartialEq<String> for crate::borrow::Cow<'a, str> {
    fn eq(&self, other: &String) -> bool { &**self == other.as_str() }
}
impl From<String> for Vec<u8> { fn from(string: String) -> Vec<u8> { string.vec } }
impl From<&str> for Vec<u8> { fn from(string: &str) -> Vec<u8> { Vec::from(string.as_bytes()) } }

impl<'a> core::iter::FromIterator<crate::borrow::Cow<'a, str>> for String {
    fn from_iter<I: IntoIterator<Item = crate::borrow::Cow<'a, str>>>(iter: I) -> String {
        let mut out = String::new();
        for piece in iter { out.push_str(&piece); }
        out
    }
}
