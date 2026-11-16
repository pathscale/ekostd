use crate::vec::Vec;

/// A priority queue: `pop` returns the greatest element.
pub struct BinaryHeap<T> { data: Vec<T> }

impl<T> BinaryHeap<T> {
    pub const fn new() -> Self { Self { data: Vec::new() } }
    pub fn len(&self) -> usize { self.data.len() }
    pub fn is_empty(&self) -> bool { self.data.is_empty() }
    pub fn peek(&self) -> Option<&T> { self.data.first() }
}

impl<T: Ord> BinaryHeap<T> {
    pub fn push(&mut self, item: T) {
        self.data.push(item);
        let mut child = self.data.len() - 1;
        while child > 0 {
            let parent = (child - 1) / 2;
            if self.data[child] <= self.data[parent] { break; }
            self.data.swap(child, parent);
            child = parent;
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        let last = self.data.len().checked_sub(1)?;
        self.data.swap(0, last);
        let item = self.data.pop();
        let len = self.data.len();
        let mut parent = 0;
        loop {
            let mut child = 2 * parent + 1;
            if child >= len { break; }
            if child + 1 < len && self.data[child] < self.data[child + 1] { child += 1; }
            if self.data[parent] >= self.data[child] { break; }
            self.data.swap(parent, child);
            parent = child;
        }
        item
    }
}

impl<T> Default for BinaryHeap<T> { fn default() -> Self { Self::new() } }
