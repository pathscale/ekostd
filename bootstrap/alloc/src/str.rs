pub use core::str::*;

pub fn into_boxed_str(s: crate::string::String) -> crate::boxed::Box<str> {
    s.into_boxed_str()
}
