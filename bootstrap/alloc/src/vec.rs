use crate::alloc;
use core::alloc::Layout;
use core::mem;
use core::ptr::{self, NonNull};
use core::slice;

pub struct Vec<T> {
    pointer: NonNull<T>,
    length: usize,
    capacity: usize,
}

impl<T> Vec<T> {
    pub const fn new() -> Self {
        Self { pointer: NonNull::dangling(), length: 0, capacity: 0 }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        if mem::size_of::<T>() == 0 {
            return Self { pointer: NonNull::dangling(), length: 0, capacity: usize::MAX };
        }
        if capacity == 0 {
            return Self::new();
        }
        let layout = Layout::array::<T>(capacity).unwrap_or_else(|_| panic!("capacity overflow"));
        let pointer = unsafe { alloc::alloc(layout).cast::<T>() };
        Self { pointer: unsafe { NonNull::new_unchecked(pointer) }, length: 0, capacity }
    }

    pub fn len(&self) -> usize { self.length }
    pub fn capacity(&self) -> usize { self.capacity }
    pub fn is_empty(&self) -> bool { self.length == 0 }
    pub fn as_ptr(&self) -> *const T { self.pointer.as_ptr() }
    pub fn as_mut_ptr(&mut self) -> *mut T { self.pointer.as_ptr() }

    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.length) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.length) }
    }

    pub unsafe fn set_len(&mut self, length: usize) {
        self.length = length;
    }

    pub unsafe fn from_raw_parts(pointer: *mut T, length: usize, capacity: usize) -> Self {
        Self { pointer: unsafe { NonNull::new_unchecked(pointer) }, length, capacity }
    }

    pub fn push(&mut self, value: T) {
        if self.length == self.capacity {
            self.reserve(1);
        }
        unsafe { self.pointer.as_ptr().add(self.length).write(value) };
        self.length += 1;
    }

    pub fn insert(&mut self, index: usize, value: T) {
        assert!(index <= self.length, "insertion index out of bounds");
        self.reserve(1);
        unsafe {
            let p = self.pointer.as_ptr().add(index);
            ptr::copy(p, p.add(1), self.length - index);
            p.write(value);
        }
        self.length += 1;
    }

    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.length, "removal index out of bounds");
        unsafe {
            let p = self.pointer.as_ptr().add(index);
            let value = p.read();
            ptr::copy(p.add(1), p, self.length - index - 1);
            self.length -= 1;
            value
        }
    }

    pub fn swap_remove(&mut self, index: usize) -> T {
        assert!(index < self.length, "swap_remove index out of bounds");
        unsafe {
            let p = self.pointer.as_ptr();
            let value = p.add(index).read();
            self.length -= 1;
            if index != self.length { p.add(index).write(p.add(self.length).read()); }
            value
        }
    }

    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut keep: F) {
        let mut write = 0;
        for read in 0..self.length {
            unsafe {
                let p = self.pointer.as_ptr();
                if keep(&*p.add(read)) {
                    if read != write { p.add(write).write(p.add(read).read()); }
                    write += 1;
                } else { ptr::drop_in_place(p.add(read)); }
            }
        }
        self.length = write;
    }

    pub fn dedup(&mut self) where T: PartialEq {
        if self.length < 2 { return; }
        let mut write = 1;
        for read in 1..self.length {
            unsafe {
                let p = self.pointer.as_ptr();
                if *p.add(write - 1) == *p.add(read) {
                    ptr::drop_in_place(p.add(read));
                } else {
                    if read != write { p.add(write).write(p.add(read).read()); }
                    write += 1;
                }
            }
        }
        self.length = write;
    }

    pub fn append(&mut self, other: &mut Self) {
        self.reserve(other.length);
        unsafe {
            ptr::copy_nonoverlapping(other.as_ptr(), self.pointer.as_ptr().add(self.length), other.length);
            self.length += other.length;
            other.length = 0;
        }
    }

    pub fn split_off(&mut self, at: usize) -> Self {
        assert!(at <= self.length, "split index out of bounds");
        let count = self.length - at;
        let mut result = Self::with_capacity(count);
        unsafe {
            ptr::copy_nonoverlapping(self.pointer.as_ptr().add(at), result.pointer.as_ptr(), count);
            result.length = count;
            self.length = at;
        }
        result
    }

    pub fn resize(&mut self, new_len: usize, value: T) where T: Clone {
        if new_len <= self.length { self.truncate(new_len); return; }
        let count = new_len - self.length;
        self.reserve(count);
        for _ in 1..count { self.push(value.clone()); }
        if count != 0 { self.push(value); }
    }

    pub fn resize_with<F: FnMut() -> T>(&mut self, new_len: usize, mut make: F) {
        if new_len <= self.length { self.truncate(new_len); return; }
        self.reserve(new_len - self.length);
        while self.length < new_len { self.push(make()); }
    }

    pub fn from_elem(elem: T, n: usize) -> Vec<T> where T: Clone {
        let mut result = Vec::with_capacity(n);
        for _ in 1..n { result.push(elem.clone()); }
        if n != 0 { result.push(elem); }
        result
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.length == 0 {
            return None;
        }
        self.length -= 1;
        Some(unsafe { self.pointer.as_ptr().add(self.length).read() })
    }

    pub fn clear(&mut self) {
        self.truncate(0);
    }

    pub fn truncate(&mut self, length: usize) {
        while self.length > length {
            self.length -= 1;
            unsafe { ptr::drop_in_place(self.pointer.as_ptr().add(self.length)) };
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.reserve_inner(additional, false);
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.reserve_inner(additional, true);
    }

    fn reserve_inner(&mut self, additional: usize, exact: bool) {
        let required = self.length.checked_add(additional).unwrap_or_else(|| panic!("capacity overflow"));
        if required <= self.capacity { return; }
        if mem::size_of::<T>() == 0 {
            self.capacity = usize::MAX;
            return;
        }
        let doubled = self.capacity.checked_mul(2).unwrap_or(required);
        let new_capacity = if exact { required } else { core::cmp::max(required, doubled.max(4)) };
        let new_layout = Layout::array::<T>(new_capacity).unwrap_or_else(|_| panic!("capacity overflow"));
        let new_pointer = if self.capacity == 0 {
            unsafe { alloc::alloc(new_layout).cast::<T>() }
        } else {
            let old_layout = Layout::array::<T>(self.capacity).unwrap_or_else(|_| panic!("capacity overflow"));
            unsafe { alloc::realloc(self.pointer.as_ptr().cast(), old_layout, new_layout.size()).cast::<T>() }
        };
        self.pointer = unsafe { NonNull::new_unchecked(new_pointer) };
        self.capacity = new_capacity;
    }

    pub fn extend_from_slice(&mut self, values: &[T])
    where
        T: Clone,
    {
        self.reserve(values.len());
        for value in values {
            self.push(value.clone());
        }
    }

    pub fn into_boxed_slice(mut self) -> crate::boxed::Box<[T]> {
        if self.capacity != self.length && mem::size_of::<T>() != 0 {
            if self.length == 0 {
                unsafe {
                    let layout = Layout::array::<T>(self.capacity).unwrap();
                    alloc::dealloc(self.pointer.as_ptr().cast(), layout);
                }
                self.pointer = NonNull::dangling();
                self.capacity = 0;
            } else {
                let old_layout = Layout::array::<T>(self.capacity).unwrap();
                let new_layout = Layout::array::<T>(self.length).unwrap();
                let pointer = unsafe {
                    alloc::realloc(self.pointer.as_ptr().cast(), old_layout, new_layout.size()).cast::<T>()
                };
                self.pointer = unsafe { NonNull::new_unchecked(pointer) };
                self.capacity = self.length;
            }
        }
        let this = mem::ManuallyDrop::new(self);
        let slice_ptr = ptr::slice_from_raw_parts_mut(this.pointer.as_ptr(), this.length);
        unsafe { crate::boxed::Box::from_raw(slice_ptr) }
    }
}

impl<T> Default for Vec<T> {
    fn default() -> Self { Self::new() }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        self.clear();
        if mem::size_of::<T>() != 0 && self.capacity != 0 {
            let layout = Layout::array::<T>(self.capacity).unwrap();
            unsafe { alloc::dealloc(self.pointer.as_ptr().cast(), layout) };
        }
    }
}

impl<T> core::ops::Deref for Vec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] { self.as_slice() }
}

impl<T> core::ops::DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut [T] { self.as_mut_slice() }
}

impl<T: Clone> Clone for Vec<T> {
    fn clone(&self) -> Self {
        let mut result = Self::with_capacity(self.length);
        result.extend_from_slice(self.as_slice());
        result
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for Vec<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<T: PartialEq> PartialEq for Vec<T> {
    fn eq(&self, other: &Self) -> bool { self.as_slice() == other.as_slice() }
}

impl<T: Eq> Eq for Vec<T> {}

impl<T> AsRef<[T]> for Vec<T> { fn as_ref(&self) -> &[T] { self.as_slice() } }
impl<T> AsMut<[T]> for Vec<T> { fn as_mut(&mut self) -> &mut [T] { self.as_mut_slice() } }
impl<T> core::borrow::Borrow<[T]> for Vec<T> { fn borrow(&self) -> &[T] { self.as_slice() } }
impl<T> core::borrow::BorrowMut<[T]> for Vec<T> { fn borrow_mut(&mut self) -> &mut [T] { self.as_mut_slice() } }
impl<T, I> core::ops::Index<I> for Vec<T> where I: core::slice::SliceIndex<[T]> {
    type Output = I::Output;
    fn index(&self, index: I) -> &Self::Output { &self.as_slice()[index] }
}
impl<T, I> core::ops::IndexMut<I> for Vec<T> where I: core::slice::SliceIndex<[T]> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output { &mut self.as_mut_slice()[index] }
}
impl<T: Clone> From<&[T]> for Vec<T> { fn from(values: &[T]) -> Self { let mut v=Self::with_capacity(values.len()); v.extend_from_slice(values); v } }
impl<T> From<crate::boxed::Box<[T]>> for Vec<T> { fn from(values: crate::boxed::Box<[T]>) -> Self { crate::boxed::Box::into_vec(values) } }
impl<T> core::iter::FromIterator<T> for Vec<T> { fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self { let mut v=Self::new(); v.extend(iter); v } }
impl<T> core::iter::Extend<T> for Vec<T> { fn extend<I: IntoIterator<Item=T>>(&mut self, iter: I) { let iter=iter.into_iter(); self.reserve(iter.size_hint().0); for item in iter { self.push(item); } } }
impl<T> IntoIterator for Vec<T> {
    type Item=T; type IntoIter=IntoIter<T>;
    fn into_iter(self)->IntoIter<T> { let back=self.length; IntoIter { vec: self, front: 0, back } }
}
impl<'a,T> IntoIterator for &'a Vec<T> { type Item=&'a T; type IntoIter=core::slice::Iter<'a,T>; fn into_iter(self)->Self::IntoIter { self.as_slice().iter() } }
impl<'a,T> IntoIterator for &'a mut Vec<T> { type Item=&'a mut T; type IntoIter=core::slice::IterMut<'a,T>; fn into_iter(self)->Self::IntoIter { self.as_mut_slice().iter_mut() } }

pub struct IntoIter<T> { vec: Vec<T>, front: usize, back: usize }
impl<T> Iterator for IntoIter<T> {
    type Item=T;
    fn next(&mut self)->Option<T> { if self.front==self.back{return None} let i=self.front; self.front+=1; Some(unsafe{self.vec.pointer.as_ptr().add(i).read()}) }
    fn size_hint(&self)->(usize,Option<usize>) { let n=self.back-self.front;(n,Some(n)) }
}
impl<T> DoubleEndedIterator for IntoIter<T> { fn next_back(&mut self)->Option<T> { if self.front==self.back{return None} self.back-=1; Some(unsafe{self.vec.pointer.as_ptr().add(self.back).read()}) } }
impl<T> ExactSizeIterator for IntoIter<T> {}
impl<T> Drop for IntoIter<T> { fn drop(&mut self) { unsafe { let p=self.vec.pointer.as_ptr(); for i in self.front..self.back { ptr::drop_in_place(p.add(i)); } self.vec.length=0; } } }
