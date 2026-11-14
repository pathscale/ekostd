use crate::clone::Clone;
use crate::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use crate::intrinsics;
use crate::marker::Copy;
use crate::ops::{Index, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use crate::option::Option::{self, None, Some};
use crate::result::Result::{self, Err, Ok};
use crate::slice::{self, SliceIndex};

pub mod pattern;
use self::pattern::Pattern;

pub struct Utf8Error {
    valid_up_to: usize,
    error_len: Option<u8>,
}

impl Copy for Utf8Error {}
impl Clone for Utf8Error {
    #[inline(always)]
    fn clone(&self) -> Self { *self }
}
impl PartialEq for Utf8Error {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.valid_up_to == other.valid_up_to && self.error_len == other.error_len
    }
}
impl Eq for Utf8Error {}

impl Utf8Error {
    #[inline(always)]
    pub const fn valid_up_to(&self) -> usize {
        self.valid_up_to
    }

    #[inline(always)]
    pub const fn error_len(&self) -> Option<usize> {
        match self.error_len {
            Some(len) => Some(len as usize),
            None => None,
        }
    }
}

#[inline(always)]
pub const unsafe fn from_utf8_unchecked(v: &[u8]) -> &str {
    unsafe { intrinsics::transmute(v) }
}

#[inline(always)]
pub const unsafe fn from_utf8_unchecked_mut(v: &mut [u8]) -> &mut str {
    unsafe { intrinsics::transmute(v) }
}

pub const fn from_utf8(v: &[u8]) -> Result<&str, Utf8Error> {
    let mut i = 0;
    let len = v.len();
    while i < len {
        let b0 = v[i];
        if b0 < 0x80 {
            i += 1;
        } else if b0 < 0xC2 {
            return Err(Utf8Error { valid_up_to: i, error_len: Some(1) });
        } else if b0 < 0xE0 {
            if i + 1 >= len {
                return Err(Utf8Error { valid_up_to: i, error_len: None });
            }
            let b1 = v[i + 1];
            if (b1 & 0xC0) != 0x80 {
                return Err(Utf8Error { valid_up_to: i, error_len: Some(1) });
            }
            i += 2;
        } else if b0 < 0xF0 {
            if i + 1 >= len {
                return Err(Utf8Error { valid_up_to: i, error_len: None });
            }
            let b1 = v[i + 1];
            if (b1 & 0xC0) != 0x80
                || (b0 == 0xE0 && b1 < 0xA0)
                || (b0 == 0xED && b1 >= 0xA0)
            {
                return Err(Utf8Error { valid_up_to: i, error_len: Some(1) });
            }
            if i + 2 >= len {
                return Err(Utf8Error { valid_up_to: i, error_len: None });
            }
            let b2 = v[i + 2];
            if (b2 & 0xC0) != 0x80 {
                return Err(Utf8Error { valid_up_to: i, error_len: Some(2) });
            }
            i += 3;
        } else if b0 < 0xF5 {
            if i + 1 >= len {
                return Err(Utf8Error { valid_up_to: i, error_len: None });
            }
            let b1 = v[i + 1];
            if (b1 & 0xC0) != 0x80
                || (b0 == 0xF0 && b1 < 0x90)
                || (b0 == 0xF4 && b1 >= 0x90)
            {
                return Err(Utf8Error { valid_up_to: i, error_len: Some(1) });
            }
            if i + 2 >= len {
                return Err(Utf8Error { valid_up_to: i, error_len: None });
            }
            let b2 = v[i + 2];
            if (b2 & 0xC0) != 0x80 {
                return Err(Utf8Error { valid_up_to: i, error_len: Some(2) });
            }
            if i + 3 >= len {
                return Err(Utf8Error { valid_up_to: i, error_len: None });
            }
            let b3 = v[i + 3];
            if (b3 & 0xC0) != 0x80 {
                return Err(Utf8Error { valid_up_to: i, error_len: Some(3) });
            }
            i += 4;
        } else {
            return Err(Utf8Error { valid_up_to: i, error_len: Some(1) });
        }
    }
    unsafe { Ok(from_utf8_unchecked(v)) }
}

impl str {
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.as_bytes().len()
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub const fn as_bytes(&self) -> &[u8] {
        unsafe { intrinsics::transmute(self) }
    }

    #[inline(always)]
    pub const unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { intrinsics::transmute(self) }
    }

    #[inline(always)]
    pub const fn as_ptr(&self) -> *const u8 {
        self as *const str as *const u8
    }

    #[inline(always)]
    pub const fn as_mut_ptr(&mut self) -> *mut u8 {
        self as *mut str as *mut u8
    }

    #[inline(always)]
    pub const fn as_str(&self) -> &str {
        self
    }

    #[inline(always)]
    pub const fn is_char_boundary(&self, index: usize) -> bool {
        if index == 0 {
            return true;
        }
        let bytes = self.as_bytes();
        if index >= bytes.len() {
            index == bytes.len()
        } else {
            (bytes[index] as i8) >= -0x40
        }
    }

    #[inline(always)]
    pub fn split_at(&self, mid: usize) -> (&str, &str) {
        if !self.is_char_boundary(mid) {
            intrinsics::abort();
        }
        let (a, b) = self.as_bytes().split_at(mid);
        unsafe { (from_utf8_unchecked(a), from_utf8_unchecked(b)) }
    }

    #[inline(always)]
    pub fn starts_with<P: Pattern>(&self, mut pat: P) -> bool {
        pat.prefix_len(self).is_some()
    }

    #[inline(always)]
    pub fn ends_with<P: Pattern>(&self, mut pat: P) -> bool {
        pat.suffix_len(self).is_some()
    }

    #[inline(always)]
    pub fn strip_prefix<P: Pattern>(&self, mut pat: P) -> Option<&str> {
        match pat.prefix_len(self) {
            Some(len) => Some(&self[len..]),
            None => None,
        }
    }

    #[inline(always)]
    pub fn strip_suffix<P: Pattern>(&self, mut pat: P) -> Option<&str> {
        match pat.suffix_len(self) {
            Some(len) => Some(&self[..self.len() - len]),
            None => None,
        }
    }

    /// As the free function `from_utf8`.
    pub const fn from_utf8(v: &[u8]) -> Result<&str, Utf8Error> {
        from_utf8(v)
    }

    /// The byte offset of the first match.
    pub fn find<P: Pattern>(&self, mut pat: P) -> Option<usize> {
        match pat.find_in(self, 0) {
            Some((start, _)) => Some(start),
            None => None,
        }
    }

    pub fn contains<P: Pattern>(&self, mut pat: P) -> bool {
        pat.find_in(self, 0).is_some()
    }

    /// The pieces between matches. A pattern that matches nothing at all, such as an empty string,
    /// ends the splitting: what is left comes out as the last piece.
    pub fn split<P: Pattern>(&self, pat: P) -> Split<'_, P> {
        Split { haystack: self, start: 0, finished: false, pattern: pat }
    }

    /// As `split`, with each match kept at the end of the piece before it. A match at the very end
    /// is not followed by an empty piece.
    pub fn split_inclusive<P: Pattern>(&self, pat: P) -> SplitInclusive<'_, P> {
        SplitInclusive { haystack: self, start: 0, finished: false, pattern: pat }
    }

    /// What lies before the first match, and what lies after it.
    pub fn split_once<P: Pattern>(&self, mut pat: P) -> Option<(&str, &str)> {
        match pat.find_in(self, 0) {
            Some((start, end)) => Some((&self[..start], &self[end..])),
            None => None,
        }
    }

    /// What is left after removing every match from the end.
    pub fn trim_end_matches<P: Pattern>(&self, mut pat: P) -> &str {
        let mut rest = self;
        while let Some(len) = pat.suffix_len(rest) {
            if len == 0 {
                break;
            }
            rest = &rest[..rest.len() - len];
        }
        rest
    }

    pub fn chars(&self) -> Chars<'_> {
        Chars { rest: self }
    }

    /// The lines, without their `\n` or `\r\n` endings.
    pub fn lines(&self) -> Lines<'_> {
        Lines { inner: self.split_inclusive('\n') }
    }

    #[inline(always)]
    pub fn trim(&self) -> &str {
        self.trim_start().trim_end()
    }

    #[inline(always)]
    pub fn trim_start(&self) -> &str {
        let bytes = self.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                i += 1;
            } else {
                break;
            }
        }
        unsafe { from_utf8_unchecked(&bytes[i..]) }
    }

    #[inline(always)]
    pub fn trim_end(&self) -> &str {
        let bytes = self.as_bytes();
        let mut end = bytes.len();
        while end > 0 {
            let b = bytes[end - 1];
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                end -= 1;
            } else {
                break;
            }
        }
        unsafe { from_utf8_unchecked(&bytes[..end]) }
    }

    #[inline(always)]
    pub fn bytes(&self) -> slice::Iter<'_, u8> {
        self.as_bytes().iter()
    }
}

unsafe impl SliceIndex<str> for Range<usize> {
    type Output = str;

    #[inline(always)]
    fn get(self, slice: &str) -> Option<&str> {
        if self.start <= self.end
            && slice.is_char_boundary(self.start)
            && slice.is_char_boundary(self.end)
        {
            unsafe { Some(&*self.get_unchecked(slice)) }
        } else {
            None
        }
    }

    #[inline(always)]
    fn get_mut(self, slice: &mut str) -> Option<&mut str> {
        if self.start <= self.end
            && slice.is_char_boundary(self.start)
            && slice.is_char_boundary(self.end)
        {
            unsafe { Some(&mut *self.get_unchecked_mut(slice)) }
        } else {
            None
        }
    }

    #[inline(always)]
    unsafe fn get_unchecked(self, slice: *const str) -> *const str {
        let s = unsafe { &*slice };
        unsafe { from_utf8_unchecked(s.as_bytes().get_unchecked(self)) as *const str }
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(self, slice: *mut str) -> *mut str {
        let s = unsafe { &mut *slice };
        unsafe { from_utf8_unchecked_mut(s.as_bytes_mut().get_unchecked_mut(self)) as *mut str }
    }

    #[inline(always)]
    fn index(self, slice: &str) -> &str {
        match self.get(slice) {
            Some(s) => s,
            None => intrinsics::abort(),
        }
    }

    #[inline(always)]
    fn index_mut(self, slice: &mut str) -> &mut str {
        match self.get_mut(slice) {
            Some(s) => s,
            None => intrinsics::abort(),
        }
    }
}

unsafe impl SliceIndex<str> for RangeTo<usize> {
    type Output = str;

    #[inline(always)]
    fn get(self, slice: &str) -> Option<&str> {
        (0..self.end).get(slice)
    }

    #[inline(always)]
    fn get_mut(self, slice: &mut str) -> Option<&mut str> {
        (0..self.end).get_mut(slice)
    }

    #[inline(always)]
    unsafe fn get_unchecked(self, slice: *const str) -> *const str {
        unsafe { (0..self.end).get_unchecked(slice) }
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(self, slice: *mut str) -> *mut str {
        unsafe { (0..self.end).get_unchecked_mut(slice) }
    }

    #[inline(always)]
    fn index(self, slice: &str) -> &str {
        (0..self.end).index(slice)
    }

    #[inline(always)]
    fn index_mut(self, slice: &mut str) -> &mut str {
        (0..self.end).index_mut(slice)
    }
}

unsafe impl SliceIndex<str> for RangeFrom<usize> {
    type Output = str;

    #[inline(always)]
    fn get(self, slice: &str) -> Option<&str> {
        (self.start..slice.len()).get(slice)
    }

    #[inline(always)]
    fn get_mut(self, slice: &mut str) -> Option<&mut str> {
        (self.start..slice.len()).get_mut(slice)
    }

    #[inline(always)]
    unsafe fn get_unchecked(self, slice: *const str) -> *const str {
        let len = unsafe { (*slice).len() };
        unsafe { (self.start..len).get_unchecked(slice) }
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(self, slice: *mut str) -> *mut str {
        let len = unsafe { (*slice).len() };
        unsafe { (self.start..len).get_unchecked_mut(slice) }
    }

    #[inline(always)]
    fn index(self, slice: &str) -> &str {
        (self.start..slice.len()).index(slice)
    }

    #[inline(always)]
    fn index_mut(self, slice: &mut str) -> &mut str {
        let len = slice.len();
        (self.start..len).index_mut(slice)
    }
}

unsafe impl SliceIndex<str> for RangeFull {
    type Output = str;

    #[inline(always)]
    fn get(self, slice: &str) -> Option<&str> {
        Some(slice)
    }

    #[inline(always)]
    fn get_mut(self, slice: &mut str) -> Option<&mut str> {
        Some(slice)
    }

    #[inline(always)]
    unsafe fn get_unchecked(self, slice: *const str) -> *const str {
        slice
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(self, slice: *mut str) -> *mut str {
        slice
    }

    #[inline(always)]
    fn index(self, slice: &str) -> &str {
        slice
    }

    #[inline(always)]
    fn index_mut(self, slice: &mut str) -> &mut str {
        slice
    }
}

unsafe impl SliceIndex<str> for RangeInclusive<usize> {
    type Output = str;

    #[inline(always)]
    fn get(self, slice: &str) -> Option<&str> {
        if *self.end() == usize::MAX {
            None
        } else {
            (*self.start()..*self.end() + 1).get(slice)
        }
    }

    #[inline(always)]
    fn get_mut(self, slice: &mut str) -> Option<&mut str> {
        if *self.end() == usize::MAX {
            None
        } else {
            (*self.start()..*self.end() + 1).get_mut(slice)
        }
    }

    #[inline(always)]
    unsafe fn get_unchecked(self, slice: *const str) -> *const str {
        unsafe { (*self.start()..*self.end() + 1).get_unchecked(slice) }
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(self, slice: *mut str) -> *mut str {
        unsafe { (*self.start()..*self.end() + 1).get_unchecked_mut(slice) }
    }

    #[inline(always)]
    fn index(self, slice: &str) -> &str {
        match self.get(slice) {
            Some(s) => s,
            None => intrinsics::abort(),
        }
    }

    #[inline(always)]
    fn index_mut(self, slice: &mut str) -> &mut str {
        match self.get_mut(slice) {
            Some(s) => s,
            None => intrinsics::abort(),
        }
    }
}

unsafe impl SliceIndex<str> for RangeToInclusive<usize> {
    type Output = str;

    #[inline(always)]
    fn get(self, slice: &str) -> Option<&str> {
        (0..=self.end).get(slice)
    }

    #[inline(always)]
    fn get_mut(self, slice: &mut str) -> Option<&mut str> {
        (0..=self.end).get_mut(slice)
    }

    #[inline(always)]
    unsafe fn get_unchecked(self, slice: *const str) -> *const str {
        unsafe { (0..=self.end).get_unchecked(slice) }
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(self, slice: *mut str) -> *mut str {
        unsafe { (0..=self.end).get_unchecked_mut(slice) }
    }

    #[inline(always)]
    fn index(self, slice: &str) -> &str {
        (0..=self.end).index(slice)
    }

    #[inline(always)]
    fn index_mut(self, slice: &mut str) -> &mut str {
        (0..=self.end).index_mut(slice)
    }
}

impl<I> Index<I> for str
where
    I: SliceIndex<str>,
{
    type Output = I::Output;

    #[inline(always)]
    fn index(&self, index: I) -> &I::Output {
        index.index(self)
    }
}

impl PartialEq for str {
    #[inline(always)]
    fn eq(&self, other: &str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}
impl Eq for str {}

impl PartialOrd for str {
    #[inline(always)]
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

impl Ord for str {
    #[inline(always)]
    fn cmp(&self, other: &str) -> Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

/// Parsing a value from a string. `str::parse` goes through this.
pub trait FromStr: crate::marker::Sized {
    type Err;
    fn from_str(s: &str) -> Result<Self, Self::Err>;
}
macro_rules! from_str_int {
    ($($t:ty)*) => {$(
        impl FromStr for $t {
            type Err = crate::num::ParseIntError;
            fn from_str(s: &str) -> Result<Self, Self::Err> { <$t>::from_str_radix(s, 10) }
        }
    )*};
}
from_str_int!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ParseBoolError;

impl crate::fmt::Debug for ParseBoolError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("ParseBoolError")
    }
}

impl crate::fmt::Display for ParseBoolError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("provided string was not `true` or `false`")
    }
}

impl FromStr for bool {
    type Err = ParseBoolError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(ParseBoolError),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ParseCharError {
    _private: (),
}

impl crate::fmt::Debug for ParseCharError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("ParseCharError")
    }
}

impl crate::fmt::Display for ParseCharError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("failed to parse char")
    }
}

impl FromStr for char {
    type Err = ParseCharError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Ok(c),
            _ => Err(ParseCharError { _private: () }),
        }
    }
}

impl str {
    pub fn parse<F: FromStr>(&self) -> Result<F, F::Err> { F::from_str(self) }
    /// Equal when ASCII letters are compared without regard to case.
    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {
        let (a, b) = (self.as_bytes(), other.as_bytes());
        if a.len() != b.len() { return false; }
        let mut i = 0;
        while i < a.len() {
            if !a[i].eq_ignore_ascii_case(&b[i]) { return false; }
            i += 1;
        }
        true
    }
}

/// The character that starts at `at`, and how many bytes encode it. `bytes` is valid UTF-8 and
/// `at` a character boundary inside it.
pub(crate) fn decode(bytes: &[u8], at: usize) -> (char, usize) {
    let first = bytes[at] as u32;
    let continuation = |offset: usize| (bytes[at + offset] & 0x3f) as u32;
    let (value, len) = if first < 0x80 {
        (first, 1)
    } else if first < 0xe0 {
        ((first & 0x1f) << 6 | continuation(1), 2)
    } else if first < 0xf0 {
        ((first & 0x0f) << 12 | continuation(1) << 6 | continuation(2), 3)
    } else {
        ((first & 0x07) << 18 | continuation(1) << 12 | continuation(2) << 6 | continuation(3), 4)
    };
    // SAFETY: Valid UTF-8 encodes only Unicode scalar values.
    (unsafe { char::from_u32_unchecked(value) }, len)
}

pub struct Chars<'a> {
    rest: &'a str,
}
impl<'a> Chars<'a> {
    /// What has not been yielded yet.
    pub fn as_str(&self) -> &'a str { self.rest }
}
impl Clone for Chars<'_> {
    fn clone(&self) -> Self { Chars { rest: self.rest } }
}
impl crate::iter::Iterator for Chars<'_> {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        if self.rest.is_empty() {
            return None;
        }
        let (character, len) = decode(self.rest.as_bytes(), 0);
        self.rest = &self.rest[len..];
        Some(character)
    }
}

pub struct Split<'a, P> {
    haystack: &'a str,
    start: usize,
    finished: bool,
    pattern: P,
}
impl<'a, P: Pattern> crate::iter::Iterator for Split<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.finished {
            return None;
        }
        match self.pattern.find_in(self.haystack, self.start) {
            Some((start, end)) if end > start => {
                let piece = &self.haystack[self.start..start];
                self.start = end;
                Some(piece)
            }
            _ => {
                self.finished = true;
                Some(&self.haystack[self.start..])
            }
        }
    }
}

pub struct SplitInclusive<'a, P> {
    haystack: &'a str,
    start: usize,
    finished: bool,
    pattern: P,
}
impl<'a, P: Pattern> crate::iter::Iterator for SplitInclusive<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.finished || self.start == self.haystack.len() {
            return None;
        }
        match self.pattern.find_in(self.haystack, self.start) {
            Some((start, end)) if end > start => {
                let piece = &self.haystack[self.start..end];
                self.start = end;
                Some(piece)
            }
            _ => {
                self.finished = true;
                Some(&self.haystack[self.start..])
            }
        }
    }
}

pub struct Lines<'a> {
    inner: SplitInclusive<'a, char>,
}
impl<'a> crate::iter::Iterator for Lines<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        let line = match self.inner.next() {
            Some(line) => line,
            None => return None,
        };
        let line = match line.strip_suffix('\n') {
            Some(line) => line,
            None => return Some(line),
        };
        Some(match line.strip_suffix('\r') {
            Some(line) => line,
            None => line,
        })
    }
}

impl crate::iter::DoubleEndedIterator for Chars<'_> {
    fn next_back(&mut self) -> Option<char> {
        if self.rest.is_empty() {
            return None;
        }
        let bytes = self.rest.as_bytes();
        // Step back over continuation bytes to the start of the last character.
        let mut at = bytes.len() - 1;
        while at > 0 && bytes[at] & 0xc0 == 0x80 {
            at -= 1;
        }
        let (character, _) = decode(bytes, at);
        self.rest = &self.rest[..at];
        Some(character)
    }
}

impl crate::fmt::Debug for Utf8Error {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.debug_struct("Utf8Error").field("valid_up_to", &self.valid_up_to).field("error_len", &self.error_len).finish()
    }
}
impl crate::fmt::Display for Utf8Error {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        match self.error_len {
            Some(_) => crate::write!(f, "invalid utf-8 sequence starting at index {}", self.valid_up_to),
            None => crate::write!(f, "incomplete utf-8 byte sequence from index {}", self.valid_up_to),
        }
    }
}
