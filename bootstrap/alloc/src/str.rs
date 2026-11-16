pub use core::str::*;

pub fn into_boxed_str(s: crate::string::String) -> crate::boxed::Box<str> {
    s.into_boxed_str()
}

impl str {
    /// A copy with every character replaced by its uppercase mapping (Unicode `Changes_When_Uppercased`, no context).
    #[rustc_allow_incoherent_impl]
    pub fn to_uppercase(&self) -> crate::string::String {
        let mut out = crate::string::String::with_capacity(self.len());
        for c in self.chars() {
            out.extend(c.to_uppercase());
        }
        out
    }

    /// A copy with every character replaced by its lowercase mapping, with Rust's final-sigma rule (Σ becomes ς at the
    /// end of a word, else σ).
    #[rustc_allow_incoherent_impl]
    pub fn to_lowercase(&self) -> crate::string::String {
        let mut out = crate::string::String::with_capacity(self.len());
        for (i, c) in self.char_indices() {
            if c == 'Σ' {
                out.push(map_uppercase_sigma(self, i));
            } else {
                out.extend(c.to_lowercase());
            }
        }
        out
    }

    /// A copy with the ASCII capitals made lower case; other characters are unchanged.
    #[rustc_allow_incoherent_impl]
    pub fn to_ascii_lowercase(&self) -> crate::string::String {
        let mut bytes = self.as_bytes().to_vec();
        for byte in bytes.iter_mut() {
            *byte = byte.to_ascii_lowercase();
        }
        // SAFETY: Only ASCII bytes changed, each to another ASCII byte.
        unsafe { crate::string::String::from_utf8_unchecked(bytes) }
    }
}

impl str {
    /// A new string of this one repeated `n` times.
    #[rustc_allow_incoherent_impl]
    pub fn repeat(&self, n: usize) -> crate::string::String {
        let mut out = crate::string::String::with_capacity(self.len().checked_mul(n).expect("capacity overflow"));
        let mut i = 0;
        while i < n {
            out.push_str(self);
            i += 1;
        }
        out
    }
}

impl str {
    /// A copy with the ASCII lower-case letters made capitals; other characters are unchanged.
    #[rustc_allow_incoherent_impl]
    pub fn to_ascii_uppercase(&self) -> crate::string::String {
        let mut bytes = self.as_bytes().to_vec();
        for byte in bytes.iter_mut() {
            *byte = byte.to_ascii_uppercase();
        }
        // SAFETY: Only ASCII bytes changed, each to another ASCII byte.
        unsafe { crate::string::String::from_utf8_unchecked(bytes) }
    }
}

impl str {
    /// A new string with every match of `from` replaced by `to`.
    #[rustc_allow_incoherent_impl]
    pub fn replace<P: core::str::pattern::Pattern>(&self, from: P, to: &str) -> crate::string::String {
        let mut out = crate::string::String::new();
        let mut last = 0;
        for (start, matched) in self.match_indices(from) {
            out.push_str(&self[last..start]);
            out.push_str(to);
            last = start + matched.len();
        }
        out.push_str(&self[last..]);
        out
    }
}

fn map_uppercase_sigma(from: &str, i: usize) -> char {
    fn case_ignorable_then_cased<I: Iterator<Item = char>>(iter: I) -> bool {
        match iter.skip_while(|&c| c.is_case_ignorable()).next() {
            Some(c) => c.is_cased(),
            None => false,
        }
    }
    let is_word_final = case_ignorable_then_cased(from[..i].chars().rev())
        && !case_ignorable_then_cased(from[i + 'Σ'.len_utf8()..].chars());
    if is_word_final { 'ς' } else { 'σ' }
}
