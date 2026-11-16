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
///
/// Only the elements at `start..end` are still initialized and owned by the iterator;
/// the ones before `start` and from `end` on were handed out already.
pub struct IntoIter<T, const N: usize> { data: crate::mem::MaybeUninit<[T; N]>, start: usize, end: usize }
impl<T, const N: usize> IntoIter<T, N> {
    /// An iterator that yields nothing.
    pub const fn empty() -> Self { IntoIter { data: crate::mem::MaybeUninit::uninit(), start: 0, end: 0 } }

    /// The elements not yet yielded, in order.
    pub fn as_slice(&self) -> &[T] {
        unsafe { crate::slice::from_raw_parts((self.data.as_ptr() as *const T).add(self.start), self.end - self.start) }
    }

    /// The elements not yet yielded, in order, mutably.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { crate::slice::from_raw_parts_mut((self.data.as_mut_ptr() as *mut T).add(self.start), self.end - self.start) }
    }
}
impl<T, const N: usize> crate::iter::Iterator for IntoIter<T, N> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.start == self.end { return None; }
        let item = unsafe { crate::ptr::read((self.data.as_ptr() as *const T).add(self.start)) };
        self.start += 1;
        Some(item)
    }
    fn size_hint(&self) -> (usize, Option<usize>) { (self.end - self.start, Some(self.end - self.start)) }
}
impl<T, const N: usize> crate::iter::DoubleEndedIterator for IntoIter<T, N> {
    fn next_back(&mut self) -> Option<T> {
        if self.start == self.end { return None; }
        self.end -= 1;
        Some(unsafe { crate::ptr::read((self.data.as_ptr() as *const T).add(self.end)) })
    }
}
impl<T, const N: usize> crate::iter::ExactSizeIterator for IntoIter<T, N> {
    fn len(&self) -> usize { self.end - self.start }
}
impl<T, const N: usize> crate::iter::FusedIterator for IntoIter<T, N> {}
impl<T: crate::clone::Clone, const N: usize> crate::clone::Clone for IntoIter<T, N> {
    /// Clones only the elements not yet yielded; the copy starts at index 0 of its own storage.
    fn clone(&self) -> Self {
        let mut copy = Self::empty();
        let alive = self.as_slice();
        let mut i = 0;
        while i < alive.len() {
            // `copy.end` grows one element at a time, so a panicking `clone` drops only what was written.
            unsafe { crate::ptr::write((copy.data.as_mut_ptr() as *mut T).add(copy.end), alive[i].clone()) };
            copy.end += 1;
            i += 1;
        }
        copy
    }
}
impl<T: crate::fmt::Debug, const N: usize> crate::fmt::Debug for IntoIter<T, N> {
    /// Prints the elements not yet yielded as `IntoIter([..])`.
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}
impl<T, const N: usize> crate::ops::Drop for IntoIter<T, N> {
    fn drop(&mut self) {
        // Elements already handed out belong to the caller; the alive range is still ours to drop.
        while self.start < self.end {
            unsafe { crate::ptr::drop_in_place((self.data.as_mut_ptr() as *mut T).add(self.start)) };
            self.start += 1;
        }
    }
}
impl<T, const N: usize> crate::iter::IntoIterator for [T; N] {
    type Item = T;
    type IntoIter = IntoIter<T, N>;
    fn into_iter(self) -> IntoIter<T, N> { IntoIter { data: crate::mem::MaybeUninit::new(self), start: 0, end: N } }
}

impl<'a, T, const N: usize> crate::iter::IntoIterator for &'a [T; N] {
    type Item = &'a T;
    type IntoIter = crate::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter { (self as &'a [T]).into_iter() }
}

impl<T, const N: usize> [T; N] {
    pub const fn as_slice(&self) -> &[T] { self }

    /// The whole array as a mutable slice.
    pub const fn as_mut_slice(&mut self) -> &mut [T] { self }

    /// An array of references to each element, in order.
    pub const fn each_ref(&self) -> [&T; N] {
        let mut out = crate::mem::MaybeUninit::<[&T; N]>::uninit();
        let to = out.as_mut_ptr() as *mut &T;
        let from = self as *const [T; N] as *const T;
        let mut i = 0;
        while i < N {
            unsafe { crate::ptr::write(to.add(i), &*from.add(i)) };
            i += 1;
        }
        unsafe { out.assume_init() }
    }

    /// An array of mutable references to each element, in order.
    pub fn each_mut(&mut self) -> [&mut T; N] {
        let mut out = crate::mem::MaybeUninit::<[&mut T; N]>::uninit();
        let to = out.as_mut_ptr() as *mut &mut T;
        let from = self as *mut [T; N] as *mut T;
        let mut i = 0;
        while i < N {
            unsafe { crate::ptr::write(to.add(i), &mut *from.add(i)) };
            i += 1;
        }
        unsafe { out.assume_init() }
    }
}

impl<'a, T, const N: usize> crate::iter::IntoIterator for &'a mut [T; N] {
    type Item = &'a mut T;
    type IntoIter = crate::slice::IterMut<'a, T>;
    fn into_iter(self) -> crate::slice::IterMut<'a, T> { self.iter_mut() }
}

impl<T, const N: usize> [[T; N]] {
    /// The arrays laid end to end as one slice of `len * N` elements.
    ///
    /// Panics with "slice len overflow" if that length does not fit in `usize` (only possible for zero-sized `T`).
    pub const fn as_flattened(&self) -> &[T] {
        let len = match self.len().checked_mul(N) {
            Some(len) => len,
            None => panic!("slice len overflow"),
        };
        unsafe { crate::slice::from_raw_parts(self.as_ptr() as *const T, len) }
    }

    /// The arrays laid end to end as one mutable slice of `len * N` elements.
    ///
    /// Panics with "slice len overflow" if that length does not fit in `usize` (only possible for zero-sized `T`).
    pub fn as_flattened_mut(&mut self) -> &mut [T] {
        let len = match self.len().checked_mul(N) {
            Some(len) => len,
            None => panic!("slice len overflow"),
        };
        unsafe { crate::slice::from_raw_parts_mut(self.as_mut_ptr() as *mut T, len) }
    }
}

impl<T: crate::cmp::PartialEq<U>, U, const N: usize> crate::cmp::PartialEq<[U]> for [T; N] {
    /// Equal when the slice has exactly `N` elements and each pair compares equal.
    fn eq(&self, other: &[U]) -> bool {
        if other.len() != N { return false; }
        let mut i = 0;
        while i < N {
            if !self[i].eq(&other[i]) { return false; }
            i += 1;
        }
        true
    }
}
impl<T: crate::cmp::PartialEq<U>, U, const N: usize> crate::cmp::PartialEq<&[U]> for [T; N] {
    fn eq(&self, other: &&[U]) -> bool { crate::cmp::PartialEq::<[U]>::eq(self, *other) }
}
impl<T: crate::cmp::PartialEq<U>, U, const N: usize> crate::cmp::PartialEq<&mut [U]> for [T; N] {
    fn eq(&self, other: &&mut [U]) -> bool { crate::cmp::PartialEq::<[U]>::eq(self, &**other) }
}

impl<T: Copy, const N: usize> crate::convert::TryFrom<&mut [T]> for [T; N] {
    type Error = TryFromSliceError;
    /// Copies the slice into an array; fails unless the slice has exactly `N` elements.
    fn try_from(slice: &mut [T]) -> crate::result::Result<[T; N], TryFromSliceError> {
        <[T; N] as crate::convert::TryFrom<&[T]>>::try_from(&*slice)
    }
}

/// Moves tuples of `T` into arrays of the same length, and back.
macro_rules! array_tuple_conversions {
    ($($n:literal => ($($v:ident: $t:ident),+);)+) => {$(
        impl<T> crate::convert::From<($($t,)+)> for [T; $n] {
            fn from(tuple: ($($t,)+)) -> Self {
                let ($($v,)+) = tuple;
                [$($v,)+]
            }
        }
        impl<T> crate::convert::From<[T; $n]> for ($($t,)+) {
            fn from(array: [T; $n]) -> Self {
                let [$($v,)+] = array;
                ($($v,)+)
            }
        }
    )+};
}
array_tuple_conversions! {
    1 => (a: T);
    2 => (a: T, b: T);
    3 => (a: T, b: T, c: T);
    4 => (a: T, b: T, c: T, d: T);
    5 => (a: T, b: T, c: T, d: T, e: T);
    6 => (a: T, b: T, c: T, d: T, e: T, f: T);
    7 => (a: T, b: T, c: T, d: T, e: T, f: T, g: T);
    8 => (a: T, b: T, c: T, d: T, e: T, f: T, g: T, h: T);
    9 => (a: T, b: T, c: T, d: T, e: T, f: T, g: T, h: T, i: T);
    10 => (a: T, b: T, c: T, d: T, e: T, f: T, g: T, h: T, i: T, j: T);
    11 => (a: T, b: T, c: T, d: T, e: T, f: T, g: T, h: T, i: T, j: T, k: T);
    12 => (a: T, b: T, c: T, d: T, e: T, f: T, g: T, h: T, i: T, j: T, k: T, l: T);
}
