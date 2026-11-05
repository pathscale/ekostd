use crate::cmp::{Eq, PartialEq};
use crate::intrinsics;
use crate::marker::{Copy, PointeeSized, Sized};
use crate::ops::{Index, IndexMut};
use crate::option::Option::{self, None, Some};
use crate::ptr;

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

pub unsafe trait SliceIndex<T: PointeeSized> {
    type Output: PointeeSized;
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

#[lang = "slice"]
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
        let neg_mid = unsafe { intrinsics::arith_offset((!mid) as *const u8, 1) as isize };
        let tail_len = unsafe { intrinsics::arith_offset(len as *const u8, neg_mid) as usize };
        unsafe { (from_raw_parts(ptr, mid), from_raw_parts(ptr.add(mid), tail_len)) }
    }

    #[inline(always)]
    pub const unsafe fn split_at_mut_unchecked(&mut self, mid: usize) -> (&mut [T], &mut [T]) {
        let len = self.len();
        let ptr = self.as_mut_ptr();
        let neg_mid = unsafe { intrinsics::arith_offset((!mid) as *const u8, 1) as isize };
        let tail_len = unsafe { intrinsics::arith_offset(len as *const u8, neg_mid) as usize };
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
            None => crate::panic!("slice::split_at index out of bounds"),
        }
    }

    #[inline(always)]
    pub const fn split_at_mut(&mut self, mid: usize) -> (&mut [T], &mut [T]) {
        match self.split_at_mut_checked(mid) {
            Some(pair) => pair,
            None => crate::panic!("slice::split_at_mut index out of bounds"),
        }
    }

    #[inline(always)]
    pub const fn copy_from_slice(&mut self, src: &[T])
    where
        T: Copy,
    {
        if self.len() != src.len() {
            crate::panic!("copy_from_slice: slice lengths do not match");
        }
        unsafe {
            ptr::copy_nonoverlapping(src.as_ptr(), self.as_mut_ptr(), self.len());
        }
    }

    #[inline(always)]
    pub const fn swap(&mut self, a: usize, b: usize) {
        let len = self.len();
        if a >= len || b >= len {
            crate::panic!("slice::swap index out of bounds");
        }
        unsafe {
            let pa = self.as_mut_ptr().add(a);
            let pb = self.as_mut_ptr().add(b);
            let tmp = ptr::read(pa);
            ptr::copy(pb, pa, 1);
            ptr::write(pb, tmp);
        }
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
            i = unsafe { intrinsics::arith_offset(i as *const u8, 1) as usize };
        }
        true
    }
}
impl<T: Eq> Eq for [T] {}
