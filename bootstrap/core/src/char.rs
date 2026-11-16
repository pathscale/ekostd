pub use crate::str::ParseCharError;

pub const fn from_u32(i: u32) -> Option<char> {
    char::from_u32(i)
}

/// The highest valid code point.
pub const MAX: char = char::MAX;

/// U+FFFD, the character a decoder puts in place of invalid input.
pub const REPLACEMENT_CHARACTER: char = char::REPLACEMENT_CHARACTER;

/// The digit `num` in the given radix, lowercase letters from 10 on; panics for a radix outside 2 to 36.
pub const fn from_digit(num: u32, radix: u32) -> Option<char> {
    char::from_digit(num, radix)
}

pub const unsafe fn from_u32_unchecked(i: u32) -> char {
    unsafe { char::from_u32_unchecked(i) }
}

impl char {
    pub const MAX: char = '\u{10ffff}';

    /// The lowest valid code point.
    pub const MIN: char = '\0';

    /// U+FFFD, the character a decoder puts in place of invalid input.
    pub const REPLACEMENT_CHARACTER: char = '\u{FFFD}';

    /// The most UTF-16 code units a character takes.
    pub const MAX_LEN_UTF16: usize = 2;

    /// The digit `num` in the given radix, lowercase letters from 10 on; `None` when `num >= radix`.
    /// Panics for a radix outside 2 to 36.
    pub const fn from_digit(num: u32, radix: u32) -> Option<char> {
        if radix < 2 || radix > 36 { panic!("from_digit: radix is too high (maximum 36)"); }
        if num < radix {
            let num = num as u8;
            if num < 10 { Some((b'0' + num) as char) } else { Some((b'a' + num - 10) as char) }
        } else { None }
    }

    /// 1, or 2 (a surrogate pair) above U+FFFF.
    pub const fn len_utf16(self) -> usize {
        if (self as u32) <= 0xffff { 1 } else { 2 }
    }

    /// Writes the character as UTF-16 and returns the written part; panics when `dst` is too short.
    pub fn encode_utf16(self, dst: &mut [u16]) -> &mut [u16] {
        let code = self as u32;
        if code <= 0xffff {
            dst[0] = code as u16;
            &mut dst[..1]
        } else {
            let _ = dst[1];
            let rest = code - 0x1_0000;
            dst[0] = 0xd800 | ((rest >> 10) as u16);
            dst[1] = 0xdc00 | ((rest & 0x3ff) as u16);
            &mut dst[..2]
        }
    }

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
    /// Space, tab, line feed, form feed or carriage return; vertical tab (U+000B) is not included.
    pub const fn is_ascii_whitespace(&self) -> bool {
        matches!(*self, ' ' | '\t' | '\n' | '\x0C' | '\r')
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
        let (buf, len) = unicode_escape(c);
        EscapeDebug { buf, pos: 0, len }
    }
}

/// `\u{` + the lowercase hex digits of `c` without leading zeros + `}`, and how many characters that is.
fn unicode_escape(c: char) -> ([char; 11], usize) {
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
    (buf, len + 1)
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

/// The escape of one character as `char::escape_unicode` writes it: always `\u{...}`.
#[derive(Clone, Debug)]
pub struct EscapeUnicode { buf: [char; 11], pos: usize, len: usize }

impl crate::iter::Iterator for EscapeUnicode {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        if self.pos == self.len { return None; }
        let c = self.buf[self.pos];
        self.pos += 1;
        Some(c)
    }
    fn size_hint(&self) -> (usize, Option<usize>) { let n = self.len - self.pos; (n, Some(n)) }
}
impl crate::iter::ExactSizeIterator for EscapeUnicode {}
impl crate::iter::FusedIterator for EscapeUnicode {}

impl crate::fmt::Display for EscapeUnicode {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        let mut i = self.pos;
        while i < self.len { f.write_char(self.buf[i])?; i += 1; }
        Ok(())
    }
}

/// The escape of one character as `char::escape_default` writes it: the character itself,
/// a backslash pair, or `\u{...}`.
#[derive(Clone, Debug)]
pub struct EscapeDefault { buf: [char; 11], pos: usize, len: usize }

impl crate::iter::Iterator for EscapeDefault {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        if self.pos == self.len { return None; }
        let c = self.buf[self.pos];
        self.pos += 1;
        Some(c)
    }
    fn size_hint(&self) -> (usize, Option<usize>) { let n = self.len - self.pos; (n, Some(n)) }
}
impl crate::iter::ExactSizeIterator for EscapeDefault {}
impl crate::iter::FusedIterator for EscapeDefault {}

impl crate::fmt::Display for EscapeDefault {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        let mut i = self.pos;
        while i < self.len { f.write_char(self.buf[i])?; i += 1; }
        Ok(())
    }
}

impl char {
    /// Always `\u{` + the lowercase hex digits without leading zeros + `}`.
    pub fn escape_unicode(self) -> EscapeUnicode {
        let (buf, len) = unicode_escape(self);
        EscapeUnicode { buf, pos: 0, len }
    }

    /// Tab, carriage return, line feed, both quotes and backslash as a backslash pair, U+0020 to U+007E
    /// as themselves, everything else as `escape_unicode`.
    pub fn escape_default(self) -> EscapeDefault {
        let mut buf = ['\0'; 11];
        let len = match self {
            '\t' => { buf[0] = '\\'; buf[1] = 't'; 2 }
            '\r' => { buf[0] = '\\'; buf[1] = 'r'; 2 }
            '\n' => { buf[0] = '\\'; buf[1] = 'n'; 2 }
            '\\' | '\'' | '"' => { buf[0] = '\\'; buf[1] = self; 2 }
            '\x20'..='\x7e' => { buf[0] = self; 1 }
            _ => {
                let (unicode, len) = unicode_escape(self);
                buf = unicode;
                len
            }
        };
        EscapeDefault { buf, pos: 0, len }
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
    pub fn escape_debug(self) -> EscapeDebug { self.escape_debug_ext(true, true) }

    /// The Rust algorithm (`char::escape_debug_ext`): each quote is escaped only when its flag is set,
    /// as `Debug for char` (single quote) and `Debug for str` (double quote) need.
    pub(crate) fn escape_debug_ext(self, escape_single_quote: bool, escape_double_quote: bool) -> EscapeDebug {
        self.escape_debug_args(true, escape_single_quote, escape_double_quote)
    }

    /// The Rust algorithm with all three of Rust's switches: grapheme extenders are escaped only when
    /// `escape_grapheme_extended` is set, which `str::escape_debug` clears after the first character.
    pub(crate) fn escape_debug_args(self, escape_grapheme_extended: bool, escape_single_quote: bool,
        escape_double_quote: bool) -> EscapeDebug {
        match self {
            '"' => if escape_double_quote { EscapeDebug::backslash('"') } else { EscapeDebug::one(self) },
            '\'' => if escape_single_quote { EscapeDebug::backslash('\'') } else { EscapeDebug::one(self) },
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
                || (escape_grapheme_extended && self > '\u{02FF}' && crate::unicode::grapheme_extend::lookup(self))
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
    pub const fn eq_ignore_ascii_case(&self, other: &char) -> bool {
        self.to_ascii_lowercase() == other.to_ascii_lowercase()
    }
    pub const fn is_ascii_punctuation(&self) -> bool {
        matches!(*self, '!'..='/' | ':'..='@' | '['..='`' | '{'..='~')
    }
}

impl char {
    pub const fn to_ascii_uppercase(&self) -> char {
        if *self >= 'a' && *self <= 'z' { ((*self as u8) - 32) as char } else { *self }
    }
}

impl char {
    /// Rust's definition: ASCII letters, else the `Alphabetic` property.
    pub fn is_alphabetic(self) -> bool {
        match self {
            'a'..='z' | 'A'..='Z' => true,
            c => c > '\x7f' && crate::unicode::alphabetic::lookup(c),
        }
    }
    pub fn is_numeric(self) -> bool {
        match self {
            '0'..='9' => true,
            c => c > '\x7f' && crate::unicode::n::lookup(c),
        }
    }
    pub fn is_alphanumeric(self) -> bool { self.is_alphabetic() || self.is_numeric() }
}

/// The lowercase mapping of one character: one, two or three characters.
#[derive(Clone, Debug)]
pub struct ToLowercase { chars: [char; 3], pos: usize, len: usize }
impl crate::iter::Iterator for ToLowercase {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        if self.pos == self.len { return None; }
        let c = self.chars[self.pos];
        self.pos += 1;
        Some(c)
    }
    fn size_hint(&self) -> (usize, Option<usize>) { let n = self.len - self.pos; (n, Some(n)) }
}
impl crate::iter::ExactSizeIterator for ToLowercase {}
impl crate::fmt::Display for ToLowercase {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        let mut i = self.pos;
        while i < self.len { f.write_char(self.chars[i])?; i += 1; }
        Ok(())
    }
}
impl char {
    /// The Rust table (`unicode_data::conversions::to_lower`). The context-free mapping: no final sigma.
    pub fn to_lowercase(self) -> ToLowercase {
        let [a, b, c] = crate::unicode::conversions::to_lower(self);
        let len = if b == '\0' { 1 } else if c == '\0' { 2 } else { 3 };
        ToLowercase { chars: [a, b, c], pos: 0, len }
    }
    pub fn is_lowercase(self) -> bool {
        match self {
            'a'..='z' => true,
            '\0'..='\u{A9}' => false,
            _ => crate::unicode::lowercase::lookup(self),
        }
    }
    /// Rust: `Lowercase`, `Uppercase` or titlecase letter (`Lt`).
    pub fn is_cased(self) -> bool {
        match self {
            'a'..='z' | 'A'..='Z' => true,
            '\0'..='\u{A9}' => false,
            _ => crate::unicode::lowercase::lookup(self) || crate::unicode::uppercase::lookup(self) || crate::unicode::lt::lookup(self),
        }
    }
    /// Rust: the `Case_Ignorable` property (the final-sigma rule of `str::to_lowercase`).
    pub fn is_case_ignorable(self) -> bool {
        if self.is_ascii() {
            matches!(self, '\'' | '.' | ':' | '^' | '`')
        } else {
            crate::unicode::case_ignorable::lookup(self)
        }
    }
    pub fn is_uppercase(self) -> bool {
        match self {
            'A'..='Z' => true,
            c => c > '\x7f' && crate::unicode::uppercase::lookup(c),
        }
    }
    pub const fn is_ascii_hexdigit(&self) -> bool {
        matches!(*self, '0'..='9' | 'A'..='F' | 'a'..='f')
    }
    pub const fn is_ascii_lowercase(&self) -> bool {
        matches!(*self, 'a'..='z')
    }
    /// `!` to `~`: the visible ASCII characters, space excluded.
    pub const fn is_ascii_graphic(&self) -> bool {
        matches!(*self, '!'..='~')
    }
    /// U+0000 to U+001F and U+007F only.
    pub const fn is_ascii_control(&self) -> bool {
        matches!(*self, '\0'..='\x1F' | '\x7F')
    }
    pub const fn is_ascii_octdigit(&self) -> bool {
        matches!(*self, '0'..='7')
    }
    /// `a` to `z` become `A` to `Z`; every other character is left as it is.
    pub const fn make_ascii_uppercase(&mut self) {
        *self = self.to_ascii_uppercase();
    }
    /// `A` to `Z` become `a` to `z`; every other character is left as it is.
    pub const fn make_ascii_lowercase(&mut self) {
        *self = self.to_ascii_lowercase();
    }
}

impl char {
    pub const fn to_ascii_lowercase(&self) -> char {
        if *self >= 'A' && *self <= 'Z' { ((*self as u8) + 32) as char } else { *self }
    }
}
