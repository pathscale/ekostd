use crate::clone::Clone;
use crate::marker::{Copy, PhantomData, Sized};
use crate::ops::{FnMut, Range, RangeInclusive};
use crate::option::Option::{self, None, Some};

#[lang = "iterator"]
pub trait Iterator {
    type Item;
    #[lang = "next"]
    fn next(&mut self) -> Option<Self::Item>;
    fn size_hint(&self) -> (usize, Option<usize>) { (0, None) }
    fn count(mut self) -> usize where Self: Sized {
        let mut n = 0; while let Some(_) = self.next() { n += 1; } n
    }
    fn last(mut self) -> Option<Self::Item> where Self: Sized {
        let mut last = None; while let Some(x) = self.next() { last = Some(x); } last
    }
    fn nth(&mut self, mut n: usize) -> Option<Self::Item> {
        while let Some(x) = self.next() { if n == 0 { return Some(x); } n -= 1; } None
    }
    fn fold<B, F: FnMut(B, Self::Item) -> B>(mut self, mut init: B, mut f: F) -> B where Self: Sized {
        while let Some(x) = self.next() { init = f(init, x); } init
    }
    fn for_each<F: FnMut(Self::Item)>(mut self, mut f: F) where Self: Sized {
        while let Some(x) = self.next() { f(x); }
    }
    fn all<F: FnMut(Self::Item) -> bool>(&mut self, mut f: F) -> bool where Self: Sized {
        while let Some(x) = self.next() { if !f(x) { return false; } } true
    }
    fn any<F: FnMut(Self::Item) -> bool>(&mut self, mut f: F) -> bool where Self: Sized {
        while let Some(x) = self.next() { if f(x) { return true; } } false
    }
    fn find<P: FnMut(&Self::Item) -> bool>(&mut self, mut predicate: P) -> Option<Self::Item> where Self: Sized {
        while let Some(x) = self.next() { if predicate(&x) { return Some(x); } } None
    }
    fn find_map<B, F: FnMut(Self::Item) -> Option<B>>(&mut self, mut f: F) -> Option<B> where Self: Sized {
        while let Some(x) = self.next() { if let Some(y) = f(x) { return Some(y); } } None
    }
    fn position<P: FnMut(Self::Item) -> bool>(&mut self, mut predicate: P) -> Option<usize> where Self: Sized {
        let mut i = 0; while let Some(x) = self.next() { if predicate(x) { return Some(i); } i += 1; } None
    }
    fn map<B, F: FnMut(Self::Item) -> B>(self, f: F) -> Map<Self, F> where Self: Sized { Map { iter: self, f } }
    fn filter<P: FnMut(&Self::Item) -> bool>(self, predicate: P) -> Filter<Self, P> where Self: Sized { Filter { iter: self, predicate } }
    fn filter_map<B, F: FnMut(Self::Item) -> Option<B>>(self, f: F) -> FilterMap<Self, F> where Self: Sized { FilterMap { iter: self, f } }
    fn enumerate(self) -> Enumerate<Self> where Self: Sized { Enumerate { iter: self, count: 0 } }
    fn zip<U: IntoIterator>(self, other: U) -> Zip<Self, U::IntoIter> where Self: Sized { Zip { a: self, b: other.into_iter() } }
    fn chain<U: IntoIterator<Item = Self::Item>>(self, other: U) -> Chain<Self, U::IntoIter> where Self: Sized { Chain { a: Some(self), b: Some(other.into_iter()) } }
    fn take(self, n: usize) -> Take<Self> where Self: Sized { Take { iter: self, n } }
    fn skip(self, n: usize) -> Skip<Self> where Self: Sized { Skip { iter: self, n } }
    fn copied<'a, T: 'a + Copy>(self) -> Copied<Self> where Self: Sized + Iterator<Item = &'a T> { Copied { iter: self } }
    fn cloned<'a, T: 'a + Clone>(self) -> Cloned<Self> where Self: Sized + Iterator<Item = &'a T> { Cloned { iter: self } }
    fn rev(self) -> Rev<Self> where Self: Sized + DoubleEndedIterator { Rev { iter: self } }
    fn collect<B: FromIterator<Self::Item>>(self) -> B where Self: Sized { B::from_iter(self) }
    fn by_ref(&mut self) -> &mut Self where Self: Sized { self }
    fn eq<I>(mut self, other: I) -> bool
    where
        I: IntoIterator,
        Self::Item: crate::cmp::PartialEq<I::Item>,
        Self: Sized,
    {
        let mut other = other.into_iter();
        loop {
            match (self.next(), other.next()) {
                (None, None) => return true,
                (None, Some(_)) | (Some(_), None) => return false,
                (Some(a), Some(b)) => if !a.eq(&b) { return false; },
            }
        }
    }
    fn partial_cmp<I>(mut self, other: I) -> Option<crate::cmp::Ordering>
    where
        I: IntoIterator,
        Self::Item: crate::cmp::PartialOrd<I::Item>,
        Self: Sized,
    {
        let mut other = other.into_iter();
        loop {
            match (self.next(), other.next()) {
                (None, None) => return Some(crate::cmp::Ordering::Equal),
                (None, Some(_)) => return Some(crate::cmp::Ordering::Less),
                (Some(_), None) => return Some(crate::cmp::Ordering::Greater),
                (Some(a), Some(b)) => match a.partial_cmp(&b) {
                    Some(crate::cmp::Ordering::Equal) => continue,
                    non_eq => return non_eq,
                },
            }
        }
    }
    fn cmp<I>(mut self, other: I) -> crate::cmp::Ordering
    where
        I: IntoIterator<Item = Self::Item>,
        Self::Item: crate::cmp::Ord,
        Self: Sized,
    {
        let mut other = other.into_iter();
        loop {
            match (self.next(), other.next()) {
                (None, None) => return crate::cmp::Ordering::Equal,
                (None, Some(_)) => return crate::cmp::Ordering::Less,
                (Some(_), None) => return crate::cmp::Ordering::Greater,
                (Some(a), Some(b)) => match a.cmp(&b) {
                    crate::cmp::Ordering::Equal => continue,
                    non_eq => return non_eq,
                },
            }
        }
    }
}

impl<I: Iterator + ?Sized> Iterator for &mut I {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> { (**self).next() }
    fn size_hint(&self) -> (usize, Option<usize>) { (**self).size_hint() }
    fn nth(&mut self, n: usize) -> Option<I::Item> { (**self).nth(n) }
}

pub trait IntoIterator {
    type Item;
    type IntoIter: Iterator<Item = Self::Item>;
    #[lang = "into_iter"]
    fn into_iter(self) -> Self::IntoIter;
}
impl<I: Iterator> IntoIterator for I {
    type Item = I::Item;
    type IntoIter = I;
    fn into_iter(self) -> I { self }
}

pub trait FromIterator<A>: Sized { fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self; }
pub trait Extend<A> { fn extend<T: IntoIterator<Item = A>>(&mut self, iter: T); }

pub trait DoubleEndedIterator: Iterator {
    fn next_back(&mut self) -> Option<Self::Item>;
    fn rfind<P: FnMut(&Self::Item) -> bool>(&mut self, mut predicate: P) -> Option<Self::Item> where Self: Sized {
        while let Some(x) = self.next_back() { if predicate(&x) { return Some(x); } } None
    }
}
impl<I: DoubleEndedIterator + ?Sized> DoubleEndedIterator for &mut I {
    fn next_back(&mut self) -> Option<I::Item> { (**self).next_back() }
}

pub trait ExactSizeIterator: Iterator {
    fn len(&self) -> usize { self.size_hint().0 }
    fn is_empty(&self) -> bool { self.len() == 0 }
}
impl<I: ExactSizeIterator + ?Sized> ExactSizeIterator for &mut I {
    fn len(&self) -> usize { (**self).len() }
}

pub trait FusedIterator: Iterator {}
impl<I: FusedIterator + ?Sized> FusedIterator for &mut I {}

pub struct Map<I, F> { iter: I, f: F }
impl<I: Clone, F: Clone> Clone for Map<I, F> { fn clone(&self) -> Self { Self { iter: self.iter.clone(), f: self.f.clone() } } }
impl<B, I: Iterator, F: FnMut(I::Item) -> B> Iterator for Map<I, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> { self.iter.next().map(&mut self.f) }
    fn size_hint(&self) -> (usize, Option<usize>) { self.iter.size_hint() }
}
impl<B, I: DoubleEndedIterator, F: FnMut(I::Item) -> B> DoubleEndedIterator for Map<I, F> {
    fn next_back(&mut self) -> Option<B> { self.iter.next_back().map(&mut self.f) }
}
impl<B, I: ExactSizeIterator, F: FnMut(I::Item) -> B> ExactSizeIterator for Map<I, F> {}

pub struct Filter<I, P> { iter: I, predicate: P }
impl<I: Clone, P: Clone> Clone for Filter<I, P> { fn clone(&self) -> Self { Self { iter: self.iter.clone(), predicate: self.predicate.clone() } } }
impl<I: Iterator, P: FnMut(&I::Item) -> bool> Iterator for Filter<I, P> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> { self.iter.find(&mut self.predicate) }
    fn size_hint(&self) -> (usize, Option<usize>) { (0, self.iter.size_hint().1) }
}
impl<I: DoubleEndedIterator, P: FnMut(&I::Item) -> bool> DoubleEndedIterator for Filter<I, P> {
    fn next_back(&mut self) -> Option<I::Item> { self.iter.rfind(&mut self.predicate) }
}

pub struct FilterMap<I, F> { iter: I, f: F }
impl<I: Clone, F: Clone> Clone for FilterMap<I, F> { fn clone(&self) -> Self { Self { iter: self.iter.clone(), f: self.f.clone() } } }
impl<B, I: Iterator, F: FnMut(I::Item) -> Option<B>> Iterator for FilterMap<I, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> { self.iter.find_map(&mut self.f) }
    fn size_hint(&self) -> (usize, Option<usize>) { (0, self.iter.size_hint().1) }
}
impl<B, I: DoubleEndedIterator, F: FnMut(I::Item) -> Option<B>> DoubleEndedIterator for FilterMap<I, F> {
    fn next_back(&mut self) -> Option<B> {
        while let Some(x) = self.iter.next_back() { if let Some(y) = (self.f)(x) { return Some(y); } } None
    }
}

pub struct Enumerate<I> { iter: I, count: usize }
impl<I: Clone> Clone for Enumerate<I> { fn clone(&self) -> Self { Self { iter: self.iter.clone(), count: self.count } } }
impl<I: Iterator> Iterator for Enumerate<I> {
    type Item = (usize, I::Item);
    fn next(&mut self) -> Option<(usize, I::Item)> {
        let a = self.iter.next()?; let i = self.count; self.count += 1; Some((i, a))
    }
    fn size_hint(&self) -> (usize, Option<usize>) { self.iter.size_hint() }
}
impl<I: ExactSizeIterator> ExactSizeIterator for Enumerate<I> {}
impl<I: ExactSizeIterator + DoubleEndedIterator> DoubleEndedIterator for Enumerate<I> {
    fn next_back(&mut self) -> Option<(usize, I::Item)> {
        let a = self.iter.next_back()?; Some((self.count + self.iter.len(), a))
    }
}

pub struct Zip<A, B> { a: A, b: B }
impl<A: Clone, B: Clone> Clone for Zip<A, B> { fn clone(&self) -> Self { Self { a: self.a.clone(), b: self.b.clone() } } }
impl<A: Iterator, B: Iterator> Iterator for Zip<A, B> {
    type Item = (A::Item, B::Item);
    fn next(&mut self) -> Option<(A::Item, B::Item)> { Some((self.a.next()?, self.b.next()?)) }
}

pub struct Chain<A, B> { a: Option<A>, b: Option<B> }
impl<A: Clone, B: Clone> Clone for Chain<A, B> { fn clone(&self) -> Self { Self { a: self.a.clone(), b: self.b.clone() } } }
impl<A: Iterator, B: Iterator<Item = A::Item>> Iterator for Chain<A, B> {
    type Item = A::Item;
    fn next(&mut self) -> Option<A::Item> {
        if let Some(ref mut a) = self.a { if let Some(x) = a.next() { return Some(x); } self.a = None; }
        if let Some(ref mut b) = self.b { if let Some(x) = b.next() { return Some(x); } self.b = None; }
        None
    }
}
impl<A: DoubleEndedIterator, B: DoubleEndedIterator<Item = A::Item>> DoubleEndedIterator for Chain<A, B> {
    fn next_back(&mut self) -> Option<A::Item> {
        if let Some(ref mut b) = self.b { if let Some(x) = b.next_back() { return Some(x); } self.b = None; }
        if let Some(ref mut a) = self.a { if let Some(x) = a.next_back() { return Some(x); } self.a = None; }
        None
    }
}

pub struct Take<I> { iter: I, n: usize }
impl<I: Clone> Clone for Take<I> { fn clone(&self) -> Self { Self { iter: self.iter.clone(), n: self.n } } }
impl<I: Iterator> Iterator for Take<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> { if self.n != 0 { self.n -= 1; self.iter.next() } else { None } }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lo, hi) = self.iter.size_hint();
        let lo = if lo < self.n { lo } else { self.n };
        let hi = match hi { Some(x) if x < self.n => Some(x), _ => Some(self.n) };
        (lo, hi)
    }
}
impl<I: ExactSizeIterator> ExactSizeIterator for Take<I> {}

pub struct Skip<I> { iter: I, n: usize }
impl<I: Clone> Clone for Skip<I> { fn clone(&self) -> Self { Self { iter: self.iter.clone(), n: self.n } } }
impl<I: Iterator> Iterator for Skip<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        if self.n > 0 { let n = self.n; self.n = 0; self.iter.nth(n) } else { self.iter.next() }
    }
}

pub struct Copied<I> { iter: I }
impl<I: Clone> Clone for Copied<I> { fn clone(&self) -> Self { Self { iter: self.iter.clone() } } }
impl<'a, I: Iterator<Item = &'a T>, T: 'a + Copy> Iterator for Copied<I> {
    type Item = T;
    fn next(&mut self) -> Option<T> { self.iter.next().map(|&x| x) }
    fn size_hint(&self) -> (usize, Option<usize>) { self.iter.size_hint() }
}
impl<'a, I: DoubleEndedIterator<Item = &'a T>, T: 'a + Copy> DoubleEndedIterator for Copied<I> {
    fn next_back(&mut self) -> Option<T> { self.iter.next_back().map(|&x| x) }
}
impl<'a, I: ExactSizeIterator<Item = &'a T>, T: 'a + Copy> ExactSizeIterator for Copied<I> {}

pub struct Cloned<I> { iter: I }
impl<I: Clone> Clone for Cloned<I> { fn clone(&self) -> Self { Self { iter: self.iter.clone() } } }
impl<'a, I: Iterator<Item = &'a T>, T: 'a + Clone> Iterator for Cloned<I> {
    type Item = T;
    fn next(&mut self) -> Option<T> { self.iter.next().map(T::clone) }
    fn size_hint(&self) -> (usize, Option<usize>) { self.iter.size_hint() }
}
impl<'a, I: DoubleEndedIterator<Item = &'a T>, T: 'a + Clone> DoubleEndedIterator for Cloned<I> {
    fn next_back(&mut self) -> Option<T> { self.iter.next_back().map(T::clone) }
}
impl<'a, I: ExactSizeIterator<Item = &'a T>, T: 'a + Clone> ExactSizeIterator for Cloned<I> {}

pub struct Rev<I> { iter: I }
impl<I: Clone> Clone for Rev<I> { fn clone(&self) -> Self { Self { iter: self.iter.clone() } } }
impl<I: DoubleEndedIterator> Iterator for Rev<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> { self.iter.next_back() }
    fn size_hint(&self) -> (usize, Option<usize>) { self.iter.size_hint() }
}
impl<I: DoubleEndedIterator> DoubleEndedIterator for Rev<I> {
    fn next_back(&mut self) -> Option<I::Item> { self.iter.next() }
}
impl<I: ExactSizeIterator + DoubleEndedIterator> ExactSizeIterator for Rev<I> {}

pub struct Once<T> { inner: Option<T> }
impl<T: Clone> Clone for Once<T> { fn clone(&self) -> Self { Self { inner: self.inner.clone() } } }
impl<T> Iterator for Once<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> { let mut out = None; crate::mem::swap(&mut self.inner, &mut out); out }
    fn size_hint(&self) -> (usize, Option<usize>) { let n = if self.inner.is_some() { 1 } else { 0 }; (n, Some(n)) }
}
impl<T> DoubleEndedIterator for Once<T> { fn next_back(&mut self) -> Option<T> { self.next() } }
impl<T> ExactSizeIterator for Once<T> {}
impl<T> FusedIterator for Once<T> {}
pub fn once<T>(value: T) -> Once<T> { Once { inner: Some(value) } }

pub struct Empty<T>(PhantomData<fn() -> T>);
impl<T> Copy for Empty<T> {}
impl<T> Clone for Empty<T> { fn clone(&self) -> Self { *self } }
impl<T> Iterator for Empty<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> { None }
    fn size_hint(&self) -> (usize, Option<usize>) { (0, Some(0)) }
}
impl<T> DoubleEndedIterator for Empty<T> { fn next_back(&mut self) -> Option<T> { None } }
impl<T> ExactSizeIterator for Empty<T> {}
impl<T> FusedIterator for Empty<T> {}
pub const fn empty<T>() -> Empty<T> { Empty(PhantomData) }

pub struct Successors<T, F> { next: Option<T>, succ: F }
impl<T: Clone, F: Clone> Clone for Successors<T, F> { fn clone(&self) -> Self { Self { next: self.next.clone(), succ: self.succ.clone() } } }
impl<T, F: FnMut(&T) -> Option<T>> Iterator for Successors<T, F> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        let mut item = None; crate::mem::swap(&mut self.next, &mut item);
        let item = item?; self.next = (self.succ)(&item); Some(item)
    }
}
pub fn successors<T, F: FnMut(&T) -> Option<T>>(first: Option<T>, succ: F) -> Successors<T, F> { Successors { next: first, succ } }

pub struct FromFn<F> { f: F }
impl<F: Clone> Clone for FromFn<F> { fn clone(&self) -> Self { Self { f: self.f.clone() } } }
impl<T, F: FnMut() -> Option<T>> Iterator for FromFn<F> {
    type Item = T;
    fn next(&mut self) -> Option<T> { (self.f)() }
}
pub fn from_fn<T, F: FnMut() -> Option<T>>(f: F) -> FromFn<F> { FromFn { f } }

pub struct RepeatWith<F> { repeater: F }
impl<F: Clone> Clone for RepeatWith<F> { fn clone(&self) -> Self { Self { repeater: self.repeater.clone() } } }
impl<A, F: FnMut() -> A> Iterator for RepeatWith<F> {
    type Item = A;
    fn next(&mut self) -> Option<A> { Some((self.repeater)()) }
}
pub fn repeat_with<A, F: FnMut() -> A>(repeater: F) -> RepeatWith<F> { RepeatWith { repeater } }

impl<T> IntoIterator for Option<T> {
    type Item = T;
    type IntoIter = Once<T>;
    fn into_iter(self) -> Once<T> { Once { inner: self } }
}
impl<'a, T> IntoIterator for &'a Option<T> {
    type Item = &'a T;
    type IntoIter = Once<&'a T>;
    fn into_iter(self) -> Once<&'a T> { Once { inner: self.as_ref() } }
}
impl<'a, T> IntoIterator for &'a mut Option<T> {
    type Item = &'a mut T;
    type IntoIter = Once<&'a mut T>;
    fn into_iter(self) -> Once<&'a mut T> { Once { inner: self.as_mut() } }
}
impl<T> Option<T> {
    pub fn iter(&self) -> Once<&T> { Once { inner: self.as_ref() } }
    pub fn iter_mut(&mut self) -> Once<&mut T> { Once { inner: self.as_mut() } }
}

pub trait Step: Clone + Copy + crate::cmp::PartialOrd + Sized {
    fn forward_one(start: Self) -> Self;
    fn backward_one(end: Self) -> Self;
    fn steps_between(start: &Self, end: &Self) -> usize;
}

macro_rules! impl_step {
    ($($ty:ty),* $(,)?) => { $(
        impl Step for $ty {
            #[inline(always)]
            fn forward_one(start: Self) -> Self { start + 1 }
            #[inline(always)]
            fn backward_one(end: Self) -> Self { end - 1 }
            #[inline(always)]
            fn steps_between(start: &Self, end: &Self) -> usize {
                if *start < *end { (*end - *start) as usize } else { 0 }
            }
        }
    )* };
}

impl_step!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

impl<A: Step> Iterator for Range<A> {
    type Item = A;
    #[inline]
    fn next(&mut self) -> Option<A> {
        if self.start < self.end {
            let v = self.start;
            self.start = Step::forward_one(self.start);
            Some(v)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = Step::steps_between(&self.start, &self.end);
        (len, Some(len))
    }
}
impl<A: Step> DoubleEndedIterator for Range<A> {
    #[inline]
    fn next_back(&mut self) -> Option<A> {
        if self.start < self.end {
            self.end = Step::backward_one(self.end);
            Some(self.end)
        } else {
            None
        }
    }
}
impl<A: Step> ExactSizeIterator for Range<A> {}
impl<A: Step> FusedIterator for Range<A> {}

impl<A: Step> Iterator for RangeInclusive<A> {
    type Item = A;
    #[inline]
    fn next(&mut self) -> Option<A> {
        if self.exhausted || self.start > self.end {
            self.exhausted = true;
            None
        } else {
            let v = self.start;
            if self.start == self.end {
                self.exhausted = true;
            } else {
                self.start = Step::forward_one(self.start);
            }
            Some(v)
        }
    }
}
