//! Owned C strings.

use crate::boxed::Box;
use crate::vec::Vec;
use core::ffi::CStr;

/// An owned, NUL-terminated string with no NUL inside.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CString {
    // Every byte of the string, then the terminating NUL.
    inner: Box<[u8]>,
}

/// The input of `CString::new` held a NUL byte, at `nul_position`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NulError(usize, Vec<u8>);

impl NulError {
    pub fn nul_position(&self) -> usize { self.0 }
    pub fn into_vec(self) -> Vec<u8> { self.1 }
}

impl core::fmt::Display for NulError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "nul byte found in provided data at position: {}", self.0)
    }
}

impl CString {
    pub fn new<T: Into<Vec<u8>>>(t: T) -> Result<CString, NulError> {
        let bytes: Vec<u8> = t.into();
        match bytes.iter().position(|&byte| byte == 0) {
            Some(position) => Err(NulError(position, bytes)),
            None => Ok(unsafe { CString::from_vec_unchecked(bytes) }),
        }
    }

    /// The bytes must hold no NUL.
    pub unsafe fn from_vec_unchecked(mut bytes: Vec<u8>) -> CString {
        bytes.reserve_exact(1);
        bytes.push(0);
        CString { inner: bytes.into_boxed_slice() }
    }

    pub fn as_bytes(&self) -> &[u8] { &self.inner[..self.inner.len() - 1] }
    pub fn as_bytes_with_nul(&self) -> &[u8] { &self.inner }
    pub fn as_c_str(&self) -> &CStr { unsafe { CStr::from_bytes_with_nul_unchecked(&self.inner) } }
    pub fn into_bytes(self) -> Vec<u8> {
        let mut bytes = self.inner.into_vec();
        bytes.pop();
        bytes
    }
}

impl core::ops::Deref for CString {
    type Target = CStr;
    fn deref(&self) -> &CStr { self.as_c_str() }
}

impl core::fmt::Debug for CString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("\"")?;
        for &byte in self.as_bytes() { write!(f, "{}", core::ascii::escape_default(byte))?; }
        f.write_str("\"")
    }
}
