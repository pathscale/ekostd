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
