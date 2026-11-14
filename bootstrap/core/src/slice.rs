use crate::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, IntoIterator, Iterator};
use crate::marker::{Copy, PhantomData, PointeeSized, Sized};
use crate::ops::{Bound, Index, IndexMut, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use crate::option::Option::{self, None, Some};
use crate::ptr;
use crate::result::Result::{self, Err, Ok};

#[rustc_diagnostic_item = "slice_from_raw_parts"]
#[inline(always)]
pub const unsafe fn from_raw_parts<'a, T>(data: *const T, len: usize) -> &'a [T] {
    unsafe { &*ptr::slice_from_raw_parts(data, len) }
}

#[rustc_diagnostic_item = "slice_from_raw_parts_mut"]
#[inline(always)]
pub const unsafe fn from_raw_parts_mut<'a, T>(data: *mut T, len: usize) -> &'a mut [T] {
    unsafe { &mut *ptr::slice_from_raw_parts_mut(data, len) }
}

#[inline(always)]
pub const fn from_ref<T>(s: &T) -> &[T] {
    unsafe { from_raw_parts(s as *const T, 1) }
}

#[inline(always)]
pub const fn from_mut<T>(s: &mut T) -> &mut [T] {
    unsafe { from_raw_parts_mut(s as *mut T, 1) }
}

pub unsafe trait SliceIndex<T: ?Sized> {
    type Output: ?Sized;
    fn get(self, slice: &T) -> Option<&Self::Output>;
    fn get_mut(self, slice: &mut T) -> Option<&mut Self::Output>;
    unsafe fn get_unchecked(self, slice: *const T) -> *const Self::Output;
    unsafe fn get_unchecked_mut(self, slice: *mut T) -> *mut Self::Output;
    fn index(self, slice: &T) -> &Self::Output;
    fn index_mut(self, slice: &mut T) -> &mut Self::Output;
}

unsafe impl<T> SliceIndex<[T]> for usize {
    type Output = T;

    #[inline(always)]
    fn get(self, slice: &[T]) -> Option<&T> {
        if self < slice.len() { unsafe { Some(&*self.get_unchecked(slice)) } } else { None }
    }

    #[inline(always)]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut T> {
        if self < slice.len() { unsafe { Some(&mut *self.get_unchecked_mut(slice)) } } else { None }
    }

    #[inline(always)]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const T {
        unsafe { (slice as *const T).add(self) }
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut T {
        unsafe { (slice as *mut T).add(self) }
    }

    #[inline(always)]
    fn index(self, slice: &[T]) -> &T {
        &(*slice)[self]
    }

    #[inline(always)]
    fn index_mut(self, slice: &mut [T]) -> &mut T {
        &mut (*slice)[self]
    }
}

unsafe impl<T> SliceIndex<[T]> for Range<usize> {
    type Output = [T];

    #[inline(always)]
    fn get(self, slice: &[T]) -> Option<&[T]> {
        if self.start > self.end || self.end > slice.len() {
            None
        } else {
            unsafe { Some(&*self.get_unchecked(slice)) }
        }
    }

    #[inline(always)]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        if self.start > self.end || self.end > slice.len() {
            None
        } else {
            unsafe { Some(&mut *self.get_unchecked_mut(slice)) }
        }
    }

    #[inline(always)]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const [T] {
        unsafe { ptr::slice_from_raw_parts((slice as *const T).add(self.start), self.end - self.start) }
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut [T] {
        unsafe { ptr::slice_from_raw_parts_mut((slice as *mut T).add(self.start), self.end - self.start) }
    }

    #[inline(always)]
    fn index(self, slice: &[T]) -> &[T] {
        match self.get(slice) {
            Some(s) => s,
            None => crate::intrinsics::abort(),
        }
    }

    #[inline(always)]
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        match self.get_mut(slice) {
            Some(s) => s,
            None => crate::intrinsics::abort(),
        }
    }
}

unsafe impl<T> SliceIndex<[T]> for RangeTo<usize> {
    type Output = [T];

    #[inline(always)]
    fn get(self, slice: &[T]) -> Option<&[T]> {
        (0..self.end).get(slice)
    }

    #[inline(always)]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        (0..self.end).get_mut(slice)
    }

    #[inline(always)]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const [T] {
        unsafe { (0..self.end).get_unchecked(slice) }
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut [T] {
        unsafe { (0..self.end).get_unchecked_mut(slice) }
    }

    #[inline(always)]
    fn index(self, slice: &[T]) -> &[T] {
        (0..self.end).index(slice)
    }

    #[inline(always)]
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        (0..self.end).index_mut(slice)
    }
}

unsafe impl<T> SliceIndex<[T]> for RangeFrom<usize> {
    type Output = [T];

    #[inline(always)]
    fn get(self, slice: &[T]) -> Option<&[T]> {
        (self.start..slice.len()).get(slice)
    }

    #[inline(always)]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        (self.start..slice.len()).get_mut(slice)
    }

    #[inline(always)]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const [T] {
        unsafe { (self.start..slice.len()).get_unchecked(slice) }
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut [T] {
        unsafe { (self.start..slice.len()).get_unchecked_mut(slice) }
    }

    #[inline(always)]
    fn index(self, slice: &[T]) -> &[T] {
        (self.start..slice.len()).index(slice)
    }

    #[inline(always)]
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        (self.start..slice.len()).index_mut(slice)
    }
}

unsafe impl<T> SliceIndex<[T]> for RangeFull {
    type Output = [T];

    #[inline(always)]
    fn get(self, slice: &[T]) -> Option<&[T]> {
        Some(slice)
    }

    #[inline(always)]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        Some(slice)
    }

    #[inline(always)]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const [T] {
        slice
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut [T] {
        slice
    }

    #[inline(always)]
    fn index(self, slice: &[T]) -> &[T] {
        slice
    }

    #[inline(always)]
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        slice
    }
}

unsafe impl<T> SliceIndex<[T]> for RangeInclusive<usize> {
    type Output = [T];

    #[inline(always)]
    fn get(self, slice: &[T]) -> Option<&[T]> {
        if *self.end() == usize::MAX {
            None
        } else {
            (*self.start()..*self.end() + 1).get(slice)
        }
    }

    #[inline(always)]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        if *self.end() == usize::MAX {
            None
        } else {
            (*self.start()..*self.end() + 1).get_mut(slice)
        }
    }

    #[inline(always)]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const [T] {
        unsafe { (*self.start()..*self.end() + 1).get_unchecked(slice) }
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut [T] {
        unsafe { (*self.start()..*self.end() + 1).get_unchecked_mut(slice) }
    }

    #[inline(always)]
    fn index(self, slice: &[T]) -> &[T] {
        match self.get(slice) {
            Some(s) => s,
            None => crate::intrinsics::abort(),
        }
    }

    #[inline(always)]
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        match self.get_mut(slice) {
            Some(s) => s,
            None => crate::intrinsics::abort(),
        }
    }
}

unsafe impl<T> SliceIndex<[T]> for RangeToInclusive<usize> {
    type Output = [T];

    #[inline(always)]
    fn get(self, slice: &[T]) -> Option<&[T]> {
        (0..=self.end).get(slice)
    }

    #[inline(always)]
    fn get_mut(self, slice: &mut [T]) -> Option<&mut [T]> {
        (0..=self.end).get_mut(slice)
    }

    #[inline(always)]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const [T] {
        unsafe { (0..=self.end).get_unchecked(slice) }
    }

    #[inline(always)]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut [T] {
        unsafe { (0..=self.end).get_unchecked_mut(slice) }
    }

    #[inline(always)]
    fn index(self, slice: &[T]) -> &[T] {
        (0..=self.end).index(slice)
    }

    #[inline(always)]
    fn index_mut(self, slice: &mut [T]) -> &mut [T] {
        (0..=self.end).index_mut(slice)
    }
}

impl<T, I> Index<I> for [T]
where
    I: SliceIndex<[T]>,
{
    type Output = I::Output;

    #[inline(always)]
    fn index(&self, index: I) -> &I::Output {
        index.index(self)
    }
}

impl<T, I> IndexMut<I> for [T]
where
    I: SliceIndex<[T]>,
{
    #[inline(always)]
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        index.index_mut(self)
    }
}

pub struct Iter<'a, T: 'a> {
    ptr: *const T,
    len: usize,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> Iter<'a, T> {
    #[inline(always)]
    pub fn as_slice(&self) -> &'a [T] {
        unsafe { from_raw_parts(self.ptr, self.len) }
    }
}

impl<'a, T> Copy for Iter<'a, T> {}
impl<'a, T> crate::clone::Clone for Iter<'a, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline(always)]
    fn next(&mut self) -> Option<&'a T> {
        if self.len == 0 {
            None
        } else {
            let item = unsafe { &*self.ptr };
            self.ptr = unsafe { self.ptr.add(1) };
            self.len -= 1;
            Some(item)
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<&'a T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(&*self.ptr.add(self.len)) }
        }
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {}

pub struct IterMut<'a, T: 'a> {
    ptr: *mut T,
    len: usize,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> IterMut<'a, T> {
    #[inline(always)]
    pub fn into_slice(self) -> &'a mut [T] {
        unsafe { from_raw_parts_mut(self.ptr, self.len) }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        unsafe { from_raw_parts(self.ptr, self.len) }
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    #[inline(always)]
    fn next(&mut self) -> Option<&'a mut T> {
        if self.len == 0 {
            None
        } else {
            let item = unsafe { &mut *self.ptr };
            self.ptr = unsafe { self.ptr.add(1) };
            self.len -= 1;
            Some(item)
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<&'a mut T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(&mut *self.ptr.add(self.len)) }
        }
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {}

impl<'a, T> IntoIterator for &'a [T] {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    #[inline(always)]
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut [T] {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    #[inline(always)]
    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<T> [T] {
    #[lang = "slice_len_fn"]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        (self as *const [T]).len()
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { ptr: self.as_ptr(), len: self.len(), _marker: PhantomData }
    }

    #[inline(always)]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut { ptr: self.as_mut_ptr(), len: self.len(), _marker: PhantomData }
    }

    #[inline(always)]
    pub const fn first(&self) -> Option<&T> {
        if let [first, ..] = self { Some(first) } else { None }
    }

    #[inline(always)]
    pub const fn first_mut(&mut self) -> Option<&mut T> {
        if let [first, ..] = self { Some(first) } else { None }
    }

    #[inline(always)]
    pub const fn last(&self) -> Option<&T> {
        if let [.., last] = self { Some(last) } else { None }
    }

    #[inline(always)]
    pub const fn last_mut(&mut self) -> Option<&mut T> {
        if let [.., last] = self { Some(last) } else { None }
    }

    #[inline(always)]
    pub const fn split_first(&self) -> Option<(&T, &[T])> {
        if let [first, tail @ ..] = self { Some((first, tail)) } else { None }
    }

    #[inline(always)]
    pub const fn split_first_mut(&mut self) -> Option<(&mut T, &mut [T])> {
        if let [first, tail @ ..] = self { Some((first, tail)) } else { None }
    }

    #[inline(always)]
    pub const fn split_last(&self) -> Option<(&T, &[T])> {
        if let [init @ .., last] = self { Some((last, init)) } else { None }
    }

    #[inline(always)]
    pub const fn split_last_mut(&mut self) -> Option<(&mut T, &mut [T])> {
        if let [init @ .., last] = self { Some((last, init)) } else { None }
    }

    #[inline(always)]
    pub const fn as_ptr(&self) -> *const T {
        self as *const [T] as *const T
    }

    #[inline(always)]
    pub const fn as_mut_ptr(&mut self) -> *mut T {
        self as *mut [T] as *mut T
    }

    #[inline(always)]
    pub fn get<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<Self>,
    {
        index.get(self)
    }

    #[inline(always)]
    pub fn get_mut<I>(&mut self, index: I) -> Option<&mut I::Output>
    where
        I: SliceIndex<Self>,
    {
        index.get_mut(self)
    }

    #[inline(always)]
    pub unsafe fn get_unchecked<I>(&self, index: I) -> &I::Output
    where
        I: SliceIndex<Self>,
    {
        unsafe { &*index.get_unchecked(self) }
    }

    #[inline(always)]
    pub unsafe fn get_unchecked_mut<I>(&mut self, index: I) -> &mut I::Output
    where
        I: SliceIndex<Self>,
    {
        unsafe { &mut *index.get_unchecked_mut(self) }
    }

    #[inline(always)]
    pub const unsafe fn split_at_unchecked(&self, mid: usize) -> (&[T], &[T]) {
        let len = self.len();
        let ptr = self.as_ptr();
        let tail_len = len - mid;
        unsafe { (from_raw_parts(ptr, mid), from_raw_parts(ptr.add(mid), tail_len)) }
    }

    #[inline(always)]
    pub const unsafe fn split_at_mut_unchecked(&mut self, mid: usize) -> (&mut [T], &mut [T]) {
        let len = self.len();
        let ptr = self.as_mut_ptr();
        let tail_len = len - mid;
        unsafe { (from_raw_parts_mut(ptr, mid), from_raw_parts_mut(ptr.add(mid), tail_len)) }
    }

    #[inline(always)]
    pub const fn split_at_checked(&self, mid: usize) -> Option<(&[T], &[T])> {
        if mid <= self.len() { unsafe { Some(self.split_at_unchecked(mid)) } } else { None }
    }

    #[inline(always)]
    pub const fn split_at_mut_checked(&mut self, mid: usize) -> Option<(&mut [T], &mut [T])> {
        if mid <= self.len() { unsafe { Some(self.split_at_mut_unchecked(mid)) } } else { None }
    }

    #[inline(always)]
    pub const fn split_at(&self, mid: usize) -> (&[T], &[T]) {
        match self.split_at_checked(mid) {
            Some(pair) => pair,
            None => crate::intrinsics::abort(),
        }
    }

    #[inline(always)]
    pub const fn split_at_mut(&mut self, mid: usize) -> (&mut [T], &mut [T]) {
        match self.split_at_mut_checked(mid) {
            Some(pair) => pair,
            None => crate::intrinsics::abort(),
        }
    }

    #[inline(always)]
    pub const fn copy_from_slice(&mut self, src: &[T])
    where
        T: Copy,
    {
        if self.len() != src.len() {
            crate::intrinsics::abort();
        }
        unsafe {
            ptr::copy_nonoverlapping(src.as_ptr(), self.as_mut_ptr(), self.len());
        }
    }

    #[inline(always)]
    pub const fn swap(&mut self, a: usize, b: usize) {
        let len = self.len();
        if a >= len || b >= len {
            crate::intrinsics::abort();
        }
        unsafe {
            let pa = self.as_mut_ptr().add(a);
            let pb = self.as_mut_ptr().add(b);
            let tmp = ptr::read(pa);
            ptr::copy(pb, pa, 1);
            ptr::write(pb, tmp);
        }
    }

    #[inline(always)]
    pub fn contains(&self, x: &T) -> bool
    where
        T: PartialEq,
    {
        let mut i = 0;
        while i < self.len() {
            if unsafe { (*self.as_ptr().add(i)).eq(x) } {
                return true;
            }
            i += 1;
        }
        false
    }

    #[inline(always)]
    pub fn starts_with(&self, needle: &[T]) -> bool
    where
        T: PartialEq,
    {
        let n = needle.len();
        self.len() >= n && &self[..n] == needle
    }

    #[inline(always)]
    pub fn ends_with(&self, needle: &[T]) -> bool
    where
        T: PartialEq,
    {
        let (m, n) = (self.len(), needle.len());
        m >= n && &self[m - n..] == needle
    }

    #[inline(always)]
    pub fn fill(&mut self, value: T)
    where
        T: crate::clone::Clone,
    {
        let len = self.len();
        if len == 0 {
            return;
        }
        let mut i = 0;
        while i + 1 < len {
            self[i] = value.clone();
            i += 1;
        }
        self[len - 1] = value;
    }

    #[inline(always)]
    pub fn binary_search(&self, x: &T) -> Result<usize, usize>
    where
        T: Ord,
    {
        self.binary_search_by(|p| p.cmp(x))
    }

    #[inline(always)]
    pub fn binary_search_by<F>(&self, mut f: F) -> Result<usize, usize>
    where
        F: crate::ops::FnMut(&T) -> Ordering,
    {
        let mut size = self.len();
        let mut left = 0;
        let mut right = size;
        while left < right {
            let mid = left + size / 2;
            let cmp = f(unsafe { self.get_unchecked(mid) });
            match cmp {
                Ordering::Less => left = mid + 1,
                Ordering::Greater => right = mid,
                Ordering::Equal => return Ok(mid),
            }
            size = right - left;
        }
        Err(left)
    }

    pub fn binary_search_by_key<B, F>(&self, b: &B, mut f: F) -> Result<usize, usize>
    where
        F: crate::ops::FnMut(&T) -> B,
        B: Ord,
    {
        self.binary_search_by(|k| f(k).cmp(b))
    }

    /// The index of the first element pred rejects, for a slice whose accepted elements all come first.
    pub fn partition_point<P>(&self, mut pred: P) -> usize
    where
        P: crate::ops::FnMut(&T) -> bool,
    {
        let (mut left, mut right) = (0, self.len());
        while left < right {
            let mid = left + (right - left) / 2;
            if pred(unsafe { self.get_unchecked(mid) }) { left = mid + 1; } else { right = mid; }
        }
        left
    }

    /// Every run of size neighbouring elements, each one element further on than the last.
    pub fn windows(&self, size: usize) -> Windows<'_, T> {
        if size == 0 { crate::intrinsics::abort(); }
        Windows { rest: self, size }
    }

    pub fn sort_by_key<K, F>(&mut self, mut f: F)
    where
        F: crate::ops::FnMut(&T) -> K,
        K: Ord,
    {
        self.sort_by(|a, b| f(a).cmp(&f(b)));
    }

    pub fn sort_unstable_by_key<K, F>(&mut self, f: F)
    where
        F: crate::ops::FnMut(&T) -> K,
        K: Ord,
    {
        self.sort_by_key(f);
    }

    /// What follows `prefix`, when the slice starts with it.
    pub fn strip_prefix(&self, prefix: &[T]) -> Option<&[T]>
    where
        T: PartialEq,
    {
        if self.starts_with(prefix) { Some(&self[prefix.len()..]) } else { None }
    }

    /// What precedes `suffix`, when the slice ends with it.
    pub fn strip_suffix(&self, suffix: &[T]) -> Option<&[T]>
    where
        T: PartialEq,
    {
        if self.ends_with(suffix) { Some(&self[..self.len() - suffix.len()]) } else { None }
    }

    /// Reverses the order of the elements in place.
    pub fn reverse(&mut self) {
        let len = self.len();
        let mut i = 0;
        while i < len / 2 {
            self.swap(i, len - 1 - i);
            i += 1;
        }
    }

    fn reverse_range(&mut self, mut start: usize, mut end: usize) {
        while start < end {
            end -= 1;
            if start < end {
                self.swap(start, end);
                start += 1;
            }
        }
    }

    /// Rotates the slice in-place such that the first `mid` elements move to the end.
    pub fn rotate_left(&mut self, mid: usize) {
        let len = self.len();
        if mid > len {
            crate::panic::panic_nounwind("slice::rotate_left: mid > len");
        }
        if mid == 0 || mid == len {
            return;
        }
        self.reverse_range(0, mid);
        self.reverse_range(mid, len);
        self.reverse_range(0, len);
    }

    /// Rotates the slice in-place such that the last `k` elements move to the front.
    pub fn rotate_right(&mut self, k: usize) {
        let len = self.len();
        if k > len {
            crate::panic::panic_nounwind("slice::rotate_right: k > len");
        }
        self.rotate_left(len - k);
    }

    /// Consecutive pieces of `size` elements; the last one holds what is left and may be shorter.
    pub fn chunks(&self, size: usize) -> Chunks<'_, T> {
        if size == 0 { crate::intrinsics::abort(); }
        Chunks { rest: self, size }
    }

    /// As `chunks`, lending each piece mutably.
    pub fn chunks_mut(&mut self, size: usize) -> ChunksMut<'_, T> {
        if size == 0 { crate::intrinsics::abort(); }
        ChunksMut { rest: self, size }
    }

    /// Consecutive pieces of `size` elements from the end; the last one yielded may be shorter.
    pub fn rchunks(&self, size: usize) -> RChunks<'_, T> {
        if size == 0 { crate::intrinsics::abort(); }
        RChunks { rest: self, size }
    }

    /// Consecutive pieces of `size` elements from the end; what does not fill a piece is the remainder.
    pub fn rchunks_exact(&self, size: usize) -> RChunksExact<'_, T> {
        if size == 0 { crate::intrinsics::abort(); }
        let rem = self.len() % size;
        let (remainder, rest) = self.split_at(rem);
        RChunksExact { rest, remainder, size }
    }

    /// Copies elements from `src` range to `dest` index within the slice.
    pub fn copy_within<R: RangeBounds<usize>>(&mut self, src: R, dest: usize)
    where
        T: Copy,
    {
        let len = self.len();
        let start = match src.start_bound() {
            crate::ops::Bound::Included(&s) => s,
            crate::ops::Bound::Excluded(&s) => s.checked_add(1).unwrap_or_else(|| crate::intrinsics::abort()),
            crate::ops::Bound::Unbounded => 0,
        };
        let end = match src.end_bound() {
            crate::ops::Bound::Included(&e) => e.checked_add(1).unwrap_or_else(|| crate::intrinsics::abort()),
            crate::ops::Bound::Excluded(&e) => e,
            crate::ops::Bound::Unbounded => len,
        };
        if start > end || end > len {
            crate::intrinsics::abort();
        }
        let count = end - start;
        if dest > len - count {
            crate::intrinsics::abort();
        }
        unsafe {
            let ptr = self.as_mut_ptr();
            crate::ptr::copy(ptr.add(start), ptr.add(dest), count);
        }
    }

    /// Consecutive pieces of exactly `size` elements; what does not fill a piece is the remainder.
    pub fn chunks_exact(&self, size: usize) -> ChunksExact<'_, T> {
        if size == 0 { crate::intrinsics::abort(); }
        let whole = self.len() - self.len() % size;
        let (rest, remainder) = self.split_at(whole);
        ChunksExact { rest, remainder, size }
    }

    /// As `chunks_exact`, lending each piece mutably. The remainder is not visited.
    pub fn chunks_exact_mut(&mut self, size: usize) -> ChunksExactMut<'_, T> {
        if size == 0 { crate::intrinsics::abort(); }
        let whole = self.len() - self.len() % size;
        ChunksExactMut { rest: self.split_at_mut(whole).0, size }
    }

    /// Cuts the part `range` names off a slice held by reference and returns it; the reference
    /// keeps the other part. `None`, changing nothing, when `range` reaches past the end.
    pub fn split_off_mut<'a, R: OneSidedRange>(self: &mut &'a mut Self, range: R) -> Option<&'a mut [T]> {
        let (front, at) = range.cut();
        if at > self.len() {
            return None;
        }
        let whole = crate::mem::replace(self, &mut []);
        let (head, tail) = whole.split_at_mut(at);
        if front {
            *self = tail;
            Some(head)
        } else {
            *self = head;
            Some(tail)
        }
    }

    /// Cuts the first element off a slice held by reference and returns it.
    pub fn split_off_first_mut<'a>(self: &mut &'a mut Self) -> Option<&'a mut T> {
        if self.is_empty() {
            return None;
        }
        let whole = crate::mem::replace(self, &mut []);
        let (head, tail) = whole.split_at_mut(1);
        *self = tail;
        Some(&mut head[0])
    }

    pub fn sort_by<F>(&mut self, mut compare: F)
    where
        F: crate::ops::FnMut(&T, &T) -> Ordering,
    {
        let len = self.len();
        let mut i = 1;
        while i < len {
            let mut j = i;
            while j > 0 && compare(&self[j - 1], &self[j]) == Ordering::Greater {
                self.swap(j - 1, j);
                j -= 1;
            }
            i += 1;
        }
    }

    pub fn sort_unstable_by<F>(&mut self, compare: F)
    where
        F: crate::ops::FnMut(&T, &T) -> Ordering,
    {
        self.sort_by(compare);
    }
}

impl<T: PartialEq> PartialEq for [T] {
    fn eq(&self, other: &[T]) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut i = 0;
        while i < self.len() {
            if unsafe { !(*self.as_ptr().add(i)).eq(&*other.as_ptr().add(i)) } {
                return false;
            }
            i += 1;
        }
        true
    }
}
impl<T: Eq> Eq for [T] {}

impl<T: PartialOrd> PartialOrd for [T] {
    fn partial_cmp(&self, other: &[T]) -> Option<Ordering> {
        let len = if self.len() < other.len() { self.len() } else { other.len() };
        let mut i = 0;
        while i < len {
            match self[i].partial_cmp(&other[i]) {
                Some(Ordering::Equal) => i += 1,
                non_eq => return non_eq,
            }
        }
        self.len().partial_cmp(&other.len())
    }
}

impl<T: Ord> Ord for [T] {
    fn cmp(&self, other: &[T]) -> Ordering {
        let len = if self.len() < other.len() { self.len() } else { other.len() };
        let mut i = 0;
        while i < len {
            match self[i].cmp(&other[i]) {
                Ordering::Equal => i += 1,
                non_eq => return non_eq,
            }
        }
        self.len().cmp(&other.len())
    }
}

impl<T, I, const N: usize> Index<I> for [T; N]
where
    I: SliceIndex<[T]>,
{
    type Output = I::Output;

    #[inline(always)]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(self as &[T], index)
    }
}

impl<T, I, const N: usize> IndexMut<I> for [T; N]
where
    I: SliceIndex<[T]>,
{
    #[inline(always)]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(self as &mut [T], index)
    }
}

pub struct Windows<'a, T: 'a> {
    rest: &'a [T],
    size: usize,
}

impl<'a, T> Iterator for Windows<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.size > self.rest.len() {
            return None;
        }
        let window = &self.rest[..self.size];
        self.rest = &self.rest[1..];
        Some(window)
    }
}

impl<T> crate::default::Default for &[T] {
    fn default() -> Self { &[] }
}

impl<T> crate::default::Default for &mut [T] {
    fn default() -> Self { &mut [] }
}

/// A range with one end left open: `..end`, `..=last` or `start..`.
pub trait OneSidedRange {
    /// Whether the range is the front part, and the index where the two parts meet.
    fn cut(self) -> (bool, usize);
}
impl OneSidedRange for crate::ops::RangeTo<usize> {
    fn cut(self) -> (bool, usize) { (true, self.end) }
}
impl OneSidedRange for crate::ops::RangeToInclusive<usize> {
    fn cut(self) -> (bool, usize) { (true, self.end.saturating_add(1)) }
}
impl OneSidedRange for crate::ops::RangeFrom<usize> {
    fn cut(self) -> (bool, usize) { (false, self.start) }
}

pub struct Chunks<'a, T: 'a> {
    rest: &'a [T],
    size: usize,
}
impl<'a, T> Iterator for Chunks<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.rest.is_empty() {
            return None;
        }
        let at = if self.size < self.rest.len() { self.size } else { self.rest.len() };
        let (piece, rest) = self.rest.split_at(at);
        self.rest = rest;
        Some(piece)
    }
}

pub struct ChunksExact<'a, T: 'a> {
    rest: &'a [T],
    remainder: &'a [T],
    size: usize,
}
impl<'a, T> ChunksExact<'a, T> {
    /// The trailing elements that do not fill a piece.
    pub fn remainder(&self) -> &'a [T] { self.remainder }
}
impl<'a, T> Iterator for ChunksExact<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.rest.is_empty() {
            return None;
        }
        let (piece, rest) = self.rest.split_at(self.size);
        self.rest = rest;
        Some(piece)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.rest.len() / self.size;
        (n, Some(n))
    }
}
impl<T> ExactSizeIterator for ChunksExact<'_, T> {}

pub struct ChunksExactMut<'a, T: 'a> {
    rest: &'a mut [T],
    size: usize,
}
impl<'a, T> Iterator for ChunksExactMut<'a, T> {
    type Item = &'a mut [T];
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.rest.is_empty() {
            return None;
        }
        let whole = crate::mem::replace(&mut self.rest, &mut []);
        let (piece, rest) = whole.split_at_mut(self.size);
        self.rest = rest;
        Some(piece)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.rest.len() / self.size;
        (n, Some(n))
    }
}
impl<T> ExactSizeIterator for ChunksExactMut<'_, T> {}

pub struct ChunksMut<'a, T: 'a> {
    rest: &'a mut [T],
    size: usize,
}
impl<'a, T> Iterator for ChunksMut<'a, T> {
    type Item = &'a mut [T];
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.rest.is_empty() {
            return None;
        }
        let whole = crate::mem::replace(&mut self.rest, &mut []);
        let at = if self.size < whole.len() { self.size } else { whole.len() };
        let (piece, rest) = whole.split_at_mut(at);
        self.rest = rest;
        Some(piece)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.rest.len() + self.size - 1) / self.size;
        (n, Some(n))
    }
}
impl<T> ExactSizeIterator for ChunksMut<'_, T> {}

pub struct RChunks<'a, T: 'a> {
    rest: &'a [T],
    size: usize,
}
impl<'a, T> Iterator for RChunks<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.rest.is_empty() {
            return None;
        }
        let at = self.rest.len().saturating_sub(self.size);
        let (rest, piece) = self.rest.split_at(at);
        self.rest = rest;
        Some(piece)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.rest.len() + self.size - 1) / self.size;
        (n, Some(n))
    }
}
impl<T> ExactSizeIterator for RChunks<'_, T> {}

pub struct RChunksExact<'a, T: 'a> {
    rest: &'a [T],
    remainder: &'a [T],
    size: usize,
}
impl<'a, T> RChunksExact<'a, T> {
    /// The leading elements that do not fill a piece.
    pub fn remainder(&self) -> &'a [T] { self.remainder }
}
impl<'a, T> Iterator for RChunksExact<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.rest.is_empty() {
            return None;
        }
        let at = self.rest.len() - self.size;
        let (rest, piece) = self.rest.split_at(at);
        self.rest = rest;
        Some(piece)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.rest.len() / self.size;
        (n, Some(n))
    }
}
impl<T> ExactSizeIterator for RChunksExact<'_, T> {}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for [T] {
    fn eq(&self, other: &[U; N]) -> bool {
        if self.len() != N {
            return false;
        }
        let mut i = 0;
        while i < N {
            if !self[i].eq(&other[i]) {
                return false;
            }
            i += 1;
        }
        true
    }
}
impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for &[T] {
    fn eq(&self, other: &[U; N]) -> bool { (**self).eq(other) }
}
impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for &mut [T] {
    fn eq(&self, other: &[U; N]) -> bool { (**self).eq(other) }
}

impl<T> crate::clone::Clone for ChunksExact<'_, T> {
    fn clone(&self) -> Self { ChunksExact { rest: self.rest, remainder: self.remainder, size: self.size } }
}
impl<T: crate::fmt::Debug> crate::fmt::Debug for ChunksExact<'_, T> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { crate::fmt::Debug::fmt(self.rest, f) }
}
