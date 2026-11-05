use crate::vec::Vec;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::iter::FromIterator;
use core::ops::Index;

pub struct BTreeMap<K, V> { entries: Vec<(K, V)> }

pub type Iter<'a, K, V> = core::iter::Map<core::slice::Iter<'a, (K, V)>, fn(&(K, V)) -> (&K, &V)>;
pub type IterMut<'a, K, V> = core::iter::Map<core::slice::IterMut<'a, (K, V)>, fn(&mut (K, V)) -> (&K, &mut V)>;
pub type Keys<'a, K, V> = core::iter::Map<core::slice::Iter<'a, (K, V)>, fn(&(K, V)) -> &K>;
pub type Values<'a, K, V> = core::iter::Map<core::slice::Iter<'a, (K, V)>, fn(&(K, V)) -> &V>;
pub type ValuesMut<'a, K, V> = core::iter::Map<core::slice::IterMut<'a, (K, V)>, fn(&mut (K, V)) -> &mut V>;

fn map_iter<K, V>((k, v): &(K, V)) -> (&K, &V) { (k, v) }
fn map_iter_mut<K, V>((k, v): &mut (K, V)) -> (&K, &mut V) { (&*k, v) }
fn map_key<K, V>((k, _): &(K, V)) -> &K { k }
fn map_val<K, V>((_, v): &(K, V)) -> &V { v }
fn map_val_mut<K, V>((_, v): &mut (K, V)) -> &mut V { v }

impl<K, V> BTreeMap<K, V> {
    pub const fn new() -> Self { Self { entries: Vec::new() } }
    fn insert_at(&mut self, index: usize, pair: (K, V)) {
        let mut old = core::mem::take(&mut self.entries).into_iter();
        let mut entries = Vec::with_capacity(old.size_hint().0 + 1);
        for _ in 0..index { entries.push(old.next().expect("map index out of bounds")); }
        entries.push(pair);
        entries.extend(old);
        self.entries = entries;
    }
    fn remove_at(&mut self, index: usize) -> (K, V) {
        let mut old = core::mem::take(&mut self.entries).into_iter();
        let mut entries = Vec::with_capacity(old.size_hint().0);
        for _ in 0..index { entries.push(old.next().expect("map index out of bounds")); }
        let removed = old.next().expect("map index out of bounds");
        entries.extend(old);
        self.entries = entries;
        removed
    }
    pub(super) fn iter_raw(&self) -> core::slice::Iter<'_, (K, V)> { self.entries.iter() }
    pub fn clear(&mut self) { self.entries.clear(); }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn iter(&self) -> Iter<'_, K, V> {
        self.entries.iter().map(map_iter::<K, V>)
    }
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        self.entries.iter_mut().map(map_iter_mut::<K, V>)
    }
    pub fn keys(&self) -> Keys<'_, K, V> { self.entries.iter().map(map_key::<K, V>) }
    pub fn values(&self) -> Values<'_, K, V> { self.entries.iter().map(map_val::<K, V>) }
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> { self.entries.iter_mut().map(map_val_mut::<K, V>) }
    pub fn first_key_value(&self) -> Option<(&K, &V)> { self.entries.first().map(|(k, v)| (k, v)) }
    pub fn last_key_value(&self) -> Option<(&K, &V)> { self.entries.last().map(|(k, v)| (k, v)) }
    pub fn pop_first(&mut self) -> Option<(K, V)> { if self.is_empty() { None } else { Some(self.remove_at(0)) } }
    pub fn pop_last(&mut self) -> Option<(K, V)> { self.entries.pop() }
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut f: F) {
        let mut i = 0;
        while i < self.entries.len() {
            let keep = {
                let (k, v) = &mut self.entries[i];
                f(&*k, v)
            };
            if keep { i += 1; } else { self.remove_at(i); }
        }
    }
}

impl<K: Ord, V> BTreeMap<K, V> {
    fn search<Q: ?Sized + Ord>(&self, key: &Q) -> Result<usize, usize> where K: Borrow<Q> {
        self.entries.binary_search_by(|(k, _)| k.borrow().cmp(key))
    }
    pub fn get<Q: ?Sized + Ord>(&self, key: &Q) -> Option<&V> where K: Borrow<Q> {
        self.search(key).ok().map(|i| &self.entries[i].1)
    }
    pub fn get_key_value<Q: ?Sized + Ord>(&self, key: &Q) -> Option<(&K, &V)> where K: Borrow<Q> {
        self.search(key).ok().map(|i| (&self.entries[i].0, &self.entries[i].1))
    }
    pub fn get_mut<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<&mut V> where K: Borrow<Q> {
        self.search(key).ok().map(move |i| &mut self.entries[i].1)
    }
    pub fn contains_key<Q: ?Sized + Ord>(&self, key: &Q) -> bool where K: Borrow<Q> { self.search(key).is_ok() }
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.search(&key) {
            Ok(i) => Some(core::mem::replace(&mut self.entries[i].1, value)),
            Err(i) => { self.insert_at(i, (key, value)); None }
        }
    }
    pub fn remove<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<V> where K: Borrow<Q> {
        self.remove_entry(key).map(|(_, value)| value)
    }
    pub fn remove_entry<Q: ?Sized + Ord>(&mut self, key: &Q) -> Option<(K, V)> where K: Borrow<Q> {
        self.search(key).ok().map(|i| self.remove_at(i))
    }
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        match self.search(&key) {
            Ok(index) => Entry::Occupied(OccupiedEntry { map: self, index }),
            Err(index) => Entry::Vacant(VacantEntry { map: self, key, index }),
        }
    }
    pub fn append(&mut self, other: &mut Self) { while let Some((k, v)) = other.pop_first() { self.insert(k, v); } }
}

impl<K, V> Default for BTreeMap<K, V> { fn default() -> Self { Self::new() } }
impl<K: Clone, V: Clone> Clone for BTreeMap<K, V> { fn clone(&self) -> Self { Self { entries: self.entries.iter().cloned().collect() } } }
impl<K: PartialEq, V: PartialEq> PartialEq for BTreeMap<K, V> { fn eq(&self, o: &Self) -> bool { self.entries == o.entries } }
impl<K: Eq, V: Eq> Eq for BTreeMap<K, V> {}
impl<K: PartialOrd, V: PartialOrd> PartialOrd for BTreeMap<K, V> { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { self.entries.as_slice().partial_cmp(o.entries.as_slice()) } }
impl<K: Ord, V: Ord> Ord for BTreeMap<K, V> { fn cmp(&self, o: &Self) -> Ordering { self.entries.as_slice().cmp(o.entries.as_slice()) } }
impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for BTreeMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_map().entries(self.iter()).finish() }
}
impl<K, V> FromIterator<(K, V)> for BTreeMap<K, V> where K: Ord {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self { let mut m = Self::new(); m.extend(iter); m }
}
impl<K: Ord, V> Extend<(K, V)> for BTreeMap<K, V> { fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, it: I) { for (k, v) in it { self.insert(k, v); } } }
impl<'a, K, V> IntoIterator for &'a BTreeMap<K, V> { type Item = (&'a K, &'a V); type IntoIter = core::iter::Map<core::slice::Iter<'a, (K,V)>, fn(&(K,V))->(&K,&V)>; fn into_iter(self) -> Self::IntoIter { fn pair<K,V>((k,v): &(K,V))->(&K,&V) {(k,v)} self.entries.iter().map(pair::<K,V>) } }
impl<'a, K, V> IntoIterator for &'a mut BTreeMap<K, V> { type Item = (&'a K, &'a mut V); type IntoIter = core::iter::Map<core::slice::IterMut<'a, (K,V)>, fn(&mut (K,V))->(&K,&mut V)>; fn into_iter(self) -> Self::IntoIter { fn pair<K,V>((k,v): &mut (K,V))->(&K,&mut V) {(&*k,v)} self.entries.iter_mut().map(pair::<K,V>) } }
impl<K: Ord + Borrow<Q>, V, Q: ?Sized + Ord> Index<&Q> for BTreeMap<K, V> { type Output = V; fn index(&self, key: &Q) -> &V { self.get(key).expect("key not found") } }
impl<K, V> IntoIterator for BTreeMap<K, V> { type Item = (K, V); type IntoIter = <Vec<(K,V)> as IntoIterator>::IntoIter; fn into_iter(self) -> Self::IntoIter { self.entries.into_iter() } }

pub enum Entry<'a, K: 'a, V: 'a> { Occupied(OccupiedEntry<'a, K, V>), Vacant(VacantEntry<'a, K, V>) }
pub struct OccupiedEntry<'a, K: 'a, V: 'a> { map: &'a mut BTreeMap<K,V>, index: usize }
pub struct VacantEntry<'a, K: 'a, V: 'a> { map: &'a mut BTreeMap<K,V>, key: K, index: usize }
impl<'a, K: Ord, V> Entry<'a, K, V> {
    pub fn key(&self) -> &K { match self { Self::Occupied(e) => e.key(), Self::Vacant(e) => e.key() } }
    pub fn or_insert(self, value: V) -> &'a mut V { self.or_insert_with(|| value) }
    pub fn or_insert_with<F: FnOnce() -> V>(self, f: F) -> &'a mut V { match self { Self::Occupied(e) => e.into_mut(), Self::Vacant(e) => e.insert(f()) } }
    pub fn or_default(self) -> &'a mut V where V: Default { self.or_insert_with(V::default) }
    pub fn and_modify<F: FnOnce(&mut V)>(self, f: F) -> Self { match self { Self::Occupied(mut e) => { f(e.get_mut()); Self::Occupied(e) }, vacant => vacant } }
}
impl<'a, K: Ord, V> OccupiedEntry<'a, K, V> {
    pub fn key(&self) -> &K { &self.map.entries[self.index].0 }
    pub fn get(&self) -> &V { &self.map.entries[self.index].1 }
    pub fn get_mut(&mut self) -> &mut V { &mut self.map.entries[self.index].1 }
    pub fn into_mut(self) -> &'a mut V { &mut self.map.entries[self.index].1 }
    pub fn insert(&mut self, value: V) -> V { core::mem::replace(self.get_mut(), value) }
    pub fn remove(self) -> V { self.map.remove_at(self.index).1 }
}
impl<'a, K: Ord, V> VacantEntry<'a, K, V> {
    pub fn key(&self) -> &K { &self.key }
    pub fn insert(self, value: V) -> &'a mut V { self.map.insert_at(self.index, (self.key, value)); &mut self.map.entries[self.index].1 }
}
