//! Escaping of single bytes as ASCII.

use crate::fmt;
use crate::iter::Iterator;
use crate::option::Option::{self, None, Some};

/// The escaped form of a byte, as an iterator over its bytes.
#[derive(Clone)]
pub struct EscapeDefault {
    data: [u8; 4],
    start: u8,
    end: u8,
}

/// The bytes that `b` is written as in a byte or string literal.
pub fn escape_default(c: u8) -> EscapeDefault {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let (data, len) = match c {
        b'\t' => ([b'\\', b't', 0, 0], 2),
        b'\r' => ([b'\\', b'r', 0, 0], 2),
        b'\n' => ([b'\\', b'n', 0, 0], 2),
        b'\\' => ([b'\\', b'\\', 0, 0], 2),
        b'\'' => ([b'\\', b'\'', 0, 0], 2),
        b'"' => ([b'\\', b'"', 0, 0], 2),
        0x20..=0x7e => ([c, 0, 0, 0], 1),
        _ => ([b'\\', b'x', HEX[(c >> 4) as usize], HEX[(c & 0xf) as usize]], 4),
    };
    EscapeDefault { data, start: 0, end: len }
}

impl Iterator for EscapeDefault {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        if self.start == self.end { return None; }
        let byte = self.data[self.start as usize];
        self.start += 1;
        Some(byte)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.end - self.start) as usize;
        (n, Some(n))
    }
}

impl fmt::Display for EscapeDefault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut i = self.start;
        while i < self.end {
            f.write_char(self.data[i as usize] as char)?;
            i += 1;
        }
        Ok(())
    }
}
