use crate::collections::TryReserveError;
use crate::vec::Vec;
use core::alloc::Layout;
use core::fmt;
use core::iter::FromIterator;
use core::mem::{self, ManuallyDrop};
use core::ops::{Index, IndexMut};
use core::ptr;
use core::slice;

/// A double-ended queue in a ring buffer: element `i` lives in slot `head + i`, wrapped at the capacity, so both
/// ends push and pop in amortized O(1) and elements move only when the buffer grows.
pub struct VecDeque<T> {
    /// The storage: its length stays 0 and only its capacity is used; the deque owns the live slots.
    buf: Vec<T>,
    head: usize,
    len: usize,
}

impl<T> VecDeque<T> {
    pub const fn new() -> Self { Self { buf: Vec::new(), head: 0, len: 0 } }
    pub fn with_capacity(capacity: usize) -> Self { Self { buf: Vec::with_capacity(capacity), head: 0, len: 0 } }
    pub fn len(&self) -> usize { self.len }
    pub fn is_empty(&self) -> bool { self.len == 0 }
    pub fn capacity(&self) -> usize { self.buf.capacity() }

    fn ptr(&self) -> *mut T { self.buf.as_ptr() as *mut T }

    /// The slot `offset` places after `slot`, wrapping at the capacity.
    fn wrap_add(&self, slot: usize, offset: usize) -> usize {
        let room = self.capacity() - slot;
        if offset >= room { offset - room } else { slot + offset }
    }

    /// The slot `offset` places before `slot`, wrapping at the capacity.
    fn wrap_sub(&self, slot: usize, offset: usize) -> usize {
        if slot >= offset { slot - offset } else { self.capacity() - (offset - slot) }
    }

    /// The slot of element `index`.
    fn slot(&self, index: usize) -> usize { self.wrap_add(self.head, index) }

    /// Moves the elements, in order, to the start of a new buffer of `capacity` slots.
    fn relocate(&mut self, capacity: usize) {
        let mut buf = Vec::with_capacity(capacity);
        let (front, back) = self.as_slices();
        let front_len = front.len();
        // SAFETY: the new buffer is a separate allocation of at least `len` slots; each element is moved bitwise once
        // and the old buffer, whose length is 0, only frees its memory when it is replaced.
        unsafe {
            ptr::copy_nonoverlapping(front.as_ptr(), buf.as_mut_ptr(), front_len);
            ptr::copy_nonoverlapping(back.as_ptr(), buf.as_mut_ptr().add(front_len), back.len());
        }
        self.buf = buf;
        self.head = 0;
    }

    /// Makes room for at least `additional` more elements, doubling the capacity (at least 4) when it grows.
    pub fn reserve(&mut self, additional: usize) {
        let required = self.len.checked_add(additional).expect("capacity overflow");
        let capacity = self.capacity();
        if required <= capacity { return; }
        let doubled = capacity.checked_mul(2).unwrap_or(required);
        let mut target = required.max(doubled).max(4);
        if mem::size_of::<T>() != 0 && Layout::array::<T>(target).is_err() {
            target = required;
        }
        self.relocate(target);
    }

    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        match self.len.checked_add(additional) {
            Some(required) if mem::size_of::<T>() == 0 || Layout::array::<T>(required).is_ok() => {
                self.reserve(additional);
                Ok(())
            }
            _ => Err(TryReserveError::capacity_overflow()),
        }
    }

    pub fn push_back(&mut self, value: T) {
        if self.len == self.capacity() { self.reserve(1); }
        let slot = self.slot(self.len);
        // SAFETY: `len < capacity`, so `slot` is a free slot inside the buffer.
        unsafe { self.ptr().add(slot).write(value) };
        self.len += 1;
    }

    pub fn push_front(&mut self, value: T) {
        if self.len == self.capacity() { self.reserve(1); }
        self.head = self.wrap_sub(self.head, 1);
        // SAFETY: `len < capacity`, so the slot before the old head is free.
        unsafe { self.ptr().add(self.head).write(value) };
        self.len += 1;
    }

    pub fn pop_back(&mut self) -> Option<T> {
        if self.len == 0 { return None; }
        self.len -= 1;
        let slot = self.slot(self.len);
        // SAFETY: `slot` held the last live element, which is moved out and no longer counted.
        Some(unsafe { self.ptr().add(slot).read() })
    }

    pub fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 { return None; }
        let slot = self.head;
        self.head = self.wrap_add(self.head, 1);
        self.len -= 1;
        // SAFETY: `slot` held the first live element, which is moved out and no longer counted.
        Some(unsafe { self.ptr().add(slot).read() })
    }

    pub fn front(&self) -> Option<&T> { self.get(0) }
    pub fn front_mut(&mut self) -> Option<&mut T> { self.get_mut(0) }
    pub fn back(&self) -> Option<&T> { if self.len == 0 { None } else { self.get(self.len - 1) } }
    pub fn back_mut(&mut self) -> Option<&mut T> { if self.len == 0 { None } else { self.get_mut(self.len - 1) } }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len { return None; }
        // SAFETY: `index < len`, so its slot holds a live element borrowed with `self`.
        Some(unsafe { &*self.ptr().add(self.slot(index)) })
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len { return None; }
        // SAFETY: `index < len`, so its slot holds a live element borrowed mutably with `self`.
        Some(unsafe { &mut *self.ptr().add(self.slot(index)) })
    }

    /// Whether some element equals `value`.
    pub fn contains(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.iter().any(|item| item == value)
    }

    /// Swaps elements `i` and `j`; panics when either is out of bounds.
    pub fn swap(&mut self, i: usize, j: usize) {
        assert!(i < self.len && j < self.len, "swap index out of bounds");
        let (a, b) = (self.slot(i), self.slot(j));
        if a != b {
            // SAFETY: both slots hold live elements and are distinct, so the two one-element
            // regions do not overlap.
            unsafe { ptr::swap_nonoverlapping(self.ptr().add(a), self.ptr().add(b), 1) };
        }
    }

    /// Drops the elements from `len` on, back to front.
    pub fn truncate(&mut self, len: usize) {
        while self.len > len {
            self.len -= 1;
            let slot = self.slot(self.len);
            // SAFETY: `slot` held the last live element, no longer counted, so it is dropped exactly once.
            unsafe { ptr::drop_in_place(self.ptr().add(slot)) };
        }
    }

    pub fn clear(&mut self) {
        self.truncate(0);
        self.head = 0;
    }

    /// Keeps the elements `keep` accepts, in order.
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut keep: F) {
        let mut kept = 0;
        let mut i = 0;
        while i < self.len {
            if keep(&self[i]) {
                self.swap(kept, i);
                kept += 1;
            }
            i += 1;
        }
        self.truncate(kept);
    }

    /// Inserts `value` at `index`, moving the shorter side by one; panics when `index > len`.
    pub fn insert(&mut self, index: usize, value: T) {
        assert!(index <= self.len, "insertion index out of bounds");
        if self.len == self.capacity() { self.reserve(1); }
        let base = self.ptr();
        // SAFETY: `len < capacity`; each element before (or from) `index` moves one slot toward the free slot, in an
        // order that never overwrites an element not yet moved, and `value` fills the slot left at `index`.
        unsafe {
            if index < self.len - index {
                self.head = self.wrap_sub(self.head, 1);
                for i in 0..index {
                    ptr::copy(base.add(self.slot(i + 1)), base.add(self.slot(i)), 1);
                }
            } else {
                let mut i = self.len;
                while i > index {
                    ptr::copy(base.add(self.slot(i - 1)), base.add(self.slot(i)), 1);
                    i -= 1;
                }
            }
            base.add(self.slot(index)).write(value);
        }
        self.len += 1;
    }

    /// Removes and returns the element at `index`, or `None` past the end; later elements move up.
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.len { return None; }
        let base = self.ptr();
        // SAFETY: element `index` is moved out, then the shorter side moves one slot into the hole, each element once.
        unsafe {
            let value = base.add(self.slot(index)).read();
            if index < self.len - 1 - index {
                let mut i = index;
                while i > 0 {
                    ptr::copy(base.add(self.slot(i - 1)), base.add(self.slot(i)), 1);
                    i -= 1;
                }
                self.head = self.wrap_add(self.head, 1);
            } else {
                for i in index + 1..self.len {
                    ptr::copy(base.add(self.slot(i)), base.add(self.slot(i - 1)), 1);
                }
            }
            self.len -= 1;
            Some(value)
        }
    }

    /// The elements as two slices, front to back: the run from `head` to the end of the buffer, then the wrapped run.
    pub fn as_slices(&self) -> (&[T], &[T]) {
        let first = self.len.min(self.capacity() - self.head);
        // SAFETY: slots `[head, head + first)` and `[0, len - first)` are the live elements and do not overlap.
        unsafe {
            (slice::from_raw_parts(self.ptr().add(self.head), first), slice::from_raw_parts(self.ptr(), self.len - first))
        }
    }

    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        let first = self.len.min(self.capacity() - self.head);
        // SAFETY: slots `[head, head + first)` and `[0, len - first)` are the live elements and do not overlap.
        unsafe {
            (
                slice::from_raw_parts_mut(self.ptr().add(self.head), first),
                slice::from_raw_parts_mut(self.ptr(), self.len - first),
            )
        }
    }

    /// Rearranges the elements in place so they are one slice, front to back, and returns it.
    pub fn make_contiguous(&mut self) -> &mut [T] {
        let room = self.capacity() - self.head;
        if self.len > room {
            let tail = self.len - room;
            let base = self.ptr();
            // SAFETY: the wrapped run `[0, tail)` and the front run `[head, capacity)` are live with `tail <= head`;
            // moving the front run down to `tail` leaves `[0, len)` live (wrapped run, then front run), and the
            // rotation puts the front run first.
            unsafe {
                ptr::copy(base.add(self.head), base.add(tail), room);
                slice::from_raw_parts_mut(base, self.len).rotate_left(tail);
            }
            self.head = 0;
        }
        // SAFETY: the `len` live elements now sit in order from `head`.
        unsafe { slice::from_raw_parts_mut(self.ptr().add(self.head), self.len) }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        let (front, back) = self.as_slices();
        Iter { front: front.iter(), back: back.iter() }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        let (front, back) = self.as_mut_slices();
        IterMut { front: front.iter_mut(), back: back.iter_mut() }
    }
}

// SAFETY: the elements are only dropped, never read; the `Vec<T>` field tells the drop checker the deque owns `T`.
unsafe impl<#[may_dangle] T> Drop for VecDeque<T> {
    fn drop(&mut self) {
        let (front, back) = self.as_mut_slices();
        // SAFETY: each live element is dropped exactly once; `buf` then frees the memory without touching the slots.
        unsafe {
            ptr::drop_in_place(front);
            ptr::drop_in_place(back);
        }
    }
}

impl<T> Index<usize> for VecDeque<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T { self.get(index).expect("Out of bounds access") }
}
impl<T> IndexMut<usize> for VecDeque<T> {
    fn index_mut(&mut self, index: usize) -> &mut T { self.get_mut(index).expect("Out of bounds access") }
}

impl<T> Default for VecDeque<T> { fn default() -> Self { Self::new() } }
impl<T: Clone> Clone for VecDeque<T> {
    fn clone(&self) -> Self {
        let mut copy = Self::with_capacity(self.len);
        for item in self.iter() { copy.push_back(item.clone()); }
        copy
    }
}
impl<T: PartialEq> PartialEq for VecDeque<T> { fn eq(&self, o: &Self) -> bool { self.len == o.len && self.iter().eq(o.iter()) } }
impl<T: Eq> Eq for VecDeque<T> {}
impl<T: fmt::Debug> fmt::Debug for VecDeque<T> { fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result { f.debug_list().entries(self.iter()).finish() } }
impl<T> IntoIterator for VecDeque<T> { type Item=T; type IntoIter=IntoIter<T>; fn into_iter(self)->Self::IntoIter { IntoIter { deque: self } } }
impl<'a, T> IntoIterator for &'a VecDeque<T> { type Item=&'a T; type IntoIter=Iter<'a,T>; fn into_iter(self)->Self::IntoIter { self.iter() } }
impl<'a, T> IntoIterator for &'a mut VecDeque<T> { type Item=&'a mut T; type IntoIter=IterMut<'a,T>; fn into_iter(self)->Self::IntoIter { self.iter_mut() } }
impl<T> FromIterator<T> for VecDeque<T> { fn from_iter<I:IntoIterator<Item=T>>(it:I)->Self { Self::from(Vec::from_iter(it)) } }
impl<T> Extend<T> for VecDeque<T> {
    fn extend<I:IntoIterator<Item=T>>(&mut self,it:I) {
        let it = it.into_iter();
        self.reserve(it.size_hint().0);
        for item in it { self.push_back(item); }
    }
}
/// Takes over the Vec's buffer: no element moves and nothing is allocated.
impl<T> From<Vec<T>> for VecDeque<T> {
    fn from(items: Vec<T>) -> Self {
        let mut items = ManuallyDrop::new(items);
        let (pointer, len, capacity) = (items.as_mut_ptr(), items.len(), items.capacity());
        // SAFETY: the buffer came from a Vec of this capacity and is now owned only here; its `len` elements
        // stay live in slots `[0, len)`, tracked by the deque rather than by the Vec's length.
        let buf = unsafe { Vec::from_raw_parts(pointer, 0, capacity) };
        Self { buf, head: 0, len }
    }
}
impl<T, const N: usize> From<[T; N]> for VecDeque<T> { fn from(items: [T; N]) -> Self { Self::from(Vec::from(items)) } }
/// Hands the buffer to a Vec after moving the elements to its start, in order; nothing is allocated.
impl<T> From<VecDeque<T>> for Vec<T> {
    fn from(deque: VecDeque<T>) -> Self {
        let mut deque = ManuallyDrop::new(deque);
        deque.make_contiguous();
        let (head, len) = (deque.head, deque.len);
        // SAFETY: after `make_contiguous` the elements sit in slots `[head, head + len)`; they move to `[0, len)`,
        // and the buffer is read out of the never-dropped deque, so it is owned once, by the returned Vec.
        unsafe {
            let base = deque.ptr();
            ptr::copy(base.add(head), base, len);
            let mut buf = ptr::read(&deque.buf);
            buf.set_len(len);
            buf
        }
    }
}

/// Front-to-back iterator over references: the run from `head`, then the wrapped run.
pub struct Iter<'a, T> {
    front: slice::Iter<'a, T>,
    back: slice::Iter<'a, T>,
}
impl<T> Clone for Iter<'_, T> { fn clone(&self) -> Self { Iter { front: self.front.clone(), back: self.back.clone() } } }
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        match self.front.next() {
            Some(item) => Some(item),
            None => self.back.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.front.len() + self.back.len();
        (n, Some(n))
    }
}
impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> {
        match self.back.next_back() {
            Some(item) => Some(item),
            None => self.front.next_back(),
        }
    }
}
impl<T> ExactSizeIterator for Iter<'_, T> {}

/// Front-to-back iterator over mutable references.
pub struct IterMut<'a, T> {
    front: slice::IterMut<'a, T>,
    back: slice::IterMut<'a, T>,
}
impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T> {
        match self.front.next() {
            Some(item) => Some(item),
            None => self.back.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.front.len() + self.back.len();
        (n, Some(n))
    }
}
impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<&'a mut T> {
        match self.back.next_back() {
            Some(item) => Some(item),
            None => self.front.next_back(),
        }
    }
}
impl<T> ExactSizeIterator for IterMut<'_, T> {}

/// Front-to-back iterator that moves the elements out; the ones not taken are dropped with it.
pub struct IntoIter<T> {
    deque: VecDeque<T>,
}
impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> { self.deque.pop_front() }
    fn size_hint(&self) -> (usize, Option<usize>) { (self.deque.len, Some(self.deque.len)) }
}
impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> { self.deque.pop_back() }
}
impl<T> ExactSizeIterator for IntoIter<T> {}
