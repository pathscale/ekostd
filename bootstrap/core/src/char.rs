impl char {
    pub const MAX: char = '\u{10ffff}';

    pub const fn from_u32(i: u32) -> Option<char> {
        if i <= 0x10ffff && !(i >= 0xd800 && i <= 0xdfff) {
            Some(unsafe { Self::from_u32_unchecked(i) })
        } else { None }
    }

    pub const unsafe fn from_u32_unchecked(i: u32) -> char { unsafe { crate::intrinsics::transmute(i) } }

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
