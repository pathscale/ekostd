use crate::vec::Vec;
use core::fmt;
use core::iter::FromIterator;

pub struct VecDeque<T> { items: Vec<T> }
impl<T> VecDeque<T> {
    pub const fn new() -> Self { Self { items: Vec::new() } }
    pub fn with_capacity(capacity: usize) -> Self { Self { items: Vec::with_capacity(capacity) } }
    pub fn len(&self) -> usize { self.items.len() }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
    pub fn clear(&mut self) { self.items.clear(); }
    pub fn push_back(&mut self, value: T) { self.items.push(value); }
    pub fn push_front(&mut self, value: T) {
        let mut items = Vec::with_capacity(self.len() + 1);
        items.push(value);
        items.extend(core::mem::take(&mut self.items));
        self.items = items;
    }
    pub fn pop_back(&mut self) -> Option<T> { self.items.pop() }
    pub fn pop_front(&mut self) -> Option<T> {
        if self.items.is_empty() { return None; }
        let mut old = core::mem::take(&mut self.items).into_iter();
        let first = old.next();
        self.items.extend(old);
        first
    }
    pub fn front(&self) -> Option<&T> { self.items.first() }
    pub fn front_mut(&mut self) -> Option<&mut T> { self.items.first_mut() }
    pub fn back(&self) -> Option<&T> { self.items.last() }
    pub fn back_mut(&mut self) -> Option<&mut T> { self.items.last_mut() }
    pub fn get(&self, index: usize) -> Option<&T> { self.items.get(index) }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> { self.items.get_mut(index) }
    pub fn iter(&self) -> core::slice::Iter<'_, T> { self.items.iter() }
    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, T> { self.items.iter_mut() }
}
impl<T> Default for VecDeque<T> { fn default() -> Self { Self::new() } }
impl<T: Clone> Clone for VecDeque<T> { fn clone(&self) -> Self { Self { items: self.items.clone() } } }
impl<T: PartialEq> PartialEq for VecDeque<T> { fn eq(&self, o:&Self)->bool { self.items == o.items } }
impl<T: Eq> Eq for VecDeque<T> {}
impl<T: fmt::Debug> fmt::Debug for VecDeque<T> { fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result { f.debug_list().entries(self.iter()).finish() } }
impl<T> IntoIterator for VecDeque<T> { type Item=T; type IntoIter=<Vec<T> as IntoIterator>::IntoIter; fn into_iter(self)->Self::IntoIter { self.items.into_iter() } }
impl<'a, T> IntoIterator for &'a VecDeque<T> { type Item=&'a T; type IntoIter=core::slice::Iter<'a,T>; fn into_iter(self)->Self::IntoIter { self.items.iter() } }
impl<'a, T> IntoIterator for &'a mut VecDeque<T> { type Item=&'a mut T; type IntoIter=core::slice::IterMut<'a,T>; fn into_iter(self)->Self::IntoIter { self.items.iter_mut() } }
impl<T> FromIterator<T> for VecDeque<T> { fn from_iter<I:IntoIterator<Item=T>>(it:I)->Self { Self { items: it.into_iter().collect() } } }
impl<T> Extend<T> for VecDeque<T> { fn extend<I:IntoIterator<Item=T>>(&mut self,it:I) { self.items.extend(it); } }
