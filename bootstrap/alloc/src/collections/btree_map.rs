//! An ordered map as a B-tree: every node holds between `MIN` and `CAPACITY` pairs in key order
//! (the root may hold fewer), an internal node one child more than it has pairs, and every leaf is
//! at the same depth. An insert or a removal touches one root-to-leaf path and moves at most
//! `CAPACITY` pairs inside a node, so neither depends on the size of the map. That is why it is a
//! tree and not one sorted `Vec`: keys that arrive out of order (`frontend_tree::deftree`'s
//! `join_unit_as` fills `invoc_parent` that way) used to move the whole tail of the map on every
//! insert.
//!
//! Inserts split a full child before descending into it and removals fill a minimal child before
//! descending into it (top-down, as in Cormen et al.), so neither ever walks back up: the code is
//! plain recursion over `&mut Node` with no parent pointers and no `unsafe`.

use crate::vec::Vec;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::iter::{FromIterator, FusedIterator};
use core::ops::{Bound, Index, RangeBounds};

/// Minimum degree: a node other than the root has at least `B` children and `B - 1` pairs.
const B: usize = 6;
const CAPACITY: usize = 2 * B - 1;
const MIN: usize = B - 1;

/// One node. A leaf has no children; an internal node has `entries.len() + 1`.
struct Node<K, V> {
    entries: Vec<(K, V)>,
    children: Vec<Node<K, V>>,
}

impl<K: Clone, V: Clone> Clone for Node<K, V> {
    fn clone(&self) -> Self { Node { entries: self.entries.clone(), children: self.children.clone() } }
}

impl<K, V> Node<K, V> {
    const fn new() -> Self { Node { entries: Vec::new(), children: Vec::new() } }
    fn is_leaf(&self) -> bool { self.children.is_empty() }

    /// Splits the full child `index` around its middle pair, which moves up into `self`.
    fn split_child(&mut self, index: usize) {
        let child = &mut self.children[index];
        let mut right = Node { entries: Vec::with_capacity(CAPACITY), children: Vec::new() };
        right.entries.extend(child.entries.drain(B..));
        let middle = child.entries.pop().expect("a full B-tree node has a middle pair");
        if !child.is_leaf() {
            right.children.reserve_exact(CAPACITY + 1);
            right.children.extend(child.children.drain(B..));
        }
        self.entries.insert(index, middle);
        self.children.insert(index + 1, right);
    }

    /// Moves separator `index` and the whole of child `index + 1` into child `index`.
    fn merge_children(&mut self, index: usize) {
        let right = self.children.remove(index + 1);
        let separator = self.entries.remove(index);
        let left = &mut self.children[index];
        left.entries.push(separator);
        left.entries.extend(right.entries);
        left.children.extend(right.children);
    }

    /// Gives child `index`, which holds `MIN` pairs, one more: rotated in through the separator
    /// from a sibling that can spare one, or else merged with a sibling. Returns where the child's
    /// pairs are now (`index - 1` after a merge into the left sibling).
    fn fill_child(&mut self, index: usize) -> usize {
        if index > 0 && self.children[index - 1].entries.len() > MIN {
            let (before, after) = self.children.split_at_mut(index);
            let (left, child) = (&mut before[index - 1], &mut after[0]);
            let lent = left.entries.pop().expect("a sibling above MIN has pairs");
            let separator = core::mem::replace(&mut self.entries[index - 1], lent);
            child.entries.insert(0, separator);
            if let Some(grandchild) = left.children.pop() {
                child.children.insert(0, grandchild);
            }
            index
        } else if index + 1 < self.children.len() && self.children[index + 1].entries.len() > MIN {
            let (before, after) = self.children.split_at_mut(index + 1);
            let (child, right) = (&mut before[index], &mut after[0]);
            let lent = right.entries.remove(0);
            let separator = core::mem::replace(&mut self.entries[index], lent);
            child.entries.push(separator);
            if !right.is_leaf() {
                child.children.push(right.children.remove(0));
            }
            index
        } else if index + 1 < self.children.len() {
            self.merge_children(index);
            index
        } else {
            self.merge_children(index - 1);
            index - 1
        }
    }

    /// Removes the pair `find` picks. `find` sees a node's pairs and whether it is a leaf, and
    /// answers `Ok(i)` (pair `i` of this node) or `Err(i)` (look in child `i`). Every child it
    /// descends into holds more than `MIN` pairs first, so the removal never rebalances upwards;
    /// the map only collapses an emptied root afterwards.
    fn remove_by<F: Fn(&[(K, V)], bool) -> Result<usize, usize>>(&mut self, find: &F) -> Option<(K, V)> {
        let leaf = self.is_leaf();
        match find(&self.entries, leaf) {
            Ok(index) if leaf => Some(self.entries.remove(index)),
            Err(_) if leaf => None,
            Ok(index) => {
                if self.children[index].entries.len() > MIN {
                    let before = self.children[index].remove_by(&pick_last::<K, V>).expect("a B-tree child is never empty");
                    Some(core::mem::replace(&mut self.entries[index], before))
                } else if self.children[index + 1].entries.len() > MIN {
                    let after = self.children[index + 1].remove_by(&pick_first::<K, V>).expect("a B-tree child is never empty");
                    Some(core::mem::replace(&mut self.entries[index], after))
                } else {
                    self.merge_children(index);
                    self.children[index].remove_by(find)
                }
            }
            Err(index) => {
                let index = if self.children[index].entries.len() > MIN { index } else { self.fill_child(index) };
                self.children[index].remove_by(find)
            }
        }
    }

    /// Appends a pair greater than every key in the subtree, along the right edge, without
    /// comparing keys. `self` is not full.
    fn push_last(&mut self, pair: (K, V)) {
        if self.is_leaf() {
            self.entries.push(pair);
            return;
        }
        let mut last = self.children.len() - 1;
        if self.children[last].entries.len() == CAPACITY {
            self.split_child(last);
            last += 1;
        }
        self.children[last].push_last(pair);
    }

    /// Moves every pair of the subtree, in order, to the end of `out`.
    fn drain_into(self, out: &mut Vec<(K, V)>) {
        let Node { entries, children } = self;
        if children.is_empty() {
            out.extend(entries);
            return;
        }
        let mut children = children.into_iter();
        for pair in entries {
            if let Some(child) = children.next() {
                child.drain_into(out);
            }
            out.push(pair);
        }
        if let Some(child) = children.next() {
            child.drain_into(out);
        }
    }
}

fn pick_first<K, V>(entries: &[(K, V)], leaf: bool) -> Result<usize, usize> {
    if leaf && !entries.is_empty() { Ok(0) } else { Err(0) }
}

fn pick_last<K, V>(entries: &[(K, V)], leaf: bool) -> Result<usize, usize> {
    if leaf && !entries.is_empty() { Ok(entries.len() - 1) } else { Err(entries.len()) }
}

fn search<K: Borrow<Q>, V, Q: ?Sized + Ord>(entries: &[(K, V)], key: &Q) -> Result<usize, usize> {
    entries.binary_search_by(|(k, _)| k.borrow().cmp(key))
}

/// The first pair index of `entries` inside the lower bound, and the first one past the upper.
fn bound_indices<K: Borrow<T>, V, T: ?Sized + Ord>(entries: &[(K, V)], lower: Bound<&T>, upper: Bound<&T>) -> (usize, usize) {
    let lo = match lower {
        Bound::Unbounded => 0,
        Bound::Included(s) => entries.partition_point(|(k, _)| k.borrow() < s),
        Bound::Excluded(s) => entries.partition_point(|(k, _)| k.borrow() <= s),
    };
    let hi = match upper {
        Bound::Unbounded => entries.len(),
        Bound::Included(e) => entries.partition_point(|(k, _)| k.borrow() <= e),
        Bound::Excluded(e) => entries.partition_point(|(k, _)| k.borrow() < e),
    };
    (lo, if hi < lo { lo } else { hi })
}

/// A borrowed subtree, shared or exclusive: the iterators are written once over both.
trait Subtree: Sized {
    type K;
    type V;
    type Pair;
    type Pairs: DoubleEndedIterator<Item = Self::Pair>;
    type Children: DoubleEndedIterator<Item = Self>;
    fn entries(&self) -> &[(Self::K, Self::V)];
    /// Pairs `lo..hi` and, of an internal node, children `lo..=hi`.
    fn split(self, lo: usize, hi: usize) -> (Self::Pairs, Self::Children);
}

impl<'a, K, V> Subtree for &'a Node<K, V> {
    type K = K;
    type V = V;
    type Pair = &'a (K, V);
    type Pairs = core::slice::Iter<'a, (K, V)>;
    type Children = core::slice::Iter<'a, Node<K, V>>;
    fn entries(&self) -> &[(K, V)] { &self.entries }
    fn split(self, lo: usize, hi: usize) -> (Self::Pairs, Self::Children) {
        let end = if self.children.is_empty() { 0 } else { hi + 1 };
        (self.entries[lo..hi].iter(), self.children[if lo < end { lo } else { end }..end].iter())
    }
}

impl<'a, K, V> Subtree for &'a mut Node<K, V> {
    type K = K;
    type V = V;
    type Pair = &'a mut (K, V);
    type Pairs = core::slice::IterMut<'a, (K, V)>;
    type Children = core::slice::IterMut<'a, Node<K, V>>;
    fn entries(&self) -> &[(K, V)] { &self.entries }
    fn split(self, lo: usize, hi: usize) -> (Self::Pairs, Self::Children) {
        let Node { entries, children } = self;
        let end = if children.is_empty() { 0 } else { hi + 1 };
        (entries[lo..hi].iter_mut(), children[if lo < end { lo } else { end }..end].iter_mut())
    }
}

enum Item<N: Subtree> {
    Pair(N::Pair),
    Node(N),
}

impl<N: Subtree + Clone> Clone for Item<N> where N::Pair: Clone {
    fn clone(&self) -> Self {
        match self {
            Item::Pair(pair) => Item::Pair(pair.clone()),
            Item::Node(node) => Item::Node(node.clone()),
        }
    }
}

/// Pushes the in-order items of `pairs` and `children` onto `out`: child, pair, child, ..., child
/// for an internal node, the pairs alone for a leaf. Reversed iterators push them in reverse.
fn interleave<N: Subtree, P: Iterator<Item = N::Pair>, C: Iterator<Item = N>>(pairs: P, mut children: C, out: &mut Vec<Item<N>>) {
    if let Some(child) = children.next() {
        out.push(Item::Node(child));
    }
    for pair in pairs {
        out.push(Item::Pair(pair));
        if let Some(child) = children.next() {
            out.push(Item::Node(child));
        }
    }
}

/// The pairs still to visit, as two stacks of items: `front` (top is the next pair) and `back`
/// (top is the next pair from the end); the order is `front` top to bottom, then `back` bottom to
/// top. A subtree stays one unexpanded item until an end reaches it, so the stacks hold
/// `O(CAPACITY * depth)` items, never the whole map.
struct Walk<N: Subtree> {
    front: Vec<Item<N>>,
    back: Vec<Item<N>>,
}

impl<N: Subtree + Clone> Clone for Walk<N> where N::Pair: Clone {
    fn clone(&self) -> Self { Walk { front: self.front.clone(), back: self.back.clone() } }
}

impl<N: Subtree> Walk<N> {
    const fn empty() -> Self { Walk { front: Vec::new(), back: Vec::new() } }

    fn whole(root: N) -> Self {
        let mut walk = Self::empty();
        walk.front.push(Item::Node(root));
        walk
    }

    fn next(&mut self) -> Option<N::Pair> {
        loop {
            let item = match self.front.pop() {
                Some(item) => item,
                None if self.back.is_empty() => return None,
                None => self.back.remove(0),
            };
            match item {
                Item::Pair(pair) => return Some(pair),
                Item::Node(node) => {
                    let n = node.entries().len();
                    let (pairs, children) = node.split(0, n);
                    interleave(pairs.rev(), children.rev(), &mut self.front);
                }
            }
        }
    }

    fn next_back(&mut self) -> Option<N::Pair> {
        loop {
            let item = match self.back.pop() {
                Some(item) => item,
                None if self.front.is_empty() => return None,
                None => self.front.remove(0),
            };
            match item {
                Item::Pair(pair) => return Some(pair),
                Item::Node(node) => {
                    let n = node.entries().len();
                    let (pairs, children) = node.split(0, n);
                    interleave(pairs, children, &mut self.back);
                }
            }
        }
    }
}

impl<N: Subtree> Walk<N> where N::K: Ord {
    /// The items of the pairs inside the bounds, in order: whole subtrees stay unexpanded, and only
    /// the (at most two) paths the bounds end in are opened.
    fn bounded<T: ?Sized + Ord>(root: N, lower: Bound<&T>, upper: Bound<&T>) -> Self where N::K: Borrow<T> {
        let mut items = Vec::new();
        Self::collect(root, lower, upper, &mut items);
        items.reverse();
        Walk { front: items, back: Vec::new() }
    }

    fn collect<T: ?Sized + Ord>(node: N, lower: Bound<&T>, upper: Bound<&T>, out: &mut Vec<Item<N>>) where N::K: Borrow<T> {
        let (lo, hi) = bound_indices(node.entries(), lower, upper);
        let (pairs, mut children) = node.split(lo, hi);
        if lo == hi {
            // No pair of this node is inside: everything that is lies in child `lo`.
            if let Some(child) = children.next() {
                Self::collect(child, lower, upper, out);
            }
            return;
        }
        // Child `lo` holds keys below pair `lo` (inside the upper bound) and child `hi` keys above
        // pair `hi - 1` (inside the lower bound); the children between are inside both.
        let last = if let Some(first) = children.next() {
            Self::collect(first, lower, Bound::Unbounded, out);
            children.next_back()
        } else {
            None
        };
        for pair in pairs {
            out.push(Item::Pair(pair));
            if let Some(child) = children.next() {
                out.push(Item::Node(child));
            }
        }
        if let Some(last) = last {
            Self::collect(last, Bound::Unbounded, upper, out);
        }
    }
}

/// Panics as Rust's `BTreeMap::range` does on a range that cannot hold anything.
fn check_range<T: ?Sized + Ord, R: RangeBounds<T>>(range: &R) {
    match (range.start_bound(), range.end_bound()) {
        (Bound::Excluded(s), Bound::Excluded(e)) if s == e => panic!("range start and end are equal and excluded in BTreeMap"),
        (Bound::Included(s) | Bound::Excluded(s), Bound::Included(e) | Bound::Excluded(e)) if s > e => panic!("range start is greater than range end in BTreeMap"),
        _ => {}
    }
}

pub struct BTreeMap<K, V> {
    root: Node<K, V>,
    length: usize,
}

pub struct Iter<'a, K, V> { walk: Walk<&'a Node<K, V>>, length: usize }
pub struct IterMut<'a, K, V> { walk: Walk<&'a mut Node<K, V>>, length: usize }
pub struct Keys<'a, K, V> { inner: Iter<'a, K, V> }
pub struct Values<'a, K, V> { inner: Iter<'a, K, V> }
pub struct ValuesMut<'a, K, V> { inner: IterMut<'a, K, V> }
pub struct Range<'a, K, V> { walk: Walk<&'a Node<K, V>> }
pub struct RangeMut<'a, K, V> { walk: Walk<&'a mut Node<K, V>> }
pub type IntoIter<K, V> = crate::vec::IntoIter<(K, V)>;
pub type IntoKeys<K, V> = core::iter::Map<IntoIter<K, V>, fn((K, V)) -> K>;
pub type IntoValues<K, V> = core::iter::Map<IntoIter<K, V>, fn((K, V)) -> V>;

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        let (k, v) = self.walk.next()?;
        self.length -= 1;
        Some((k, v))
    }
    fn size_hint(&self) -> (usize, Option<usize>) { (self.length, Some(self.length)) }
}
impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
        let (k, v) = self.walk.next_back()?;
        self.length -= 1;
        Some((k, v))
    }
}
impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {}
impl<'a, K, V> FusedIterator for Iter<'a, K, V> {}
impl<'a, K, V> Clone for Iter<'a, K, V> { fn clone(&self) -> Self { Iter { walk: self.walk.clone(), length: self.length } } }

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        let (k, v) = self.walk.next()?;
        self.length -= 1;
        Some((&*k, v))
    }
    fn size_hint(&self) -> (usize, Option<usize>) { (self.length, Some(self.length)) }
}
impl<'a, K, V> DoubleEndedIterator for IterMut<'a, K, V> {
    fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
        let (k, v) = self.walk.next_back()?;
        self.length -= 1;
        Some((&*k, v))
    }
}
impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {}
impl<'a, K, V> FusedIterator for IterMut<'a, K, V> {}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;
    fn next(&mut self) -> Option<&'a K> { self.inner.next().map(|(k, _)| k) }
    fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
}
impl<'a, K, V> DoubleEndedIterator for Keys<'a, K, V> { fn next_back(&mut self) -> Option<&'a K> { self.inner.next_back().map(|(k, _)| k) } }
impl<'a, K, V> ExactSizeIterator for Keys<'a, K, V> {}
impl<'a, K, V> FusedIterator for Keys<'a, K, V> {}
impl<'a, K, V> Clone for Keys<'a, K, V> { fn clone(&self) -> Self { Keys { inner: self.inner.clone() } } }

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;
    fn next(&mut self) -> Option<&'a V> { self.inner.next().map(|(_, v)| v) }
    fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
}
impl<'a, K, V> DoubleEndedIterator for Values<'a, K, V> { fn next_back(&mut self) -> Option<&'a V> { self.inner.next_back().map(|(_, v)| v) } }
impl<'a, K, V> ExactSizeIterator for Values<'a, K, V> {}
impl<'a, K, V> FusedIterator for Values<'a, K, V> {}
impl<'a, K, V> Clone for Values<'a, K, V> { fn clone(&self) -> Self { Values { inner: self.inner.clone() } } }

impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
    type Item = &'a mut V;
    fn next(&mut self) -> Option<&'a mut V> { self.inner.next().map(|(_, v)| v) }
    fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
}
impl<'a, K, V> DoubleEndedIterator for ValuesMut<'a, K, V> { fn next_back(&mut self) -> Option<&'a mut V> { self.inner.next_back().map(|(_, v)| v) } }
impl<'a, K, V> ExactSizeIterator for ValuesMut<'a, K, V> {}
impl<'a, K, V> FusedIterator for ValuesMut<'a, K, V> {}

impl<'a, K, V> Iterator for Range<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<(&'a K, &'a V)> { self.walk.next().map(|(k, v)| (k, v)) }
}
impl<'a, K, V> DoubleEndedIterator for Range<'a, K, V> { fn next_back(&mut self) -> Option<(&'a K, &'a V)> { self.walk.next_back().map(|(k, v)| (k, v)) } }
impl<'a, K, V> FusedIterator for Range<'a, K, V> {}
impl<'a, K, V> Clone for Range<'a, K, V> { fn clone(&self) -> Self { Range { walk: self.walk.clone() } } }

impl<'a, K, V> Iterator for RangeMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<(&'a K, &'a mut V)> { self.walk.next().map(|(k, v)| (&*k, v)) }
}
impl<'a, K, V> DoubleEndedIterator for RangeMut<'a, K, V> { fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> { self.walk.next_back().map(|(k, v)| (&*k, v)) } }
impl<'a, K, V> FusedIterator for RangeMut<'a, K, V> {}

impl<K, V> BTreeMap<K, V> {
    pub const fn new() -> Self { Self { root: Node::new(), length: 0 } }
    pub fn clear(&mut self) { *self = Self::new(); }
    pub fn len(&self) -> usize { self.length }
    pub fn is_empty(&self) -> bool { self.length == 0 }
    pub fn iter(&self) -> Iter<'_, K, V> {
        if self.length == 0 { Iter { walk: Walk::empty(), length: 0 } } else { Iter { walk: Walk::whole(&self.root), length: self.length } }
    }
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        let length = self.length;
        if length == 0 { IterMut { walk: Walk::empty(), length: 0 } } else { IterMut { walk: Walk::whole(&mut self.root), length } }
    }
    pub fn keys(&self) -> Keys<'_, K, V> { Keys { inner: self.iter() } }
    pub fn values(&self) -> Values<'_, K, V> { Values { inner: self.iter() } }
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> { ValuesMut { inner: self.iter_mut() } }
    pub fn into_keys(self) -> IntoKeys<K, V> {
        fn key<K, V>((k, _): (K, V)) -> K { k }
        self.into_iter().map(key::<K, V>)
    }
    pub fn into_values(self) -> IntoValues<K, V> {
        fn value<K, V>((_, v): (K, V)) -> V { v }
        self.into_iter().map(value::<K, V>)
    }
    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        let mut node = &self.root;
        while let Some(child) = node.children.first() {
            node = child;
        }
        node.entries.first().map(|(k, v)| (k, v))
    }
    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        let mut node = &self.root;
        while let Some(child) = node.children.last() {
            node = child;
        }
        node.entries.last().map(|(k, v)| (k, v))
    }
    pub fn pop_first(&mut self) -> Option<(K, V)> { self.remove_by(&pick_first::<K, V>) }
    pub fn pop_last(&mut self) -> Option<(K, V)> { self.remove_by(&pick_last::<K, V>) }
    /// Keeps the pairs `f` accepts: one pass over the pairs in order, then the tree is rebuilt
    /// along its right edge (no key comparisons).
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut f: F) {
        let mut pairs = self.take_pairs();
        pairs.retain_mut(|(k, v)| f(&*k, v));
        self.extend_sorted(pairs);
    }

    fn remove_by<F: Fn(&[(K, V)], bool) -> Result<usize, usize>>(&mut self, find: &F) -> Option<(K, V)> {
        let removed = self.root.remove_by(find);
        if self.root.entries.is_empty() {
            if let Some(only) = self.root.children.pop() {
                self.root = only;
            }
        }
        if removed.is_some() {
            self.length -= 1;
        }
        removed
    }

    /// Every pair, in order, leaving the map empty.
    fn take_pairs(&mut self) -> Vec<(K, V)> {
        let mut pairs = Vec::with_capacity(self.length);
        core::mem::replace(self, Self::new()).root.drain_into(&mut pairs);
        pairs
    }

    /// Appends `pairs`, already in strictly ascending key order and above every key here.
    fn extend_sorted(&mut self, pairs: Vec<(K, V)>) {
        for pair in pairs {
            if self.root.entries.len() == CAPACITY {
                let old = core::mem::replace(&mut self.root, Node::new());
                self.root.children.push(old);
                self.root.split_child(0);
            }
            self.root.push_last(pair);
            self.length += 1;
        }
    }
}

impl<K: Ord, V> BTreeMap<K, V> {
    fn find<Q: ?Sized + Ord>(&self, key: &Q) -> Option<&(K, V)> where K: Borrow<Q> {
        let mut node = &self.root;
        loop {
            match search(&node.entries, key) {
                Ok(index) => return Some(&node.entries[index]),
                Err(index) => node = node.children.get(index)?,
            }
        }
    }
    fn find_mut<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<&mut (K, V)> where K: Borrow<Q> {
        let mut node = &mut self.root;
        loop {
            match search(&node.entries, key) {
                Ok(index) => return Some(&mut node.entries[index]),
                Err(index) => node = node.children.get_mut(index)?,
            }
        }
    }
    pub fn get<Q: ?Sized + Ord>(&self, key: &Q) -> Option<&V> where K: Borrow<Q> { self.find(key).map(|(_, v)| v) }
    pub fn get_key_value<Q: ?Sized + Ord>(&self, key: &Q) -> Option<(&K, &V)> where K: Borrow<Q> { self.find(key).map(|(k, v)| (k, v)) }
    pub fn get_mut<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<&mut V> where K: Borrow<Q> { self.find_mut(key).map(|(_, v)| v) }
    pub fn contains_key<Q: ?Sized + Ord>(&self, key: &Q) -> bool where K: Borrow<Q> { self.find(key).is_some() }

    /// Inserts or replaces, and returns the value's slot and the value it replaced. A full root
    /// is split first and every full child before the descent enters it, so the leaf that takes a
    /// new pair always has room.
    fn put(&mut self, key: K, value: V) -> (&mut V, Option<V>) {
        if self.root.entries.len() == CAPACITY {
            let old = core::mem::replace(&mut self.root, Node::new());
            self.root.children.push(old);
            self.root.split_child(0);
        }
        let (slot, old) = Self::put_in(&mut self.root, key, value);
        if old.is_none() {
            self.length += 1;
        }
        (slot, old)
    }

    fn put_in(node: &mut Node<K, V>, key: K, value: V) -> (&mut V, Option<V>) {
        match search(&node.entries, &key) {
            Ok(index) => {
                let old = core::mem::replace(&mut node.entries[index].1, value);
                (&mut node.entries[index].1, Some(old))
            }
            Err(index) if node.is_leaf() => {
                node.entries.insert(index, (key, value));
                (&mut node.entries[index].1, None)
            }
            Err(mut index) => {
                if node.children[index].entries.len() == CAPACITY {
                    node.split_child(index);
                    match node.entries[index].0.cmp(&key) {
                        Ordering::Equal => {
                            let old = core::mem::replace(&mut node.entries[index].1, value);
                            return (&mut node.entries[index].1, Some(old));
                        }
                        Ordering::Less => index += 1,
                        Ordering::Greater => {}
                    }
                }
                Self::put_in(&mut node.children[index], key, value)
            }
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> { self.put(key, value).1 }
    pub fn remove<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<V> where K: Borrow<Q> {
        self.remove_entry(key).map(|(_, value)| value)
    }
    pub fn remove_entry<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<(K, V)> where K: Borrow<Q> {
        self.remove_by(&|entries: &[(K, V)], _leaf: bool| search(entries, key))
    }
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        if self.contains_key(&key) { Entry::Occupied(OccupiedEntry { map: self, key }) } else { Entry::Vacant(VacantEntry { map: self, key }) }
    }
    pub fn range<T: ?Sized + Ord, R: RangeBounds<T>>(&self, range: R) -> Range<'_, K, V> where K: Borrow<T> {
        check_range(&range);
        Range { walk: Walk::bounded(&self.root, range.start_bound(), range.end_bound()) }
    }
    pub fn range_mut<T: ?Sized + Ord, R: RangeBounds<T>>(&mut self, range: R) -> RangeMut<'_, K, V> where K: Borrow<T> {
        check_range(&range);
        RangeMut { walk: Walk::bounded(&mut self.root, range.start_bound(), range.end_bound()) }
    }
    /// Moves every pair of `other` in; on an equal key `other`'s pair wins. Both sides are
    /// sorted, so this is one linear merge and a rebuild along the right edge.
    pub fn append(&mut self, other: &mut Self) {
        if other.is_empty() {
            return;
        }
        if self.is_empty() {
            core::mem::swap(self, other);
            return;
        }
        let (ours, theirs) = (self.take_pairs(), other.take_pairs());
        let mut merged = Vec::with_capacity(ours.len() + theirs.len());
        let (mut a, mut b) = (ours.into_iter(), theirs.into_iter());
        let (mut x, mut y) = (a.next(), b.next());
        loop {
            match (x.take(), y.take()) {
                (Some(p), Some(q)) => match p.0.cmp(&q.0) {
                    Ordering::Less => { merged.push(p); x = a.next(); y = Some(q); }
                    Ordering::Greater => { merged.push(q); x = Some(p); y = b.next(); }
                    Ordering::Equal => { merged.push(q); x = a.next(); y = b.next(); }
                },
                (Some(p), None) => { merged.push(p); merged.extend(a); break; }
                (None, Some(q)) => { merged.push(q); merged.extend(b); break; }
                (None, None) => break,
            }
        }
        self.extend_sorted(merged);
    }
}

impl<K, V> Default for BTreeMap<K, V> { fn default() -> Self { Self::new() } }
impl<K: Clone, V: Clone> Clone for BTreeMap<K, V> { fn clone(&self) -> Self { Self { root: self.root.clone(), length: self.length } } }
impl<K: PartialEq, V: PartialEq> PartialEq for BTreeMap<K, V> { fn eq(&self, o: &Self) -> bool { self.length == o.length && self.iter().eq(o.iter()) } }
impl<K: Eq, V: Eq> Eq for BTreeMap<K, V> {}
impl<K: PartialOrd, V: PartialOrd> PartialOrd for BTreeMap<K, V> { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { self.iter().partial_cmp(o.iter()) } }
impl<K: Ord, V: Ord> Ord for BTreeMap<K, V> { fn cmp(&self, o: &Self) -> Ordering { self.iter().cmp(o.iter()) } }
impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for BTreeMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_map().entries(self.iter()).finish() }
}
impl<K, V> FromIterator<(K, V)> for BTreeMap<K, V> where K: Ord {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self { let mut m = Self::new(); m.extend(iter); m }
}
impl<K: Ord, V> Extend<(K, V)> for BTreeMap<K, V> { fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, it: I) { for (k, v) in it { self.insert(k, v); } } }
impl<'a, K, V> IntoIterator for &'a BTreeMap<K, V> { type Item = (&'a K, &'a V); type IntoIter = Iter<'a, K, V>; fn into_iter(self) -> Iter<'a, K, V> { self.iter() } }
impl<'a, K, V> IntoIterator for &'a mut BTreeMap<K, V> { type Item = (&'a K, &'a mut V); type IntoIter = IterMut<'a, K, V>; fn into_iter(self) -> IterMut<'a, K, V> { self.iter_mut() } }
impl<K: Ord + Borrow<Q>, V, Q: ?Sized + Ord> Index<&Q> for BTreeMap<K, V> { type Output = V; fn index(&self, key: &Q) -> &V { self.get(key).expect("key not found") } }
impl<K, V> IntoIterator for BTreeMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(mut self) -> IntoIter<K, V> { self.take_pairs().into_iter() }
}

pub enum Entry<'a, K: 'a, V: 'a> { Occupied(OccupiedEntry<'a, K, V>), Vacant(VacantEntry<'a, K, V>) }
/// A present key: the map and the probe key, which finds the pair again on each access.
pub struct OccupiedEntry<'a, K: 'a, V: 'a> { map: &'a mut BTreeMap<K, V>, key: K }
pub struct VacantEntry<'a, K: 'a, V: 'a> { map: &'a mut BTreeMap<K, V>, key: K }
impl<'a, K: Ord, V> Entry<'a, K, V> {
    pub fn key(&self) -> &K { match self { Self::Occupied(e) => e.key(), Self::Vacant(e) => e.key() } }
    pub fn or_insert(self, value: V) -> &'a mut V { self.or_insert_with(|| value) }
    pub fn or_insert_with<F: FnOnce() -> V>(self, f: F) -> &'a mut V { match self { Self::Occupied(e) => e.into_mut(), Self::Vacant(e) => e.insert(f()) } }
    pub fn or_default(self) -> &'a mut V where V: Default { self.or_insert_with(V::default) }
    pub fn and_modify<F: FnOnce(&mut V)>(self, f: F) -> Self { match self { Self::Occupied(mut e) => { f(e.get_mut()); Self::Occupied(e) }, vacant => vacant } }
}
impl<'a, K: Ord, V> OccupiedEntry<'a, K, V> {
    fn pair(&self) -> &(K, V) { self.map.find(&self.key).expect("an occupied entry's key is in the map") }
    fn pair_mut(&mut self) -> &mut (K, V) { self.map.find_mut(&self.key).expect("an occupied entry's key is in the map") }
    pub fn key(&self) -> &K { &self.pair().0 }
    pub fn get(&self) -> &V { &self.pair().1 }
    pub fn get_mut(&mut self) -> &mut V { &mut self.pair_mut().1 }
    pub fn into_mut(self) -> &'a mut V { &mut self.map.find_mut(&self.key).expect("an occupied entry's key is in the map").1 }
    pub fn insert(&mut self, value: V) -> V { core::mem::replace(self.get_mut(), value) }
    pub fn remove(self) -> V { self.remove_entry().1 }
    pub fn remove_entry(self) -> (K, V) { self.map.remove_entry(&self.key).expect("an occupied entry's key is in the map") }
}
impl<'a, K: Ord, V> VacantEntry<'a, K, V> {
    pub fn key(&self) -> &K { &self.key }
    pub fn into_key(self) -> K { self.key }
    pub fn insert(self, value: V) -> &'a mut V { self.map.put(self.key, value).0 }
}

impl<K: core::hash::Hash, V: core::hash::Hash> core::hash::Hash for BTreeMap<K, V> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.len());
        for (k, v) in self.iter() { k.hash(state); v.hash(state); }
    }
}
impl<K: Ord, V, const N: usize> From<[(K, V); N]> for BTreeMap<K, V> {
    fn from(entries: [(K, V); N]) -> Self { let mut map = BTreeMap::new(); for (k, v) in entries { map.insert(k, v); } map }
}
