use crate::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use crate::iter::{DoubleEndedIterator, ExactSizeIterator, IntoIterator, Iterator};
use crate::marker::{Copy, PhantomData, PointeeSized, Sized};
use crate::ops::{Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
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
