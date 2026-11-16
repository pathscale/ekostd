use crate::collections::btree_map::BTreeMap;
use core::cmp::Ordering;
use core::fmt;
use core::iter::{FromIterator, FusedIterator};

pub struct BTreeSet<T> { map: BTreeMap<T, ()> }
impl<T> BTreeSet<T> {
    pub const fn new() -> Self { Self { map: BTreeMap::new() } }
    pub fn clear(&mut self) { self.map.clear(); }
    pub fn len(&self) -> usize { self.map.len() }
    pub fn is_empty(&self) -> bool { self.map.is_empty() }
    pub fn iter(&self) -> Iter<'_, T> { Iter { inner: self.map.keys() } }
    pub fn first(&self) -> Option<&T> { self.map.first_key_value().map(|(k, _)| k) }
    pub fn last(&self) -> Option<&T> { self.map.last_key_value().map(|(k, _)| k) }
    pub fn pop_first(&mut self) -> Option<T> { self.map.pop_first().map(|(k, _)| k) }
    pub fn pop_last(&mut self) -> Option<T> { self.map.pop_last().map(|(k, _)| k) }
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut f: F) { self.map.retain(|k, _| f(k)); }
}
impl<T: Ord> BTreeSet<T> {
    pub fn contains<Q: ?Sized + Ord>(&self, value: &Q) -> bool where T: core::borrow::Borrow<Q> { self.map.contains_key(value) }
    pub fn get<Q: ?Sized + Ord>(&self, value: &Q) -> Option<&T> where T: core::borrow::Borrow<Q> { self.map.get_key_value(value).map(|(k, _)| k) }
    pub fn insert(&mut self, value: T) -> bool { self.map.insert(value, ()).is_none() }
    pub fn remove<Q: ?Sized + Ord>(&mut self, value: &Q) -> bool where T: core::borrow::Borrow<Q> { self.map.remove(value).is_some() }
    pub fn take<Q: ?Sized + Ord>(&mut self, value: &Q) -> Option<T> where T: core::borrow::Borrow<Q> { self.map.remove_entry(value).map(|(k, _)| k) }
    pub fn is_disjoint(&self, other: &Self) -> bool { self.iter().all(|value| !other.contains(value)) }
    /// The values in both sets, in ascending order.
    pub fn intersection<'a>(&'a self, other: &'a Self) -> crate::vec::IntoIter<&'a T> {
        let mut common = crate::vec::Vec::new();
        for value in self.iter() {
            if other.contains(value) { common.push(value); }
        }
        common.into_iter()
    }
    pub fn append(&mut self, other: &mut Self) { self.map.append(&mut other.map); }
}
impl<T: Ord> Default for BTreeSet<T> { fn default() -> Self { Self::new() } }
impl<T: Clone> Clone for BTreeSet<T> { fn clone(&self) -> Self { Self { map: self.map.clone() } } }
impl<T: PartialEq> PartialEq for BTreeSet<T> { fn eq(&self, o: &Self) -> bool { self.iter().eq(o.iter()) } }
impl<T: Eq> Eq for BTreeSet<T> {}
impl<T: PartialOrd> PartialOrd for BTreeSet<T> { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { self.iter().partial_cmp(o.iter()) } }
impl<T: Ord> Ord for BTreeSet<T> { fn cmp(&self, o: &Self) -> Ordering { self.iter().cmp(o.iter()) } }
impl<T: fmt::Debug> fmt::Debug for BTreeSet<T> { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_set().entries(self.iter()).finish() } }
impl<T: Ord> FromIterator<T> for BTreeSet<T> { fn from_iter<I: IntoIterator<Item=T>>(it:I)->Self { let mut s=Self::new(); s.extend(it); s } }
impl<T: Ord> Extend<T> for BTreeSet<T> { fn extend<I:IntoIterator<Item=T>>(&mut self,it:I) { for v in it { self.insert(v); } } }
impl<'a, T: Ord> IntoIterator for &'a BTreeSet<T> { type Item=&'a T; type IntoIter=Iter<'a, T>; fn into_iter(self)->Self::IntoIter { self.iter() } }
impl<T: Ord> IntoIterator for BTreeSet<T> { type Item=T; type IntoIter=core::iter::Map<<BTreeMap<T,()> as IntoIterator>::IntoIter, fn((T,()))->T>; fn into_iter(self)->Self::IntoIter { fn key<T>((k,_):(T,()))->T {k} self.map.into_iter().map(key::<T>) } }

impl<T: Ord> BTreeSet<T> {
    pub fn is_subset(&self, other: &Self) -> bool { self.iter().all(|value| other.contains(value)) }
    /// The values in this set and not in `other`, in ascending order.
    pub fn difference<'a>(&'a self, other: &'a Self) -> crate::vec::IntoIter<&'a T> {
        let mut only = crate::vec::Vec::new();
        for value in self.iter() {
            if !other.contains(value) { only.push(value); }
        }
        only.into_iter()
    }
}
impl<T: Ord, const N: usize> From<[T; N]> for BTreeSet<T> {
    fn from(values: [T; N]) -> Self { let mut set = BTreeSet::new(); for value in values { set.insert(value); } set }
}
impl<T: core::hash::Hash> core::hash::Hash for BTreeSet<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.len());
        for value in self.iter() { value.hash(state); }
    }
}

pub type Range<'a, T> = core::iter::Map<crate::collections::btree_map::Range<'a, T, ()>, fn((&'a T, &'a ())) -> &'a T>;

impl<T: Ord> BTreeSet<T> {
    pub fn range<K: ?Sized + Ord, R: core::ops::RangeBounds<K>>(&self, range: R) -> Range<'_, T> where T: core::borrow::Borrow<K> {
        fn key<'a, T>((key, _): (&'a T, &'a ())) -> &'a T { key }
        self.map.range(range).map(key::<T>)
    }

    /// The values in either set, in ascending order, each once.
    pub fn union<'a>(&'a self, other: &'a Self) -> crate::vec::IntoIter<&'a T> {
        let mut all = crate::vec::Vec::with_capacity(self.len() + other.len());
        let mut left = self.iter();
        let mut right = other.iter();
        let mut a = left.next();
        let mut b = right.next();
        loop {
            match (a, b) {
                (Some(x), Some(y)) => match x.cmp(y) {
                    Ordering::Less => { all.push(x); a = left.next(); }
                    Ordering::Greater => { all.push(y); b = right.next(); }
                    Ordering::Equal => { all.push(x); a = left.next(); b = right.next(); }
                },
                (Some(x), None) => { all.push(x); a = left.next(); }
                (None, Some(y)) => { all.push(y); b = right.next(); }
                (None, None) => break,
            }
        }
        all.into_iter()
    }
}
impl<'a, T: 'a + Ord + Copy> Extend<&'a T> for BTreeSet<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) { for value in iter { self.insert(*value); } }
}

/// The values of a `BTreeSet`, in ascending order (from either end).
pub struct Iter<'a, T> { inner: crate::collections::btree_map::Keys<'a, T, ()> }
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> { self.inner.next() }
    fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
}
impl<'a, T> DoubleEndedIterator for Iter<'a, T> { fn next_back(&mut self) -> Option<&'a T> { self.inner.next_back() } }
impl<'a, T> ExactSizeIterator for Iter<'a, T> {}
impl<'a, T> FusedIterator for Iter<'a, T> {}
impl<'a, T> Clone for Iter<'a, T> { fn clone(&self) -> Self { Iter { inner: self.inner.clone() } } }

impl<T> BTreeSet<T> {
    /// A set of `values`, which come in strictly ascending order, appended along the right edge.
    fn from_sorted<I: Iterator<Item = T>>(values: I) -> Self {
        Self { map: BTreeMap::from_sorted(values.map(|value| (value, ())).collect::<crate::vec::Vec<(T, ())>>()) }
    }
}

impl<T: Ord> BTreeSet<T> {
    pub fn is_superset(&self, other: &Self) -> bool { other.is_subset(self) }
    /// The values in exactly one of the two sets, in ascending order: one merge of both ordered walks.
    pub fn symmetric_difference<'a>(&'a self, other: &'a Self) -> crate::vec::IntoIter<&'a T> {
        let mut only = crate::vec::Vec::new();
        let mut left = self.iter();
        let mut right = other.iter();
        let mut a = left.next();
        let mut b = right.next();
        loop {
            match (a, b) {
                (Some(x), Some(y)) => match x.cmp(y) {
                    Ordering::Less => { only.push(x); a = left.next(); }
                    Ordering::Greater => { only.push(y); b = right.next(); }
                    Ordering::Equal => { a = left.next(); b = right.next(); }
                },
                (Some(x), None) => { only.push(x); a = left.next(); }
                (None, Some(y)) => { only.push(y); b = right.next(); }
                (None, None) => break,
            }
        }
        only.into_iter()
    }
    /// Adds `value`, putting it in place of an equal stored value, which is returned.
    pub fn replace(&mut self, value: T) -> Option<T> {
        match self.map.replace_key(value) {
            Ok(old) => Some(old),
            Err(value) => {
                self.map.insert(value, ());
                None
            }
        }
    }
    /// Moves every value at or above `value` into a new set and returns it; the rest stay.
    pub fn split_off<Q: ?Sized + Ord>(&mut self, value: &Q) -> Self where T: core::borrow::Borrow<Q> {
        Self { map: self.map.split_off(value) }
    }
}

impl<T: Ord + Clone> core::ops::BitOr<&BTreeSet<T>> for &BTreeSet<T> {
    type Output = BTreeSet<T>;
    /// The union, copied in ascending order.
    fn bitor(self, rhs: &BTreeSet<T>) -> BTreeSet<T> { BTreeSet::from_sorted(self.union(rhs).cloned()) }
}
impl<T: Ord + Clone> core::ops::BitAnd<&BTreeSet<T>> for &BTreeSet<T> {
    type Output = BTreeSet<T>;
    /// The intersection, copied in ascending order.
    fn bitand(self, rhs: &BTreeSet<T>) -> BTreeSet<T> { BTreeSet::from_sorted(self.intersection(rhs).cloned()) }
}
impl<T: Ord + Clone> core::ops::Sub<&BTreeSet<T>> for &BTreeSet<T> {
    type Output = BTreeSet<T>;
    /// The values of `self` not in `rhs`, copied in ascending order.
    fn sub(self, rhs: &BTreeSet<T>) -> BTreeSet<T> { BTreeSet::from_sorted(self.difference(rhs).cloned()) }
}
impl<T: Ord + Clone> core::ops::BitXor<&BTreeSet<T>> for &BTreeSet<T> {
    type Output = BTreeSet<T>;
    /// The symmetric difference, copied in ascending order.
    fn bitxor(self, rhs: &BTreeSet<T>) -> BTreeSet<T> { BTreeSet::from_sorted(self.symmetric_difference(rhs).cloned()) }
}
