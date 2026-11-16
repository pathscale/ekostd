use crate::clone::Clone;
use crate::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use crate::intrinsics;
use crate::marker::Copy;
use crate::ops::{Index, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use crate::option::Option::{self, None, Some};
use crate::result::Result::{self, Err, Ok};
use crate::slice::{self, SliceIndex};

pub mod pattern;
mod dec2flt;
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
        Split { haystack: self, start: 0, end: self.len(), finished: false, pattern: pat }
    }

    /// As `split`, with each match kept at the end of the piece before it. A match at the very end
    /// is not followed by an empty piece.
    pub fn split_inclusive<P: Pattern>(&self, pat: P) -> SplitInclusive<'_, P> {
        SplitInclusive { haystack: self, start: 0, end: self.len(), finished: false, pattern: pat }
    }

    /// What lies before the first match, and what lies after it.
    pub fn split_once<P: Pattern>(&self, mut pat: P) -> Option<(&str, &str)> {
        match pat.find_in(self, 0) {
            Some((start, end)) => Some((&self[..start], &self[end..])),
            None => None,
        }
    }

    /// As `split`, but a last piece that is empty is left out.
    pub fn split_terminator<P: Pattern>(&self, pat: P) -> SplitTerminator<'_, P> {
        SplitTerminator { inner: self.split(pat) }
    }

    pub fn get<I: SliceIndex<str>>(&self, i: I) -> Option<&I::Output> { i.get(self) }

    pub fn matches<P: Pattern>(&self, pat: P) -> Matches<'_, P> { Matches { inner: self.match_indices(pat) } }

    pub fn rsplit_once<P: Pattern>(&self, mut pat: P) -> Option<(&str, &str)> {
        let mut i = self.len();
        loop {
            if self.is_char_boundary(i) {
                if let Some(len) = pat.prefix_len(&self[i..]) {
                    if len > 0 { return Some((&self[..i], &self[i + len..])); }
                }
            }
            if i == 0 { return None; }
            i -= 1;
        }
    }

    pub fn rsplit<P: Pattern>(&self, pat: P) -> RSplit<'_, P> { RSplit { haystack: self, end: self.len(), pattern: pat, done: false } }

    /// At most `n` pieces, cut at the last matches; the first piece is the rest.
    pub fn rsplitn<P: Pattern>(&self, n: usize, pat: P) -> RSplitN<'_, P> {
        RSplitN { inner: self.rsplit(pat), count: n }
    }

    pub fn splitn<P: Pattern>(&self, n: usize, pat: P) -> SplitN<'_, P> { SplitN { inner: self.split(pat), count: n } }

    pub fn escape_default(&self) -> EscapeDefault<'_> {
        EscapeDefault { chars: self.chars(), buf: ['\0'; 10], pos: 0, len: 0 }
    }

    pub fn is_ascii(&self) -> bool { self.as_bytes().is_ascii() }

    /// The byte index of the last match.
    pub fn rfind<P: Pattern>(&self, mut pat: P) -> Option<usize> {
        let mut i = self.len();
        loop {
            if self.is_char_boundary(i) && pat.prefix_len(&self[i..]).is_some() { return Some(i); }
            if i == 0 { return None; }
            i -= 1;
        }
    }

    /// The matches, left to right and not overlapping, with the byte index each starts at.
    /// The matches from the right, with the byte index each starts at.
    pub fn rmatch_indices<P: Pattern>(&self, pat: P) -> crate::iter::Rev<MatchIndices<'_, P>> {
        self.match_indices(pat).rev()
    }

    pub fn match_indices<P: Pattern>(&self, pat: P) -> MatchIndices<'_, P> {
        MatchIndices { haystack: self, pattern: pat, pos: 0, back: self.len(), done: false }
    }

    /// The pieces between runs of whitespace, with no empty pieces.
    pub fn split_whitespace(&self) -> SplitWhitespace<'_> {
        SplitWhitespace { rest: self }
    }

    /// What is left after removing every match from both ends.
    pub fn trim_matches<P: Pattern>(&self, mut pat: P) -> &str {
        let mut rest = self;
        while let Some(len) = pat.prefix_len(rest) {
            if len == 0 { break; }
            rest = &rest[len..];
        }
        while let Some(len) = pat.suffix_len(rest) {
            if len == 0 { break; }
            rest = &rest[..rest.len() - len];
        }
        rest
    }

    /// What is left after removing every match from the start.
    pub fn trim_start_matches<P: Pattern>(&self, mut pat: P) -> &str {
        let mut rest = self;
        while let Some(len) = pat.prefix_len(rest) {
            if len == 0 { break; }
            rest = &rest[len..];
        }
        rest
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

    pub fn char_indices(&self) -> CharIndices<'_> {
        CharIndices { front_offset: 0, iter: Chars { rest: self } }
    }

    /// The lines, without their `\n` or `\r\n` endings.
    pub fn lines(&self) -> Lines<'_> {
        Lines { inner: self.split_inclusive('\n') }
    }

    #[inline(always)]
    pub fn trim(&self) -> &str {
        self.trim_start().trim_end()
    }

    /// Without the leading chars that are `char::is_whitespace` (Unicode White_Space).
    #[inline(always)]
    pub fn trim_start(&self) -> &str {
        let bytes = self.as_bytes();
        let mut it = self.char_indices();
        while let Some((at, c)) = crate::iter::Iterator::next(&mut it) {
            if !c.is_whitespace() {
                // SAFETY: `at` is the start of a char, so the tail is still UTF-8.
                return unsafe { from_utf8_unchecked(&bytes[at..]) };
            }
        }
        unsafe { from_utf8_unchecked(&bytes[bytes.len()..]) }
    }

    /// Without the trailing chars that are `char::is_whitespace` (Unicode White_Space).
    #[inline(always)]
    pub fn trim_end(&self) -> &str {
        let bytes = self.as_bytes();
        let mut it = self.chars();
        while let Some(c) = crate::iter::DoubleEndedIterator::next_back(&mut it) {
            if !c.is_whitespace() {
                let end = it.as_str().len() + c.len_utf8();
                // SAFETY: `end` is the end of a char, so the head is still UTF-8.
                return unsafe { from_utf8_unchecked(&bytes[..end]) };
            }
        }
        unsafe { from_utf8_unchecked(&bytes[..0]) }
    }

    /// Without the ASCII whitespace at the start.
    pub const fn trim_ascii_start(&self) -> &str {
        // SAFETY: the cut is next to an ASCII byte, so the rest is still UTF-8.
        unsafe { from_utf8_unchecked(self.as_bytes().trim_ascii_start()) }
    }

    /// Without the ASCII whitespace at the end.
    pub const fn trim_ascii_end(&self) -> &str {
        // SAFETY: the cut is next to an ASCII byte, so the rest is still UTF-8.
        unsafe { from_utf8_unchecked(self.as_bytes().trim_ascii_end()) }
    }

    /// Without the ASCII whitespace at both ends.
    pub const fn trim_ascii(&self) -> &str {
        // SAFETY: both cuts are next to ASCII bytes, so the rest is still UTF-8.
        unsafe { from_utf8_unchecked(self.as_bytes().trim_ascii()) }
    }

    #[inline(always)]
    pub fn bytes(&self) -> crate::iter::Copied<slice::Iter<'_, u8>> {
        self.as_bytes().iter().copied()
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

pub use crate::num::ParseFloatError;

impl FromStr for f32 {
    type Err = ParseFloatError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        dec2flt::parse_f32(s)
    }
}
impl FromStr for f64 {
    type Err = ParseFloatError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        dec2flt::parse_f64(s)
    }
}

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
    end: usize,
    finished: bool,
    pattern: P,
}
impl<'a, P: Pattern> crate::iter::Iterator for Split<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.finished {
            return None;
        }
        match self.pattern.find_in(&self.haystack[..self.end], self.start) {
            Some((start, end)) if end > start => {
                let piece = &self.haystack[self.start..start];
                self.start = end;
                Some(piece)
            }
            _ => {
                self.finished = true;
                Some(&self.haystack[self.start..self.end])
            }
        }
    }
}
/// Pieces taken from the back are cut at the last non-empty matches, as in `rsplit`.
impl<'a, P: Pattern> crate::iter::DoubleEndedIterator for Split<'a, P> {
    fn next_back(&mut self) -> Option<&'a str> {
        if self.finished {
            return None;
        }
        let mut i = self.end;
        while i > self.start {
            i -= 1;
            if !self.haystack.is_char_boundary(i) { continue; }
            if let Some(len) = self.pattern.prefix_len(&self.haystack[i..self.end]) {
                if len > 0 {
                    let piece = &self.haystack[i + len..self.end];
                    self.end = i;
                    return Some(piece);
                }
            }
        }
        self.finished = true;
        Some(&self.haystack[self.start..self.end])
    }
}

pub struct SplitInclusive<'a, P> {
    haystack: &'a str,
    start: usize,
    end: usize,
    finished: bool,
    pattern: P,
}
impl<'a, P: Pattern> crate::iter::Iterator for SplitInclusive<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.finished || self.start == self.end {
            return None;
        }
        match self.pattern.find_in(&self.haystack[..self.end], self.start) {
            Some((start, end)) if end > start => {
                let piece = &self.haystack[self.start..end];
                self.start = end;
                Some(piece)
            }
            _ => {
                self.finished = true;
                Some(&self.haystack[self.start..self.end])
            }
        }
    }
}
impl<'a, P: Pattern> crate::iter::DoubleEndedIterator for SplitInclusive<'a, P> {
    fn next_back(&mut self) -> Option<&'a str> {
        if self.finished || self.start == self.end {
            return None;
        }
        // The last piece ends at `end`, terminator included; it starts after the match before that terminator.
        let mut limit = self.end;
        if let Some(len) = self.pattern.suffix_len(&self.haystack[self.start..self.end]) {
            limit -= len;
        }
        let mut i = limit;
        while i > self.start {
            i -= 1;
            if !self.haystack.is_char_boundary(i) { continue; }
            if let Some(len) = self.pattern.prefix_len(&self.haystack[i..limit]) {
                if len > 0 {
                    let piece = &self.haystack[i + len..self.end];
                    self.end = i + len;
                    return Some(piece);
                }
            }
        }
        self.finished = true;
        Some(&self.haystack[self.start..self.end])
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

impl<'a> crate::iter::DoubleEndedIterator for Lines<'a> {
    fn next_back(&mut self) -> Option<&'a str> {
        let line = self.inner.next_back()?;
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

impl crate::iter::DoubleEndedIterator for CharIndices<'_> {
    fn next_back(&mut self) -> Option<(usize, char)> {
        let ch = self.iter.next_back()?;
        Some((self.front_offset + self.iter.as_str().len(), ch))
    }
}

impl<'a> crate::iter::DoubleEndedIterator for SplitWhitespace<'a> {
    fn next_back(&mut self) -> Option<&'a str> {
        let rest = self.rest.trim_end_matches(|c: char| c.is_whitespace());
        if rest.is_empty() { self.rest = ""; return None; }
        let start = match rest.rfind(|c: char| c.is_whitespace()) {
            Some(at) => at + rest[at..].chars().next().map_or(1, |c| c.len_utf8()),
            None => 0,
        };
        self.rest = &rest[..start];
        Some(&rest[start..])
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

/// The characters of a string slice with the byte offset each starts at.
pub struct CharIndices<'a> {
    front_offset: usize,
    iter: Chars<'a>,
}
impl Clone for CharIndices<'_> {
    fn clone(&self) -> Self { CharIndices { front_offset: self.front_offset, iter: self.iter.clone() } }
}
impl crate::iter::Iterator for CharIndices<'_> {
    type Item = (usize, char);
    fn next(&mut self) -> Option<(usize, char)> {
        let before = self.iter.as_str().len();
        match self.iter.next() {
            None => None,
            Some(ch) => {
                let index = self.front_offset;
                self.front_offset += before - self.iter.as_str().len();
                Some((index, ch))
            }
        }
    }
}

pub struct SplitTerminator<'a, P> {
    inner: Split<'a, P>,
}
impl<'a, P: Pattern> crate::iter::Iterator for SplitTerminator<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        match self.inner.next() {
            Some(piece) if piece.is_empty() && self.inner.finished => None,
            other => other,
        }
    }
}

pub struct SplitWhitespace<'a> {
    rest: &'a str,
}
impl<'a> Clone for SplitWhitespace<'a> {
    fn clone(&self) -> Self { SplitWhitespace { rest: self.rest } }
}
impl<'a> crate::iter::Iterator for SplitWhitespace<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        let rest = self.rest;
        let start = match rest.char_indices().find(|&(_, c)| !c.is_whitespace()) {
            Some((start, _)) => start,
            None => { self.rest = ""; return None; }
        };
        let rest = &rest[start..];
        let end = match rest.char_indices().find(|&(_, c)| c.is_whitespace()) {
            Some((end, _)) => end,
            None => rest.len(),
        };
        self.rest = &rest[end..];
        Some(&rest[..end])
    }
}

pub struct MatchIndices<'a, P> {
    haystack: &'a str,
    pattern: P,
    pos: usize,
    back: usize,
    done: bool,
}
impl<'a, P: Pattern> crate::iter::Iterator for MatchIndices<'a, P> {
    type Item = (usize, &'a str);
    fn next(&mut self) -> Option<(usize, &'a str)> {
        if self.done { return None; }
        let haystack = &self.haystack[..self.back];
        match self.pattern.find_in(haystack, self.pos) {
            None => { self.done = true; None }
            Some((start, end)) => {
                if end > start {
                    self.pos = end;
                } else if end >= haystack.len() {
                    self.done = true;
                } else {
                    self.pos = end + haystack[end..].chars().next().map_or(1, |c| c.len_utf8());
                }
                Some((start, &self.haystack[start..end]))
            }
        }
    }
}
/// Matches taken from the back are the last non-empty matches, as in `rfind`.
impl<'a, P: Pattern> crate::iter::DoubleEndedIterator for MatchIndices<'a, P> {
    fn next_back(&mut self) -> Option<(usize, &'a str)> {
        if self.done { return None; }
        let mut i = self.back;
        while i > self.pos {
            i -= 1;
            if !self.haystack.is_char_boundary(i) { continue; }
            if let Some(len) = self.pattern.prefix_len(&self.haystack[i..self.back]) {
                if len > 0 {
                    let end = i + len;
                    self.back = i;
                    return Some((i, &self.haystack[i..end]));
                }
            }
        }
        self.done = true;
        None
    }
}

pub struct Matches<'a, P> { inner: MatchIndices<'a, P> }
impl<'a, P: Pattern> crate::iter::Iterator for Matches<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> { self.inner.next().map(|(_, piece)| piece) }
}
impl<'a, P: Pattern> crate::iter::DoubleEndedIterator for Matches<'a, P> {
    fn next_back(&mut self) -> Option<&'a str> { self.inner.next_back().map(|(_, piece)| piece) }
}

pub struct RSplit<'a, P> { haystack: &'a str, end: usize, pattern: P, done: bool }
impl<'a, P: Pattern> crate::iter::Iterator for RSplit<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.done { return None; }
        let mut i = self.end;
        while i > 0 {
            i -= 1;
            if !self.haystack.is_char_boundary(i) { continue; }
            if let Some(len) = self.pattern.prefix_len(&self.haystack[i..self.end]) {
                if len > 0 {
                    let piece = &self.haystack[i + len..self.end];
                    self.end = i;
                    return Some(piece);
                }
            }
        }
        self.done = true;
        Some(&self.haystack[..self.end])
    }
}

pub struct RSplitN<'a, P> { inner: RSplit<'a, P>, count: usize }
impl<'a, P: Pattern> crate::iter::Iterator for RSplitN<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        match self.count {
            0 => None,
            1 => {
                self.count = 0;
                if self.inner.done { None } else { self.inner.done = true; Some(&self.inner.haystack[..self.inner.end]) }
            }
            _ => { self.count -= 1; self.inner.next() }
        }
    }
}

pub struct SplitN<'a, P> { inner: Split<'a, P>, count: usize }
impl<'a, P: Pattern> crate::iter::Iterator for SplitN<'a, P> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        match self.count {
            0 => None,
            1 => {
                self.count = 0;
                if self.inner.finished { None } else { self.inner.finished = true; Some(&self.inner.haystack[self.inner.start..self.inner.end]) }
            }
            _ => { self.count -= 1; self.inner.next() }
        }
    }
}

/// The characters of a string with each escaped as `char::escape_default` does.
pub struct EscapeDefault<'a> { chars: Chars<'a>, buf: [char; 10], pos: usize, len: usize }
impl<'a> crate::iter::Iterator for EscapeDefault<'a> {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        if self.pos == self.len {
            let c = self.chars.next()?;
            self.pos = 0;
            self.len = 0;
            let mut push = |this: &mut Self, ch: char| { this.buf[this.len] = ch; this.len += 1; };
            match c {
                '\t' => { push(self, '\\'); push(self, 't'); }
                '\r' => { push(self, '\\'); push(self, 'r'); }
                '\n' => { push(self, '\\'); push(self, 'n'); }
                '\\' | '\'' | '"' => { push(self, '\\'); push(self, c); }
                '\u{20}'..='\u{7e}' => push(self, c),
                _ => {
                    push(self, '\\'); push(self, 'u'); push(self, '{');
                    let value = c as u32;
                    let mut digits = 1;
                    while digits < 6 && (value >> (4 * digits)) != 0 { digits += 1; }
                    let mut shift = digits;
                    while shift > 0 {
                        shift -= 1;
                        let d = (value >> (4 * shift)) & 0xf;
                        push(self, if d < 10 { (b'0' + d as u8) as char } else { (b'a' + d as u8 - 10) as char });
                    }
                    push(self, '}');
                }
            }
        }
        let c = self.buf[self.pos];
        self.pos += 1;
        Some(c)
    }
}
impl crate::fmt::Display for EscapeDefault<'_> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        let mut it = EscapeDefault { chars: self.chars.clone(), buf: self.buf, pos: self.pos, len: self.len };
        while let Some(c) = crate::iter::Iterator::next(&mut it) { f.write_char(c)?; }
        Ok(())
    }
}
