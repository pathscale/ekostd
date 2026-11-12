//! Fixed-size arrays.
pub use crate::convert::TryFromSliceError;

impl<T, const N: usize> [T; N] {
    /// Applies `f` to each element in order and collects the results.
    ///
    /// If `f` panics the elements not yet visited are leaked, not dropped twice.
    pub fn map<U, F: crate::ops::FnMut(T) -> U>(self, mut f: F) -> [U; N] {
        let source = crate::mem::ManuallyDrop::new(self);
        let mut out = crate::mem::MaybeUninit::<[U; N]>::uninit();
        let from = &*source as *const [T; N] as *const T;
        let to = out.as_mut_ptr() as *mut U;
        let mut i = 0;
        while i < N {
            unsafe { crate::ptr::write(to.add(i), f(crate::ptr::read(from.add(i)))) };
            i += 1;
        }
        unsafe { out.assume_init() }
    }
}
