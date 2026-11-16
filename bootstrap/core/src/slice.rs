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

    /// The pieces between the elements `pred` accepts, which are not in any piece. An empty slice gives one empty piece.
    pub fn split<F: FnMut(&T) -> bool>(&self, pred: F) -> Split<'_, T, F> {
        Split { v: self, pred, finished: false }
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

    /// Stable: insertion sort on runs of 16, then bottom-up in-place SymMerge of neighbouring runs.
    pub fn sort_by<F>(&mut self, mut compare: F)
    where
        F: crate::ops::FnMut(&T, &T) -> Ordering,
    {
        const RUN: usize = 16;
        let len = self.len();
        if len < 2 {
            return;
        }
        let mut start = 0;
        while start < len {
            let end = if len - start > RUN { start + RUN } else { len };
            self.insertion_sort_range(start, end, &mut compare);
            start = end;
        }
        let mut width = RUN;
        while width < len {
            let mut a = 0;
            while a + width < len {
                let m = a + width;
                let b = if len - m > width { m + width } else { len };
                self.sym_merge(a, m, b, &mut compare);
                a = b;
            }
            width *= 2;
        }
    }

    /// Sorts `self[lo..hi]`; swaps only on `Greater`, so equal elements keep their order.
    fn insertion_sort_range<F>(&mut self, lo: usize, hi: usize, compare: &mut F)
    where
        F: crate::ops::FnMut(&T, &T) -> Ordering,
    {
        let mut i = lo + 1;
        while i < hi {
            let mut j = i;
            while j > lo && compare(&self[j - 1], &self[j]) == Ordering::Greater {
                self.swap(j - 1, j);
                j -= 1;
            }
            i += 1;
        }
    }

    /// Merges the sorted neighbours `self[a..m]` and `self[m..b]` in place, stably (Kim & Kutzner SymMerge).
    fn sym_merge<F>(&mut self, a: usize, m: usize, b: usize, compare: &mut F)
    where
        F: crate::ops::FnMut(&T, &T) -> Ordering,
    {
        if m - a == 1 {
            // The first element of [m, b) not less than self[a]; self[a] goes just before it.
            let mut i = m;
            let mut j = b;
            while i < j {
                let h = i + (j - i) / 2;
                if compare(&self[h], &self[a]) == Ordering::Less { i = h + 1; } else { j = h; }
            }
            let mut k = a;
            while k + 1 < i {
                self.swap(k, k + 1);
                k += 1;
            }
            return;
        }
        if b - m == 1 {
            // The first element of [a, m) greater than self[m]; self[m] goes just before it.
            let mut i = a;
            let mut j = m;
            while i < j {
                let h = i + (j - i) / 2;
                if compare(&self[m], &self[h]) != Ordering::Less { i = h + 1; } else { j = h; }
            }
            let mut k = m;
            while k > i {
                self.swap(k, k - 1);
                k -= 1;
            }
            return;
        }
        let mid = a + (b - a) / 2;
        let n = mid + m;
        let (mut start, mut r) = if m > mid { (n - b, mid) } else { (a, m) };
        let p = n - 1;
        while start < r {
            let c = start + (r - start) / 2;
            if compare(&self[p - c], &self[c]) != Ordering::Less { start = c + 1; } else { r = c; }
        }
        let end = n - start;
        if start < m && m < end {
            self[start..end].rotate_left(m - start);
        }
        if a < start && start < mid {
            self.sym_merge(a, start, mid, compare);
        }
        if mid < end && end < b {
            self.sym_merge(mid, end, b, compare);
        }
    }

    /// Heapsort: O(n log n) worst case, in place, not stable.
    pub fn sort_unstable_by<F>(&mut self, mut compare: F)
    where
        F: crate::ops::FnMut(&T, &T) -> Ordering,
    {
        let len = self.len();
        if len < 2 {
            return;
        }
        let mut i = len / 2;
        while i > 0 {
            i -= 1;
            self.sift_down(i, len, &mut compare);
        }
        let mut end = len - 1;
        while end > 0 {
            self.swap(0, end);
            self.sift_down(0, end, &mut compare);
            end -= 1;
        }
    }

    /// Restores the max-heap property below `node` within `self[..end]`.
    fn sift_down<F>(&mut self, mut node: usize, end: usize, compare: &mut F)
    where
        F: crate::ops::FnMut(&T, &T) -> Ordering,
    {
        loop {
            let mut child = 2 * node + 1;
            if child >= end {
                break;
            }
            if child + 1 < end && compare(&self[child], &self[child + 1]) == Ordering::Less {
                child += 1;
            }
            if compare(&self[node], &self[child]) != Ordering::Less {
                break;
            }
            self.swap(node, child);
            node = child;
        }
    }
}

impl<T: PartialEq<U>, U> PartialEq<[U]> for [T] {
    fn eq(&self, other: &[U]) -> bool {
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

impl<'a, T> crate::iter::DoubleEndedIterator for Windows<'a, T> {
    fn next_back(&mut self) -> Option<&'a [T]> {
        if self.size > self.rest.len() {
            return None;
        }
        let window = &self.rest[self.rest.len() - self.size..];
        self.rest = &self.rest[..self.rest.len() - 1];
        Some(window)
    }
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

impl<T> [T] {
    #[track_caller]
    pub fn clone_from_slice(&mut self, src: &[T])
    where
        T: crate::clone::Clone,
    {
        if self.len() != src.len() { panic!("destination and source slices have different lengths"); }
        let mut i = 0;
        while i < self.len() {
            self[i].clone_from(&src[i]);
            i += 1;
        }
    }

    #[track_caller]
    pub fn swap_with_slice(&mut self, other: &mut [T]) {
        if self.len() != other.len() { panic!("destination and source slices have different lengths"); }
        // SAFETY: two distinct `&mut` slices cannot overlap, and both have `len` elements.
        unsafe { ptr::swap_nonoverlapping(self.as_mut_ptr(), other.as_mut_ptr(), self.len()) }
    }

    pub const fn split_first_chunk<const N: usize>(&self) -> Option<(&[T; N], &[T])> {
        if self.len() < N { return None; }
        let (first, tail) = self.split_at(N);
        // SAFETY: `first` has exactly `N` elements.
        Some((unsafe { &*(first.as_ptr() as *const [T; N]) }, tail))
    }
}

impl<T> [T] {
    /// Reorders the slice so `self[index]` is the element a full sort would put there, everything before it is
    /// less than or equal to it and everything after it is greater than or equal to it.
    pub fn select_nth_unstable(&mut self, index: usize) -> (&mut [T], &mut T, &mut [T])
    where
        T: Ord,
    {
        self.select_nth_unstable_by(index, |a, b| a.cmp(b))
    }

    /// `select_nth_unstable` with the order given by `compare`.
    pub fn select_nth_unstable_by<F>(&mut self, index: usize, mut compare: F) -> (&mut [T], &mut T, &mut [T])
    where
        F: crate::ops::FnMut(&T, &T) -> Ordering,
    {
        let len = self.len();
        if index >= len {
            panic!("partition_at_index index {} greater than length of slice {}", index, len);
        }
        self.select_nth_in_place(index, &mut compare);
        let (left, rest) = self.split_at_mut(index);
        let (nth, right) = rest.split_at_mut(1);
        (left, &mut nth[0], right)
    }

    /// `select_nth_unstable` with the order of the keys `f` gives.
    pub fn select_nth_unstable_by_key<K, F>(&mut self, index: usize, mut f: F) -> (&mut [T], &mut T, &mut [T])
    where
        F: crate::ops::FnMut(&T) -> K,
        K: Ord,
    {
        self.select_nth_unstable_by(index, |a, b| f(a).cmp(&f(b)))
    }

    /// Quickselect: partitions around a median-of-three pivot and keeps only the side holding `index`. After about
    /// 2*log2(len) rounds the remaining range is heapsorted, so the worst case stays O(n log n).
    fn select_nth_in_place<F>(&mut self, index: usize, compare: &mut F)
    where
        F: crate::ops::FnMut(&T, &T) -> Ordering,
    {
        let mut lo = 0;
        let mut hi = self.len();
        let mut budget = 0usize;
        let mut n = hi;
        while n > 1 {
            budget += 2;
            n >>= 1;
        }
        while hi - lo > 1 {
            if budget == 0 {
                self[lo..hi].sort_unstable_by(|a, b| compare(a, b));
                return;
            }
            budget -= 1;
            let mid = lo + (hi - lo) / 2;
            let last = hi - 1;
            if compare(&self[mid], &self[lo]) == Ordering::Less {
                self.swap(mid, lo);
            }
            if compare(&self[last], &self[mid]) == Ordering::Less {
                self.swap(last, mid);
                if compare(&self[mid], &self[lo]) == Ordering::Less {
                    self.swap(mid, lo);
                }
            }
            // The median of the three is the pivot; it waits at `last` while the range is partitioned.
            self.swap(mid, last);
            let mut store = lo;
            let mut i = lo;
            while i < last {
                if compare(&self[i], &self[last]) == Ordering::Less {
                    self.swap(i, store);
                    store += 1;
                }
                i += 1;
            }
            self.swap(store, last);
            // Now `self[lo..store]` is below the pivot, `self[store]` is the pivot and the rest is not below it.
            if index == store {
                return;
            }
            if index < store {
                hi = store;
            } else {
                lo = store + 1;
            }
        }
    }

    /// Mutable references to the elements at every index of `indices`, or an error when an index is out of
    /// bounds or two indices are equal.
    pub fn get_disjoint_mut<const N: usize>(&mut self, indices: [usize; N]) -> Result<[&mut T; N], GetDisjointMutError> {
        let len = self.len();
        let mut i = 0;
        while i < N {
            if indices[i] >= len {
                return Err(GetDisjointMutError::IndexOutOfBounds);
            }
            i += 1;
        }
        let mut i = 0;
        while i < N {
            let mut j = 0;
            while j < i {
                if indices[i] == indices[j] {
                    return Err(GetDisjointMutError::OverlappingIndices);
                }
                j += 1;
            }
            i += 1;
        }
        // SAFETY: every index is in bounds and no two are equal.
        Ok(unsafe { self.get_disjoint_unchecked_mut(indices) })
    }

    /// Mutable references to the elements at every index of `indices`, without checking them.
    ///
    /// # Safety
    /// Every index must be in bounds and no two may be equal.
    pub unsafe fn get_disjoint_unchecked_mut<const N: usize>(&mut self, indices: [usize; N]) -> [&mut T; N] {
        let base = self.as_mut_ptr();
        // SAFETY: the caller guarantees each index is in bounds and distinct, so the references never alias.
        crate::array::from_fn(|k| unsafe { &mut *base.add(indices[k]) })
    }
}

/// Why `get_disjoint_mut` refused its indices.
#[derive(crate::clone::Clone, crate::marker::Copy, crate::cmp::PartialEq, crate::cmp::Eq, crate::fmt::Debug)]
pub enum GetDisjointMutError {
    /// An index is not below the length of the slice.
    IndexOutOfBounds,
    /// Two indices are equal.
    OverlappingIndices,
}

impl crate::fmt::Display for GetDisjointMutError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        match self {
            GetDisjointMutError::IndexOutOfBounds => f.write_str("an index is out of bounds"),
            GetDisjointMutError::OverlappingIndices => f.write_str("there were overlapping indices"),
        }
    }
}

impl [u8] {
    /// Turns every ASCII lowercase letter into its uppercase form, in place.
    pub fn make_ascii_uppercase(&mut self) {
        let mut i = 0;
        while i < self.len() {
            self[i] = self[i].to_ascii_uppercase();
            i += 1;
        }
    }

    /// Turns every ASCII uppercase letter into its lowercase form, in place.
    pub fn make_ascii_lowercase(&mut self) {
        let mut i = 0;
        while i < self.len() {
            self[i] = self[i].to_ascii_lowercase();
            i += 1;
        }
    }

    /// Whether both slices have the same length and the same bytes once ASCII letters are lowercased.
    pub fn eq_ignore_ascii_case(&self, other: &[u8]) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut i = 0;
        while i < self.len() {
            if !self[i].eq_ignore_ascii_case(&other[i]) {
                return false;
            }
            i += 1;
        }
        true
    }

    /// The bytes with each one written as in a byte string literal (`core::ascii::escape_default`).
    pub fn escape_ascii(&self) -> EscapeAscii<'_> {
        EscapeAscii { bytes: self.iter(), front: EscapedByte::EMPTY, back: EscapedByte::EMPTY }
    }
}

/// The escaped form of a byte slice, as an iterator over its bytes; the result of `[u8]::escape_ascii`.
#[derive(crate::clone::Clone)]
pub struct EscapeAscii<'a> {
    bytes: Iter<'a, u8>,
    front: EscapedByte,
    back: EscapedByte,
}

/// The escape of one byte, consumable from both ends.
#[derive(crate::clone::Clone)]
struct EscapedByte {
    data: [u8; 4],
    start: u8,
    end: u8,
}

impl EscapedByte {
    const EMPTY: EscapedByte = EscapedByte { data: [0; 4], start: 0, end: 0 };

    fn of(byte: u8) -> EscapedByte {
        let mut escape = crate::ascii::escape_default(byte);
        let mut data = [0u8; 4];
        let mut end = 0u8;
        while let Some(b) = escape.next() {
            data[end as usize] = b;
            end += 1;
        }
        EscapedByte { data, start: 0, end }
    }

    fn next(&mut self) -> Option<u8> {
        if self.start == self.end { return None; }
        let b = self.data[self.start as usize];
        self.start += 1;
        Some(b)
    }

    fn next_back(&mut self) -> Option<u8> {
        if self.start == self.end { return None; }
        self.end -= 1;
        Some(self.data[self.end as usize])
    }

    fn len(&self) -> usize { (self.end - self.start) as usize }
}

impl<'a> Iterator for EscapeAscii<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        loop {
            if let Some(b) = self.front.next() { return Some(b); }
            match self.bytes.next() {
                Some(&byte) => self.front = EscapedByte::of(byte),
                None => return self.back.next(),
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let known = self.front.len() + self.back.len();
        let (rest_lo, rest_hi) = self.bytes.size_hint();
        let hi = match rest_hi {
            Some(n) => match n.checked_mul(4) {
                Some(m) => m.checked_add(known),
                None => None,
            },
            None => None,
        };
        (known.saturating_add(rest_lo), hi)
    }
}

impl<'a> DoubleEndedIterator for EscapeAscii<'a> {
    fn next_back(&mut self) -> Option<u8> {
        loop {
            if let Some(b) = self.back.next_back() { return Some(b); }
            match self.bytes.next_back() {
                Some(&byte) => self.back = EscapedByte::of(byte),
                None => return self.front.next_back(),
            }
        }
    }
}

impl crate::iter::FusedIterator for EscapeAscii<'_> {}

impl crate::fmt::Display for EscapeAscii<'_> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        let mut rest = self.clone();
        while let Some(b) = rest.next() {
            f.write_char(b as char)?;
        }
        Ok(())
    }
}

impl crate::fmt::Debug for EscapeAscii<'_> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.debug_struct("EscapeAscii").finish_non_exhaustive()
    }
}

impl [u8] {
    /// Without the ASCII whitespace at the start.
    pub const fn trim_ascii_start(&self) -> &[u8] {
        let mut bytes = self;
        while let [first, rest @ ..] = bytes {
            if first.is_ascii_whitespace() { bytes = rest; } else { break; }
        }
        bytes
    }
    /// Without the ASCII whitespace at the end.
    pub const fn trim_ascii_end(&self) -> &[u8] {
        let mut bytes = self;
        while let [rest @ .., last] = bytes {
            if last.is_ascii_whitespace() { bytes = rest; } else { break; }
        }
        bytes
    }
    /// Without the ASCII whitespace at both ends.
    pub const fn trim_ascii(&self) -> &[u8] { self.trim_ascii_start().trim_ascii_end() }

    pub const fn is_ascii(&self) -> bool {
        let mut i = 0;
        while i < self.len() {
            if self[i] > 0x7f { return false; }
            i += 1;
        }
        true
    }
}

impl<T> [T] {
    pub fn chunk_by<F: crate::ops::FnMut(&T, &T) -> bool>(&self, pred: F) -> ChunkBy<'_, T, F> {
        ChunkBy { slice: self, predicate: pred }
    }
}
/// The runs of a slice whose neighbours satisfy a predicate.
pub struct ChunkBy<'a, T, P> {
    slice: &'a [T],
    predicate: P,
}
impl<'a, T, P: crate::ops::FnMut(&T, &T) -> bool> Iterator for ChunkBy<'a, T, P> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.slice.is_empty() { return None; }
        let mut len = 1;
        while len < self.slice.len() && (self.predicate)(&self.slice[len - 1], &self.slice[len]) {
            len += 1;
        }
        let (head, tail) = self.slice.split_at(len);
        self.slice = tail;
        Some(head)
    }
}

/// What `<[T]>::split` returns.
impl<T> [T] {
    /// True when every element is less than or equal to the next one.
    pub fn is_sorted(&self) -> bool
    where
        T: PartialOrd,
    {
        self.is_sorted_by(|a, b| a.le(b))
    }

    /// True when `compare` holds for every pair of adjacent elements; slices of fewer than two elements are sorted.
    pub fn is_sorted_by<'a, F: FnMut(&'a T, &'a T) -> bool>(&'a self, mut compare: F) -> bool {
        let mut i = 1;
        while i < self.len() {
            if !compare(&self[i - 1], &self[i]) { return false; }
            i += 1;
        }
        true
    }

    /// True when the keys `f` gives for the elements are each less than or equal to the next one.
    pub fn is_sorted_by_key<'a, K: PartialOrd, F: FnMut(&'a T) -> K>(&'a self, mut f: F) -> bool {
        if self.len() < 2 { return true; }
        let mut prev = f(&self[0]);
        let mut i = 1;
        while i < self.len() {
            let key = f(&self[i]);
            if !prev.le(&key) { return false; }
            prev = key;
            i += 1;
        }
        true
    }

    /// As `split`, lending each piece mutably.
    pub fn split_mut<F: FnMut(&T) -> bool>(&mut self, pred: F) -> SplitMut<'_, T, F> {
        SplitMut { v: self, pred, finished: false }
    }

    /// The pieces of `split`, starting from the end of the slice.
    pub fn rsplit<F: FnMut(&T) -> bool>(&self, pred: F) -> RSplit<'_, T, F> {
        RSplit { inner: self.split(pred) }
    }

    /// The pieces of `split_mut`, starting from the end of the slice.
    pub fn rsplit_mut<F: FnMut(&T) -> bool>(&mut self, pred: F) -> RSplitMut<'_, T, F> {
        RSplitMut { inner: self.split_mut(pred) }
    }

    /// At most `n` pieces of `split`; the last one holds the rest of the slice. `n == 0` gives no piece.
    pub fn splitn<F: FnMut(&T) -> bool>(&self, n: usize, pred: F) -> SplitN<'_, T, F> {
        SplitN { inner: self.split(pred), count: n }
    }

    /// As `splitn`, lending each piece mutably.
    pub fn splitn_mut<F: FnMut(&T) -> bool>(&mut self, n: usize, pred: F) -> SplitNMut<'_, T, F> {
        SplitNMut { inner: self.split_mut(pred), count: n }
    }

    /// At most `n` pieces of `split` from the end; the last one holds the start of the slice. `n == 0` gives no piece.
    pub fn rsplitn<F: FnMut(&T) -> bool>(&self, n: usize, pred: F) -> RSplitN<'_, T, F> {
        RSplitN { inner: self.split(pred), count: n }
    }

    /// As `rsplitn`, lending each piece mutably.
    pub fn rsplitn_mut<F: FnMut(&T) -> bool>(&mut self, n: usize, pred: F) -> RSplitNMut<'_, T, F> {
        RSplitNMut { inner: self.split_mut(pred), count: n }
    }

    /// The pieces ending with each element `pred` accepts, which stays in its piece; no empty piece at the end, and an
    /// empty slice gives no piece.
    pub fn split_inclusive<F: FnMut(&T) -> bool>(&self, pred: F) -> SplitInclusive<'_, T, F> {
        let finished = self.is_empty();
        SplitInclusive { v: self, pred, finished }
    }

    /// As `split_inclusive`, lending each piece mutably.
    pub fn split_inclusive_mut<F: FnMut(&T) -> bool>(&mut self, pred: F) -> SplitInclusiveMut<'_, T, F> {
        let finished = self.is_empty();
        SplitInclusiveMut { v: self, pred, finished }
    }

    /// Fills the slice with the values `f` returns, called once per element from the front.
    pub fn fill_with<F: FnMut() -> T>(&mut self, mut f: F) {
        let mut i = 0;
        while i < self.len() {
            self[i] = f();
            i += 1;
        }
    }

    /// The first `N` elements as an array, or `None` when the slice is shorter than `N`.
    pub const fn first_chunk<const N: usize>(&self) -> Option<&[T; N]> {
        if self.len() < N { return None; }
        // SAFETY: the slice has at least `N` elements starting at its pointer.
        Some(unsafe { &*(self.as_ptr() as *const [T; N]) })
    }

    /// As `first_chunk`, lending the array mutably.
    pub fn first_chunk_mut<const N: usize>(&mut self) -> Option<&mut [T; N]> {
        if self.len() < N { return None; }
        // SAFETY: the slice has at least `N` elements starting at its pointer.
        Some(unsafe { &mut *(self.as_mut_ptr() as *mut [T; N]) })
    }

    /// The last `N` elements as an array, or `None` when the slice is shorter than `N`.
    pub const fn last_chunk<const N: usize>(&self) -> Option<&[T; N]> {
        if self.len() < N { return None; }
        let (_, last) = self.split_at(self.len() - N);
        // SAFETY: `last` has exactly `N` elements.
        Some(unsafe { &*(last.as_ptr() as *const [T; N]) })
    }

    /// As `last_chunk`, lending the array mutably.
    pub fn last_chunk_mut<const N: usize>(&mut self) -> Option<&mut [T; N]> {
        if self.len() < N { return None; }
        let mid = self.len() - N;
        let (_, last) = self.split_at_mut(mid);
        // SAFETY: `last` has exactly `N` elements.
        Some(unsafe { &mut *(last.as_mut_ptr() as *mut [T; N]) })
    }

    /// The slice without its last `N` elements, and those elements as an array; `None` when the slice is shorter than `N`.
    pub const fn split_last_chunk<const N: usize>(&self) -> Option<(&[T], &[T; N])> {
        if self.len() < N { return None; }
        let (init, last) = self.split_at(self.len() - N);
        // SAFETY: `last` has exactly `N` elements.
        Some((init, unsafe { &*(last.as_ptr() as *const [T; N]) }))
    }

    /// As `split_last_chunk`, lending both parts mutably.
    pub fn split_last_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [T], &mut [T; N])> {
        if self.len() < N { return None; }
        let mid = self.len() - N;
        let (init, last) = self.split_at_mut(mid);
        // SAFETY: `last` has exactly `N` elements.
        Some((init, unsafe { &mut *(last.as_mut_ptr() as *mut [T; N]) }))
    }
}

/// Index of the first element `pred` accepts.
fn split_find_first<T, P: FnMut(&T) -> bool>(v: &[T], pred: &mut P) -> Option<usize> {
    let mut i = 0;
    while i < v.len() {
        if (*pred)(&v[i]) { return Some(i); }
        i += 1;
    }
    None
}

/// Index of the last element `pred` accepts.
fn split_find_last<T, P: FnMut(&T) -> bool>(v: &[T], pred: &mut P) -> Option<usize> {
    let mut i = v.len();
    while i > 0 {
        i -= 1;
        if (*pred)(&v[i]) { return Some(i); }
    }
    None
}

/// A splitting iterator that can hand out everything it has not yielded yet as one last piece.
trait SplitFinish: DoubleEndedIterator {
    fn finish(&mut self) -> Option<Self::Item>;
}

/// One step of the `n` splitters: nothing once `count` is spent, the rest of the slice on the last step.
fn split_n_step<I: SplitFinish>(inner: &mut I, count: &mut usize, back: bool) -> Option<I::Item> {
    match *count {
        0 => None,
        1 => {
            *count = 0;
            inner.finish()
        }
        _ => {
            *count -= 1;
            if back { inner.next_back() } else { inner.next() }
        }
    }
}

impl<'a, T, P: FnMut(&T) -> bool> DoubleEndedIterator for Split<'a, T, P> {
    fn next_back(&mut self) -> Option<&'a [T]> {
        if self.finished { return None; }
        match split_find_last(self.v, &mut self.pred) {
            Some(index) => {
                let piece = &self.v[index + 1..];
                self.v = &self.v[..index];
                Some(piece)
            }
            None => {
                self.finished = true;
                Some(self.v)
            }
        }
    }
}
impl<'a, T, P: FnMut(&T) -> bool> SplitFinish for Split<'a, T, P> {
    fn finish(&mut self) -> Option<&'a [T]> {
        if self.finished { return None; }
        self.finished = true;
        Some(self.v)
    }
}
impl<'a, T, P: FnMut(&T) -> bool> crate::iter::FusedIterator for Split<'a, T, P> {}
impl<'a, T, P: FnMut(&T) -> bool + crate::clone::Clone> crate::clone::Clone for Split<'a, T, P> {
    fn clone(&self) -> Self {
        Split { v: self.v, pred: crate::clone::Clone::clone(&self.pred), finished: self.finished }
    }
}

/// What `<[T]>::rsplit` returns.
pub struct RSplit<'a, T: 'a, P: FnMut(&T) -> bool> { inner: Split<'a, T, P> }
impl<'a, T, P: FnMut(&T) -> bool> Iterator for RSplit<'a, T, P> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> { self.inner.next_back() }
}
impl<'a, T, P: FnMut(&T) -> bool> DoubleEndedIterator for RSplit<'a, T, P> {
    fn next_back(&mut self) -> Option<&'a [T]> { self.inner.next() }
}
impl<'a, T, P: FnMut(&T) -> bool> crate::iter::FusedIterator for RSplit<'a, T, P> {}
impl<'a, T, P: FnMut(&T) -> bool + crate::clone::Clone> crate::clone::Clone for RSplit<'a, T, P> {
    fn clone(&self) -> Self { RSplit { inner: crate::clone::Clone::clone(&self.inner) } }
}

/// What `<[T]>::splitn` returns.
pub struct SplitN<'a, T: 'a, P: FnMut(&T) -> bool> { inner: Split<'a, T, P>, count: usize }
impl<'a, T, P: FnMut(&T) -> bool> Iterator for SplitN<'a, T, P> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> { split_n_step(&mut self.inner, &mut self.count, false) }
}
impl<'a, T, P: FnMut(&T) -> bool> crate::iter::FusedIterator for SplitN<'a, T, P> {}
impl<'a, T, P: FnMut(&T) -> bool + crate::clone::Clone> crate::clone::Clone for SplitN<'a, T, P> {
    fn clone(&self) -> Self { SplitN { inner: crate::clone::Clone::clone(&self.inner), count: self.count } }
}

/// What `<[T]>::rsplitn` returns.
pub struct RSplitN<'a, T: 'a, P: FnMut(&T) -> bool> { inner: Split<'a, T, P>, count: usize }
impl<'a, T, P: FnMut(&T) -> bool> Iterator for RSplitN<'a, T, P> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> { split_n_step(&mut self.inner, &mut self.count, true) }
}
impl<'a, T, P: FnMut(&T) -> bool> crate::iter::FusedIterator for RSplitN<'a, T, P> {}
impl<'a, T, P: FnMut(&T) -> bool + crate::clone::Clone> crate::clone::Clone for RSplitN<'a, T, P> {
    fn clone(&self) -> Self { RSplitN { inner: crate::clone::Clone::clone(&self.inner), count: self.count } }
}

/// What `<[T]>::split_mut` returns.
pub struct SplitMut<'a, T: 'a, P: FnMut(&T) -> bool> { v: &'a mut [T], pred: P, finished: bool }
impl<'a, T, P: FnMut(&T) -> bool> Iterator for SplitMut<'a, T, P> {
    type Item = &'a mut [T];
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.finished { return None; }
        match split_find_first(&*self.v, &mut self.pred) {
            Some(index) => {
                let rest = crate::mem::take(&mut self.v);
                let (head, tail) = rest.split_at_mut(index);
                let (_, after) = tail.split_at_mut(1);
                self.v = after;
                Some(head)
            }
            None => self.finish(),
        }
    }
}
impl<'a, T, P: FnMut(&T) -> bool> DoubleEndedIterator for SplitMut<'a, T, P> {
    fn next_back(&mut self) -> Option<&'a mut [T]> {
        if self.finished { return None; }
        match split_find_last(&*self.v, &mut self.pred) {
            Some(index) => {
                let rest = crate::mem::take(&mut self.v);
                let (head, tail) = rest.split_at_mut(index);
                let (_, after) = tail.split_at_mut(1);
                self.v = head;
                Some(after)
            }
            None => self.finish(),
        }
    }
}
impl<'a, T, P: FnMut(&T) -> bool> SplitFinish for SplitMut<'a, T, P> {
    fn finish(&mut self) -> Option<&'a mut [T]> {
        if self.finished { return None; }
        self.finished = true;
        Some(crate::mem::take(&mut self.v))
    }
}
impl<'a, T, P: FnMut(&T) -> bool> crate::iter::FusedIterator for SplitMut<'a, T, P> {}

/// What `<[T]>::rsplit_mut` returns.
pub struct RSplitMut<'a, T: 'a, P: FnMut(&T) -> bool> { inner: SplitMut<'a, T, P> }
impl<'a, T, P: FnMut(&T) -> bool> Iterator for RSplitMut<'a, T, P> {
    type Item = &'a mut [T];
    fn next(&mut self) -> Option<&'a mut [T]> { self.inner.next_back() }
}
impl<'a, T, P: FnMut(&T) -> bool> DoubleEndedIterator for RSplitMut<'a, T, P> {
    fn next_back(&mut self) -> Option<&'a mut [T]> { self.inner.next() }
}
impl<'a, T, P: FnMut(&T) -> bool> crate::iter::FusedIterator for RSplitMut<'a, T, P> {}

/// What `<[T]>::splitn_mut` returns.
pub struct SplitNMut<'a, T: 'a, P: FnMut(&T) -> bool> { inner: SplitMut<'a, T, P>, count: usize }
impl<'a, T, P: FnMut(&T) -> bool> Iterator for SplitNMut<'a, T, P> {
    type Item = &'a mut [T];
    fn next(&mut self) -> Option<&'a mut [T]> { split_n_step(&mut self.inner, &mut self.count, false) }
}
impl<'a, T, P: FnMut(&T) -> bool> crate::iter::FusedIterator for SplitNMut<'a, T, P> {}

/// What `<[T]>::rsplitn_mut` returns.
pub struct RSplitNMut<'a, T: 'a, P: FnMut(&T) -> bool> { inner: SplitMut<'a, T, P>, count: usize }
impl<'a, T, P: FnMut(&T) -> bool> Iterator for RSplitNMut<'a, T, P> {
    type Item = &'a mut [T];
    fn next(&mut self) -> Option<&'a mut [T]> { split_n_step(&mut self.inner, &mut self.count, true) }
}
impl<'a, T, P: FnMut(&T) -> bool> crate::iter::FusedIterator for RSplitNMut<'a, T, P> {}

/// What `<[T]>::split_inclusive` returns.
pub struct SplitInclusive<'a, T: 'a, P: FnMut(&T) -> bool> { v: &'a [T], pred: P, finished: bool }
impl<'a, T, P: FnMut(&T) -> bool> Iterator for SplitInclusive<'a, T, P> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.finished { return None; }
        let end = match split_find_first(self.v, &mut self.pred) {
            Some(index) => index + 1,
            None => self.v.len(),
        };
        if end == self.v.len() { self.finished = true; }
        let (head, tail) = self.v.split_at(end);
        self.v = tail;
        Some(head)
    }
}
impl<'a, T, P: FnMut(&T) -> bool> DoubleEndedIterator for SplitInclusive<'a, T, P> {
    fn next_back(&mut self) -> Option<&'a [T]> {
        if self.finished { return None; }
        // The last element ends the last piece whether or not `pred` accepts it, so it is not searched.
        let searched = if self.v.is_empty() { 0 } else { self.v.len() - 1 };
        let start = match split_find_last(&self.v[..searched], &mut self.pred) {
            Some(index) => index + 1,
            None => 0,
        };
        if start == 0 { self.finished = true; }
        let (head, tail) = self.v.split_at(start);
        self.v = head;
        Some(tail)
    }
}
impl<'a, T, P: FnMut(&T) -> bool> crate::iter::FusedIterator for SplitInclusive<'a, T, P> {}
impl<'a, T, P: FnMut(&T) -> bool + crate::clone::Clone> crate::clone::Clone for SplitInclusive<'a, T, P> {
    fn clone(&self) -> Self {
        SplitInclusive { v: self.v, pred: crate::clone::Clone::clone(&self.pred), finished: self.finished }
    }
}

/// What `<[T]>::split_inclusive_mut` returns.
pub struct SplitInclusiveMut<'a, T: 'a, P: FnMut(&T) -> bool> { v: &'a mut [T], pred: P, finished: bool }
impl<'a, T, P: FnMut(&T) -> bool> Iterator for SplitInclusiveMut<'a, T, P> {
    type Item = &'a mut [T];
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.finished { return None; }
        let end = match split_find_first(&*self.v, &mut self.pred) {
            Some(index) => index + 1,
            None => self.v.len(),
        };
        if end == self.v.len() { self.finished = true; }
        let rest = crate::mem::take(&mut self.v);
        let (head, tail) = rest.split_at_mut(end);
        self.v = tail;
        Some(head)
    }
}
impl<'a, T, P: FnMut(&T) -> bool> DoubleEndedIterator for SplitInclusiveMut<'a, T, P> {
    fn next_back(&mut self) -> Option<&'a mut [T]> {
        if self.finished { return None; }
        // The last element ends the last piece whether or not `pred` accepts it, so it is not searched.
        let searched = if self.v.is_empty() { 0 } else { self.v.len() - 1 };
        let start = match split_find_last(&self.v[..searched], &mut self.pred) {
            Some(index) => index + 1,
            None => 0,
        };
        if start == 0 { self.finished = true; }
        let rest = crate::mem::take(&mut self.v);
        let (head, tail) = rest.split_at_mut(start);
        self.v = head;
        Some(tail)
    }
}
impl<'a, T, P: FnMut(&T) -> bool> crate::iter::FusedIterator for SplitInclusiveMut<'a, T, P> {}

pub struct Split<'a, T: 'a, P: FnMut(&T) -> bool> { v: &'a [T], pred: P, finished: bool }
impl<'a, T, P: FnMut(&T) -> bool> Iterator for Split<'a, T, P> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.finished { return None; }
        let mut index = 0;
        while index < self.v.len() {
            if (self.pred)(&self.v[index]) {
                let piece = &self.v[..index];
                self.v = &self.v[index + 1..];
                return Some(piece);
            }
            index += 1;
        }
        self.finished = true;
        Some(self.v)
    }
}
