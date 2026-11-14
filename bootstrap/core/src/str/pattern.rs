//! What a string search looks for: a string, a character, or a test applied to each character.

use crate::ops::FnMut;
use crate::option::Option::{self, None, Some};

pub trait Pattern: Sized {
    /// The byte range of the first match that starts at or after `from`. `from` is a character
    /// boundary, and so are both ends of the range.
    fn find_in(&mut self, haystack: &str, from: usize) -> Option<(usize, usize)>;
    /// The length of the match the haystack starts with.
    fn prefix_len(&mut self, haystack: &str) -> Option<usize>;
    /// The length of the match the haystack ends with.
    fn suffix_len(&mut self, haystack: &str) -> Option<usize>;
}

impl Pattern for &str {
    fn find_in(&mut self, haystack: &str, from: usize) -> Option<(usize, usize)> {
        // UTF-8 is self-synchronising, so a byte-for-byte match of a whole string starts and
        // ends on character boundaries.
        let (haystack, needle) = (haystack.as_bytes(), self.as_bytes());
        let mut at = from;
        while at + needle.len() <= haystack.len() {
            if haystack[at..at + needle.len()] == *needle {
                return Some((at, at + needle.len()));
            }
            at += 1;
        }
        None
    }
    fn prefix_len(&mut self, haystack: &str) -> Option<usize> {
        if haystack.as_bytes().starts_with(self.as_bytes()) { Some(self.len()) } else { None }
    }
    fn suffix_len(&mut self, haystack: &str) -> Option<usize> {
        if haystack.as_bytes().ends_with(self.as_bytes()) { Some(self.len()) } else { None }
    }
}

impl Pattern for char {
    fn find_in(&mut self, haystack: &str, from: usize) -> Option<(usize, usize)> {
        let mut buffer = [0u8; 4];
        let mut encoded: &str = self.encode_utf8(&mut buffer);
        encoded.find_in(haystack, from)
    }
    fn prefix_len(&mut self, haystack: &str) -> Option<usize> {
        let mut buffer = [0u8; 4];
        let mut encoded: &str = self.encode_utf8(&mut buffer);
        encoded.prefix_len(haystack)
    }
    fn suffix_len(&mut self, haystack: &str) -> Option<usize> {
        let mut buffer = [0u8; 4];
        let mut encoded: &str = self.encode_utf8(&mut buffer);
        encoded.suffix_len(haystack)
    }
}

impl<F: FnMut(char) -> bool> Pattern for F {
    fn find_in(&mut self, haystack: &str, from: usize) -> Option<(usize, usize)> {
        let bytes = haystack.as_bytes();
        let mut at = from;
        while at < bytes.len() {
            let (character, len) = super::decode(bytes, at);
            if (*self)(character) {
                return Some((at, at + len));
            }
            at += len;
        }
        None
    }
    fn prefix_len(&mut self, haystack: &str) -> Option<usize> {
        if haystack.is_empty() {
            return None;
        }
        let (character, len) = super::decode(haystack.as_bytes(), 0);
        if (*self)(character) { Some(len) } else { None }
    }
    fn suffix_len(&mut self, haystack: &str) -> Option<usize> {
        if haystack.is_empty() {
            return None;
        }
        let bytes = haystack.as_bytes();
        // Step back over continuation bytes to the start of the last character.
        let mut at = bytes.len() - 1;
        while at > 0 && bytes[at] & 0xc0 == 0x80 {
            at -= 1;
        }
        let (character, len) = super::decode(bytes, at);
        if (*self)(character) { Some(len) } else { None }
    }
}

impl Pattern for &[char] {
    fn find_in(&mut self, haystack: &str, from: usize) -> Option<(usize, usize)> {
        (|c: char| self.contains(&c)).find_in(haystack, from)
    }
    fn prefix_len(&mut self, haystack: &str) -> Option<usize> {
        (|c: char| self.contains(&c)).prefix_len(haystack)
    }
    fn suffix_len(&mut self, haystack: &str) -> Option<usize> {
        (|c: char| self.contains(&c)).suffix_len(haystack)
    }
}

impl<const N: usize> Pattern for &[char; N] {
    fn find_in(&mut self, haystack: &str, from: usize) -> Option<(usize, usize)> {
        self.as_slice().find_in(haystack, from)
    }
    fn prefix_len(&mut self, haystack: &str) -> Option<usize> {
        self.as_slice().prefix_len(haystack)
    }
    fn suffix_len(&mut self, haystack: &str) -> Option<usize> {
        self.as_slice().suffix_len(haystack)
    }
}

impl<const N: usize> Pattern for [char; N] {
    fn find_in(&mut self, haystack: &str, from: usize) -> Option<(usize, usize)> {
        self.as_slice().find_in(haystack, from)
    }
    fn prefix_len(&mut self, haystack: &str) -> Option<usize> {
        self.as_slice().prefix_len(haystack)
    }
    fn suffix_len(&mut self, haystack: &str) -> Option<usize> {
        self.as_slice().suffix_len(haystack)
    }
}
