use core::fmt;
use core::ops::{Deref, DerefMut};

use crate::vec::{self, Vec};

/// A priority queue: `pop` returns the greatest element.
pub struct BinaryHeap<T> { data: Vec<T> }

impl<T> BinaryHeap<T> {
    pub const fn new() -> Self { Self { data: Vec::new() } }
    /// An empty heap with room for at least `capacity` elements.
    pub fn with_capacity(capacity: usize) -> Self { Self { data: Vec::with_capacity(capacity) } }
    pub fn capacity(&self) -> usize { self.data.capacity() }
    pub fn reserve(&mut self, additional: usize) { self.data.reserve(additional) }
    pub fn len(&self) -> usize { self.data.len() }
    pub fn is_empty(&self) -> bool { self.data.is_empty() }
    pub fn peek(&self) -> Option<&T> { self.data.first() }
    /// The elements in heap order, which is arbitrary from the caller's point of view.
    pub fn iter(&self) -> core::slice::Iter<'_, T> { self.data.iter() }
    /// The underlying buffer in heap order.
    pub fn as_slice(&self) -> &[T] { self.data.as_slice() }
    /// The underlying buffer in heap order, without sorting.
    pub fn into_vec(self) -> Vec<T> { self.data }
    pub fn clear(&mut self) { self.data.clear() }
    /// Removes every element, yielding them in heap order.
    pub fn drain(&mut self) -> vec::Drain<'_, T> { self.data.drain(..) }
}

impl<T: Ord> BinaryHeap<T> {
    pub fn push(&mut self, item: T) {
        self.data.push(item);
        let last = self.data.len() - 1;
        self.sift_up(last);
    }

    pub fn pop(&mut self) -> Option<T> {
        let last = self.data.len().checked_sub(1)?;
        self.data.swap(0, last);
        let item = self.data.pop();
        let len = self.data.len();
        self.sift_down_range(0, len);
        item
    }

    /// A guard on the greatest element; if it is changed through the guard, the heap order is restored when the guard drops.
    pub fn peek_mut(&mut self) -> Option<PeekMut<'_, T>> {
        if self.data.is_empty() { None } else { Some(PeekMut { heap: self, sift: false }) }
    }

    /// The elements in ascending order, sorted in place by repeatedly moving the root to the end.
    pub fn into_sorted_vec(mut self) -> Vec<T> {
        let mut end = self.data.len();
        while end > 1 {
            end -= 1;
            self.data.swap(0, end);
            self.sift_down_range(0, end);
        }
        self.data
    }

    /// Moves every element of `other` into `self`, leaving `other` empty.
    pub fn append(&mut self, other: &mut Self) {
        if self.data.len() < other.data.len() { core::mem::swap(self, other); }
        let start = self.data.len();
        self.data.append(&mut other.data);
        let len = self.data.len();
        let added = len - start;
        // Sifting each new element up costs about added * log2(len); a full rebuild costs about 2 * len.
        let log = (usize::BITS - len.leading_zeros()) as usize;
        if added.saturating_mul(log) < 2 * len {
            let mut pos = start;
            while pos < len {
                self.sift_up(pos);
                pos += 1;
            }
        } else {
            self.rebuild();
        }
    }

    /// Keeps only the elements for which `keep` returns true.
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, keep: F) {
        let before = self.data.len();
        self.data.retain(keep);
        if self.data.len() < before { self.rebuild(); }
    }

    /// Moves the element at `pos` up while it is greater than its parent; returns where it ends.
    fn sift_up(&mut self, mut pos: usize) -> usize {
        while pos > 0 {
            let parent = (pos - 1) / 2;
            if self.data[pos] <= self.data[parent] { break; }
            self.data.swap(pos, parent);
            pos = parent;
        }
        pos
    }

    /// Moves the element at `pos` down inside `data[..end]` while a child is greater.
    fn sift_down_range(&mut self, mut pos: usize, end: usize) {
        loop {
            let mut child = 2 * pos + 1;
            if child >= end { break; }
            if child + 1 < end && self.data[child] < self.data[child + 1] { child += 1; }
            if self.data[pos] >= self.data[child] { break; }
            self.data.swap(pos, child);
            pos = child;
        }
    }

    /// Floyd's bottom-up heapify: sifts down every parent, last parent first.
    fn rebuild(&mut self) {
        let len = self.data.len();
        let mut pos = len / 2;
        while pos > 0 {
            pos -= 1;
            self.sift_down_range(pos, len);
        }
    }
}

/// A mutable guard on the greatest element of a [`BinaryHeap`], from [`BinaryHeap::peek_mut`].
pub struct PeekMut<'a, T: 'a + Ord> {
    heap: &'a mut BinaryHeap<T>,
    /// Set once the top has been handed out mutably, so drop restores the heap order.
    sift: bool,
}

impl<'a, T: Ord> PeekMut<'a, T> {
    /// Removes the peeked element from the heap and returns it.
    pub fn pop(mut this: PeekMut<'a, T>) -> T {
        this.sift = false;
        match this.heap.pop() {
            Some(item) => item,
            None => unreachable!("PeekMut exists only for a non-empty heap"),
        }
    }
}

impl<T: Ord> Drop for PeekMut<'_, T> {
    fn drop(&mut self) {
        if self.sift {
            let len = self.heap.data.len();
            self.heap.sift_down_range(0, len);
        }
    }
}

impl<T: Ord> Deref for PeekMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &T { &self.heap.data[0] }
}

impl<T: Ord> DerefMut for PeekMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.sift = true;
        &mut self.heap.data[0]
    }
}

impl<T: Ord + fmt::Debug> fmt::Debug for PeekMut<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_tuple("PeekMut").field(&self.heap.data[0]).finish() }
}

impl<T> Default for BinaryHeap<T> { fn default() -> Self { Self::new() } }

impl<T: Clone> Clone for BinaryHeap<T> {
    fn clone(&self) -> Self { Self { data: self.data.clone() } }
}

impl<T: fmt::Debug> fmt::Debug for BinaryHeap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_list().entries(self.data.iter()).finish() }
}

impl<T: Ord> From<Vec<T>> for BinaryHeap<T> {
    fn from(data: Vec<T>) -> Self {
        let mut heap = Self { data };
        heap.rebuild();
        heap
    }
}

impl<T: Ord, const N: usize> From<[T; N]> for BinaryHeap<T> {
    fn from(items: [T; N]) -> Self { Self::from(Vec::from(items)) }
}

impl<T> From<BinaryHeap<T>> for Vec<T> {
    fn from(heap: BinaryHeap<T>) -> Self { heap.data }
}

impl<T: Ord> core::iter::FromIterator<T> for BinaryHeap<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self { Self::from(iter.into_iter().collect::<Vec<T>>()) }
}

impl<T: Ord> core::iter::Extend<T> for BinaryHeap<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0);
        for item in iter { self.push(item); }
    }
}

impl<'a, T: Ord + Copy + 'a> core::iter::Extend<&'a T> for BinaryHeap<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) { self.extend(iter.into_iter().copied()) }
}

impl<T> IntoIterator for BinaryHeap<T> {
    type Item = T;
    type IntoIter = vec::IntoIter<T>;
    /// The elements in heap order, which is arbitrary from the caller's point of view.
    fn into_iter(self) -> Self::IntoIter { self.data.into_iter() }
}

impl<'a, T> IntoIterator for &'a BinaryHeap<T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter { self.data.iter() }
}
