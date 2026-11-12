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
