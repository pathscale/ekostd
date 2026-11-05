use crate::collections::btree_map::BTreeMap;
use core::cmp::Ordering;
use core::fmt;
use core::iter::FromIterator;

pub struct BTreeSet<T: Ord> { map: BTreeMap<T, ()> }
impl<T: Ord> BTreeSet<T> {
    pub const fn new() -> Self { Self { map: BTreeMap::new() } }
    pub fn clear(&mut self) { self.map.clear(); }
    pub fn len(&self) -> usize { self.map.len() }
    pub fn is_empty(&self) -> bool { self.map.is_empty() }
    pub fn contains<Q: ?Sized + Ord>(&self, value: &Q) -> bool where T: core::borrow::Borrow<Q> { self.map.contains_key(value) }
    pub fn get<Q: ?Sized + Ord>(&self, value: &Q) -> Option<&T> where T: core::borrow::Borrow<Q> { self.map.get_key_value(value).map(|(k, _)| k) }
    pub fn insert(&mut self, value: T) -> bool { self.map.insert(value, ()).is_none() }
    pub fn remove<Q: ?Sized + Ord>(&mut self, value: &Q) -> bool where T: core::borrow::Borrow<Q> { self.map.remove(value).is_some() }
    pub fn take<Q: ?Sized + Ord>(&mut self, value: &Q) -> Option<T> where T: core::borrow::Borrow<Q> { self.map.remove_entry(value).map(|(k, _)| k) }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator { self.map.keys() }
    pub fn first(&self) -> Option<&T> { self.map.first_key_value().map(|(k, _)| k) }
    pub fn last(&self) -> Option<&T> { self.map.last_key_value().map(|(k, _)| k) }
    pub fn pop_first(&mut self) -> Option<T> { self.map.pop_first().map(|(k, _)| k) }
    pub fn pop_last(&mut self) -> Option<T> { self.map.pop_last().map(|(k, _)| k) }
    pub fn append(&mut self, other: &mut Self) { while let Some(v) = other.pop_first() { self.insert(v); } }
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut f: F) { self.map.retain(|k, _| f(k)); }
}
impl<T: Ord> Default for BTreeSet<T> { fn default() -> Self { Self::new() } }
impl<T: Ord + Clone> Clone for BTreeSet<T> { fn clone(&self) -> Self { Self { map: self.map.clone() } } }
impl<T: Ord + PartialEq> PartialEq for BTreeSet<T> { fn eq(&self, o: &Self) -> bool { self.iter().eq(o.iter()) } }
impl<T: Ord + Eq> Eq for BTreeSet<T> {}
impl<T: Ord + PartialOrd> PartialOrd for BTreeSet<T> { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { self.iter().partial_cmp(o.iter()) } }
impl<T: Ord> Ord for BTreeSet<T> { fn cmp(&self, o: &Self) -> Ordering { self.iter().cmp(o.iter()) } }
impl<T: Ord + fmt::Debug> fmt::Debug for BTreeSet<T> { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_set().entries(self.iter()).finish() } }
impl<T: Ord> FromIterator<T> for BTreeSet<T> { fn from_iter<I: IntoIterator<Item=T>>(it:I)->Self { let mut s=Self::new(); s.extend(it); s } }
impl<T: Ord> Extend<T> for BTreeSet<T> { fn extend<I:IntoIterator<Item=T>>(&mut self,it:I) { for v in it { self.insert(v); } } }
impl<'a, T: Ord> IntoIterator for &'a BTreeSet<T> { type Item=&'a T; type IntoIter=core::iter::Map<core::slice::Iter<'a,(T,())>,fn(&(T,()))->&T>; fn into_iter(self)->Self::IntoIter { fn key<T>((k,_):&(T,()))->&T {k} self.map.iter_raw().map(key::<T>) } }
impl<T: Ord> IntoIterator for BTreeSet<T> { type Item=T; type IntoIter=core::iter::Map<<BTreeMap<T,()> as IntoIterator>::IntoIter, fn((T,()))->T>; fn into_iter(self)->Self::IntoIter { fn key<T>((k,_):(T,()))->T {k} self.map.into_iter().map(key::<T>) } }
