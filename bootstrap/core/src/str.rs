use crate::clone::Clone;
use crate::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use crate::intrinsics;
use crate::marker::Copy;
use crate::ops::{Index, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use crate::option::Option::{self, None, Some};
use crate::result::Result::{self, Err, Ok};
use crate::slice::{self, SliceIndex};

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
    pub fn starts_with(&self, pat: &str) -> bool {
        self.as_bytes().starts_with(pat.as_bytes())
    }

    #[inline(always)]
    pub fn ends_with(&self, pat: &str) -> bool {
        self.as_bytes().ends_with(pat.as_bytes())
    }

    #[inline(always)]
    pub fn strip_prefix(&self, pat: &str) -> Option<&str> {
        if self.starts_with(pat) {
            Some(&self[pat.len()..])
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn strip_suffix(&self, pat: &str) -> Option<&str> {
        if self.ends_with(pat) {
            Some(&self[..self.len() - pat.len()])
        } else {
            None
        }
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
