pub use crate::str::ParseCharError;

pub const fn from_u32(i: u32) -> Option<char> {
    char::from_u32(i)
}

pub const unsafe fn from_u32_unchecked(i: u32) -> char {
    unsafe { char::from_u32_unchecked(i) }
}

impl char {
    pub const MAX: char = '\u{10ffff}';

    /// The most bytes a character takes in UTF-8.
    pub const MAX_LEN_UTF8: usize = 4;

    pub const fn from_u32(i: u32) -> Option<char> {
        if i <= 0x10ffff && !(i >= 0xd800 && i <= 0xdfff) {
            Some(unsafe { Self::from_u32_unchecked(i) })
        } else { None }
    }

    pub const unsafe fn from_u32_unchecked(i: u32) -> char { unsafe { crate::intrinsics::transmute(i) } }

    pub const fn to_digit(self, radix: u32) -> Option<u32> {
        // If not a digit, a number greater than radix will be created.
        let mut digit = (self as u32).wrapping_sub('0' as u32);
        if radix > 10 {
            if radix > 36 { panic!("to_digit: invalid radix -- radix must be in the range 2 to 36 inclusive"); }
            if digit < 10 { return Some(digit); }
            // Force the 6th bit to be set to ensure ascii is lower case.
            digit = (self as u32 | 0b10_0000).wrapping_sub('a' as u32).saturating_add(10);
        }
        if digit < radix { Some(digit) } else { None }
    }

    pub const fn is_control(self) -> bool {
        let n = self as u32;
        n < 0x20 || (n >= 0x7f && n < 0xa0)
    }

    pub const fn len_utf8(self) -> usize {
        let n = self as u32;
        if n <= 0x7f { 1 } else if n <= 0x7ff { 2 } else if n <= 0xffff { 3 } else { 4 }
    }

    pub const fn encode_utf8(self, dst: &mut [u8]) -> &mut str {
        let n = self as u32;
        let len = self.len_utf8();
        let _ = dst[len - 1];
        if len == 1 { dst[0] = n as u8; }
        else if len == 2 {
            dst[0] = (0xc0 | (n >> 6)) as u8; dst[1] = (0x80 | (n & 0x3f)) as u8;
        } else if len == 3 {
            dst[0] = (0xe0 | (n >> 12)) as u8; dst[1] = (0x80 | ((n >> 6) & 0x3f)) as u8;
            dst[2] = (0x80 | (n & 0x3f)) as u8;
        } else {
            dst[0] = (0xf0 | (n >> 18)) as u8; dst[1] = (0x80 | ((n >> 12) & 0x3f)) as u8;
            dst[2] = (0x80 | ((n >> 6) & 0x3f)) as u8; dst[3] = (0x80 | (n & 0x3f)) as u8;
        }
        let ptr = dst.as_mut_ptr();
        unsafe { crate::str::from_utf8_unchecked_mut(crate::slice::from_raw_parts_mut(ptr, len)) }
    }

    pub const fn is_ascii(&self) -> bool { *self as u32 <= 0x7f }
    pub const fn is_ascii_uppercase(&self) -> bool { *self >= 'A' && *self <= 'Z' }
    pub const fn is_ascii_digit(&self) -> bool { *self >= '0' && *self <= '9' }
    pub const fn is_ascii_alphabetic(&self) -> bool {
        (*self >= 'a' && *self <= 'z') || (*self >= 'A' && *self <= 'Z')
    }
    pub const fn is_ascii_alphanumeric(&self) -> bool { self.is_ascii_alphabetic() || self.is_ascii_digit() }
    pub const fn is_ascii_whitespace(&self) -> bool {
        *self == ' ' || (*self >= '\t' && *self <= '\r')
    }
    pub const fn is_whitespace(self) -> bool {
        matches!(self, '\u{9}'..='\u{d}' | '\u{20}' | '\u{85}' | '\u{a0}' | '\u{1680}' |
            '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}')
    }
}

/// The escape of one character as `char::escape_debug` writes it: at most `\u{10ffff}`, eleven characters.
#[derive(Clone, Debug)]
pub struct EscapeDebug { buf: [char; 11], pos: usize, len: usize }

impl EscapeDebug {
    fn one(c: char) -> Self { let mut buf = ['\0'; 11]; buf[0] = c; EscapeDebug { buf, pos: 0, len: 1 } }
    fn backslash(c: char) -> Self { let mut buf = ['\0'; 11]; buf[0] = '\\'; buf[1] = c; EscapeDebug { buf, pos: 0, len: 2 } }
    fn unicode(c: char) -> Self {
        let mut buf = ['\0'; 11];
        buf[0] = '\\'; buf[1] = 'u'; buf[2] = '{';
        let value = c as u32;
        let mut digits = 1;
        while digits < 6 && (value >> (4 * digits)) != 0 { digits += 1; }
        let mut len = 3;
        let mut shift = digits;
        while shift > 0 {
            shift -= 1;
            let d = (value >> (4 * shift)) & 0xf;
            buf[len] = if d < 10 { (b'0' + d as u8) as char } else { (b'a' + d as u8 - 10) as char };
            len += 1;
        }
        buf[len] = '}';
        EscapeDebug { buf, pos: 0, len: len + 1 }
    }
}

impl crate::iter::Iterator for EscapeDebug {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        if self.pos == self.len { return None; }
        let c = self.buf[self.pos];
        self.pos += 1;
        Some(c)
    }
    fn size_hint(&self) -> (usize, Option<usize>) { let n = self.len - self.pos; (n, Some(n)) }
}
impl crate::iter::ExactSizeIterator for EscapeDebug {}

impl crate::fmt::Display for EscapeDebug {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        let mut i = self.pos;
        while i < self.len { f.write_char(self.buf[i])?; i += 1; }
        Ok(())
    }
}

impl char {
    /// `Co`: the set never changes, so it is written out as Rust does.
    const fn is_private_use(self) -> bool {
        matches!(self, '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{FFFFD}' | '\u{100000}'..='\u{10FFFD}')
    }

    fn is_assigned(self) -> bool {
        match self {
            '\0'..='\u{377}' => true,
            '\u{378}'..='\u{3FFFD}' => !crate::unicode::cn_planes_0_3::lookup(self),
            '\u{E0001}' | '\u{E0020}'..='\u{E007F}' | '\u{E0100}'..='\u{E01EF}'
            | '\u{F0000}'..='\u{FFFFD}' | '\u{100000}'..='\u{10FFFD}' => true,
            _ => false,
        }
    }

    /// The Rust algorithm (`char::escape_debug_ext` with every quote escaped), over the Rust tables.
    pub fn escape_debug(self) -> EscapeDebug {
        match self {
            '"' => EscapeDebug::backslash('"'),
            '\'' => EscapeDebug::backslash('\''),
            '\\' => EscapeDebug::backslash('\\'),
            '\n' => EscapeDebug::backslash('n'),
            '\t' => EscapeDebug::backslash('t'),
            '\r' => EscapeDebug::backslash('r'),
            '\0' => EscapeDebug::backslash('0'),
            // ASCII fast path, plus the two halfwidth katakana marks, which are grapheme extenders that
            // are not escaped.
            '\x20'..='\x7E' | '\u{FF9E}' | '\u{FF9F}' => EscapeDebug::one(self),
            _ if self.is_control()
                || self.is_private_use()
                || self.is_whitespace()
                || (self > '\u{02FF}' && crate::unicode::grapheme_extend::lookup(self))
                || (self > '\u{AC}' && crate::unicode::default_ignorable_code_point::lookup(self))
                || (self > '\u{AC}' && crate::unicode::cf::lookup(self))
                || !self.is_assigned() => EscapeDebug::unicode(self),
            _ => EscapeDebug::one(self),
        }
    }
}

/// The uppercase mapping of one character: one, two or three characters.
#[derive(Clone, Debug)]
pub struct ToUppercase { chars: [char; 3], pos: usize, len: usize }

impl crate::iter::Iterator for ToUppercase {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        if self.pos == self.len { return None; }
        let c = self.chars[self.pos];
        self.pos += 1;
        Some(c)
    }
    fn size_hint(&self) -> (usize, Option<usize>) { let n = self.len - self.pos; (n, Some(n)) }
}
impl crate::iter::ExactSizeIterator for ToUppercase {}

impl crate::fmt::Display for ToUppercase {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        let mut i = self.pos;
        while i < self.len { f.write_char(self.chars[i])?; i += 1; }
        Ok(())
    }
}

impl char {
    /// The Rust table (`unicode_data::conversions::to_upper`), including the multi-character mappings.
    pub fn to_uppercase(self) -> ToUppercase {
        let [a, b, c] = crate::unicode::conversions::to_upper(self);
        let len = if b == '\0' { 1 } else if c == '\0' { 2 } else { 3 };
        ToUppercase { chars: [a, b, c], pos: 0, len }
    }

    /// Whether the character is a digit in the given radix (2 to 36); panics for another radix, as `to_digit`.
    pub const fn is_digit(self, radix: u32) -> bool {
        match self.to_digit(radix) { Some(_) => true, None => false }
    }
}

impl char {
    pub const fn to_ascii_uppercase(&self) -> char {
        if *self >= 'a' && *self <= 'z' { ((*self as u8) - 32) as char } else { *self }
    }
}
