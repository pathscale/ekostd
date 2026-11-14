//! Fixed-size arrays.
pub use crate::convert::TryFromSliceError;

#[inline]
pub const fn from_ref<T>(s: &T) -> &[T; 1] {
    // SAFETY: `[T; 1]` has identical layout to `T`.
    unsafe { &*(s as *const T as *const [T; 1]) }
}

#[inline]
pub const fn from_mut<T>(s: &mut T) -> &mut [T; 1] {
    // SAFETY: `[T; 1]` has identical layout to `T`.
    unsafe { &mut *(s as *mut T as *mut [T; 1]) }
}

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

/// Builds an array by calling `f` with each index in order.
pub fn from_fn<T, const N: usize, F: crate::ops::FnMut(usize) -> T>(mut f: F) -> [T; N] {
    let mut out = crate::mem::MaybeUninit::<[T; N]>::uninit();
    let to = out.as_mut_ptr() as *mut T;
    let mut i = 0;
    while i < N {
        unsafe { crate::ptr::write(to.add(i), f(i)) };
        i += 1;
    }
    unsafe { out.assume_init() }
}

/// An array's elements, by value, in order.
pub struct IntoIter<T, const N: usize> { data: crate::mem::MaybeUninit<[T; N]>, next: usize }
impl<T, const N: usize> crate::iter::Iterator for IntoIter<T, N> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.next == N { return None; }
        let item = unsafe { crate::ptr::read((self.data.as_ptr() as *const T).add(self.next)) };
        self.next += 1;
        Some(item)
    }
    fn size_hint(&self) -> (usize, Option<usize>) { (N - self.next, Some(N - self.next)) }
}
impl<T, const N: usize> crate::ops::Drop for IntoIter<T, N> {
    fn drop(&mut self) {
        // Elements already handed out belong to the caller; the rest are still ours to drop.
        while self.next < N {
            unsafe { crate::ptr::drop_in_place((self.data.as_mut_ptr() as *mut T).add(self.next)) };
            self.next += 1;
        }
    }
}
impl<T, const N: usize> crate::iter::IntoIterator for [T; N] {
    type Item = T;
    type IntoIter = IntoIter<T, N>;
    fn into_iter(self) -> IntoIter<T, N> { IntoIter { data: crate::mem::MaybeUninit::new(self), next: 0 } }
}

impl<'a, T, const N: usize> crate::iter::IntoIterator for &'a [T; N] {
    type Item = &'a T;
    type IntoIter = crate::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter { (self as &'a [T]).into_iter() }
}

impl<T, const N: usize> [T; N] {
    pub const fn as_slice(&self) -> &[T] { self }
}

impl<'a, T, const N: usize> crate::iter::IntoIterator for &'a mut [T; N] {
    type Item = &'a mut T;
    type IntoIter = crate::slice::IterMut<'a, T>;
    fn into_iter(self) -> crate::slice::IterMut<'a, T> { self.iter_mut() }
}
