pub use core::str::*;

pub fn into_boxed_str(s: crate::string::String) -> crate::boxed::Box<str> {
    s.into_boxed_str()
}

impl str {
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
