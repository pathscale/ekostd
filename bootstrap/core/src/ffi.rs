//! C ABI types for supported bootstrap targets.
#![allow(non_camel_case_types)]
#[lang = "c_void"]
#[repr(u8)]
pub enum c_void { __variant1, __variant2 }
#[cfg(all(not(any(target_vendor = "apple", target_os = "windows")), any(target_arch = "aarch64", target_arch = "arm", target_arch = "riscv32", target_arch = "riscv64")))]
pub type c_char = u8;
#[cfg(not(all(not(any(target_vendor = "apple", target_os = "windows")), any(target_arch = "aarch64", target_arch = "arm", target_arch = "riscv32", target_arch = "riscv64"))))]
pub type c_char = i8;
pub type c_schar = i8;
pub type c_uchar = u8;
pub type c_short = i16;
pub type c_ushort = u16;
pub type c_int = i32;
pub type c_uint = u32;
#[cfg(target_os = "windows")]
pub type c_long = i32;
#[cfg(target_os = "windows")]
pub type c_ulong = u32;
#[cfg(not(target_os = "windows"))]
pub type c_long = isize;
#[cfg(not(target_os = "windows"))]
pub type c_ulong = usize;
pub type c_longlong = i64;
pub type c_ulonglong = u64;
pub type c_size_t = usize;
pub type c_ssize_t = isize;
#[repr(transparent)]
pub struct CStr { bytes: [c_char] }
pub struct FromBytesWithNulError { position: Option<usize> }
impl Copy for FromBytesWithNulError {}
impl Clone for FromBytesWithNulError { fn clone(&self) -> Self { *self } }
impl FromBytesWithNulError {
    pub const fn nul_position(&self) -> Option<usize> { self.position }
}
impl CStr {
    /// The pointer must address a readable, NUL-terminated byte sequence that
    /// remains alive and unchanged for the returned reference's lifetime.
    pub unsafe fn from_ptr<'a>(ptr: *const c_char) -> &'a CStr {
        let mut len = 0;
        while unsafe { *ptr.add(len) } != 0 { len += 1; }
        let bytes = unsafe { crate::slice::from_raw_parts(ptr as *const u8, len + 1) };
        unsafe { Self::from_bytes_with_nul_unchecked(bytes) }
    }
    pub fn from_bytes_with_nul(bytes: &[u8]) -> Result<&Self, FromBytesWithNulError> {
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0 {
                if i + 1 == bytes.len() {
                    return Ok(unsafe { Self::from_bytes_with_nul_unchecked(bytes) });
                }
                return Err(FromBytesWithNulError { position: Some(i) });
            }
            i += 1;
        }
        Err(FromBytesWithNulError { position: None })
    }
    /// Bytes must contain exactly one NUL, in the final position.
    pub const unsafe fn from_bytes_with_nul_unchecked(bytes: &[u8]) -> &Self {
        unsafe { &*(bytes as *const [u8] as *const CStr) }
    }
    pub const fn as_ptr(&self) -> *const c_char { self.bytes.as_ptr() }
    pub const fn to_bytes_with_nul(&self) -> &[u8] {
        unsafe { crate::slice::from_raw_parts(self.as_ptr() as *const u8, self.bytes.len()) }
    }
    pub const fn to_bytes(&self) -> &[u8] {
        unsafe { crate::slice::from_raw_parts(self.as_ptr() as *const u8, self.bytes.len() - 1) }
    }
    pub const fn is_empty(&self) -> bool { self.bytes.len() == 1 }
    pub fn to_str(&self) -> Result<&str, crate::str::Utf8Error> { crate::str::from_utf8(self.to_bytes()) }
}
