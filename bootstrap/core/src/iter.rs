use crate::clone::Clone;
use crate::marker::{Copy, PhantomData, Sized};
use crate::ops::{ControlFlow, FnMut, FromResidual, Range, RangeInclusive, Try};
use crate::result::Result;
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
    fn rposition<P: FnMut(Self::Item) -> bool>(&mut self, mut predicate: P) -> Option<usize>
    where Self: Sized + ExactSizeIterator + DoubleEndedIterator {
        let mut i = self.len();
        while let Some(x) = self.next_back() {
            i -= 1;
            if predicate(x) { return Some(i); }
        }
        None
    }
    fn map<B, F: FnMut(Self::Item) -> B>(self, f: F) -> Map<Self, F> where Self: Sized { Map { iter: self, f } }
    fn filter<P: FnMut(&Self::Item) -> bool>(self, predicate: P) -> Filter<Self, P> where Self: Sized { Filter { iter: self, predicate } }
    fn filter_map<B, F: FnMut(Self::Item) -> Option<B>>(self, f: F) -> FilterMap<Self, F> where Self: Sized { FilterMap { iter: self, f } }
    fn enumerate(self) -> Enumerate<Self> where Self: Sized { Enumerate { iter: self, count: 0 } }
    fn zip<U: IntoIterator>(self, other: U) -> Zip<Self, U::IntoIter> where Self: Sized { Zip { a: self, b: other.into_iter() } }
    fn chain<U: IntoIterator<Item = Self::Item>>(self, other: U) -> Chain<Self, U::IntoIter> where Self: Sized { Chain { a: Some(self), b: Some(other.into_iter()) } }
    fn take(self, n: usize) -> Take<Self> where Self: Sized { Take { iter: self, n } }
    fn skip(self, n: usize) -> Skip<Self> where Self: Sized { Skip { iter: self, n } }
    /// Adds up every item.
    fn sum<S: Sum<Self::Item>>(self) -> S where Self: Sized { S::sum(self) }
    /// Multiplies every item.
    fn product<P: Product<Self::Item>>(self) -> P where Self: Sized { P::product(self) }
    /// Every `step`th item, starting with the first. `step` must not be zero.
    fn step_by(self, step: usize) -> StepBy<Self> where Self: Sized {
        if step == 0 { crate::panic::panic_nounwind("step_by: step must be non-zero"); }
        StepBy { iter: self, step, first: true }
    }
    fn copied<'a, T: 'a + Copy>(self) -> Copied<Self> where Self: Sized + Iterator<Item = &'a T> { Copied { iter: self } }
    fn cloned<'a, T: 'a + Clone>(self) -> Cloned<Self> where Self: Sized + Iterator<Item = &'a T> { Cloned { iter: self } }
    fn rev(self) -> Rev<Self> where Self: Sized + DoubleEndedIterator { Rev { iter: self } }
    fn collect<B: FromIterator<Self::Item>>(self) -> B where Self: Sized { B::from_iter(self) }
    fn by_ref(&mut self) -> &mut Self where Self: Sized { self }
    /// The smallest item; the first of several equal ones.
    fn min(self) -> Option<Self::Item> where Self: Sized, Self::Item: crate::cmp::Ord {
        self.fold(None, |best, x| match best { Some(b) if !(x < b) => Some(b), _ => Some(x) })
    }
    /// The largest item; the last of several equal ones.
    fn max(self) -> Option<Self::Item> where Self: Sized, Self::Item: crate::cmp::Ord {
        self.fold(None, |best, x| match best { Some(b) if x < b => Some(b), _ => Some(x) })
    }
    /// The item whose key is largest; the last of several equal ones.
    fn max_by_key<K: crate::cmp::Ord, F: FnMut(&Self::Item) -> K>(mut self, mut f: F) -> Option<Self::Item> where Self: Sized {
        let mut best: Option<(K, Self::Item)> = None;
        while let Some(x) = self.next() {
            let key = f(&x);
            best = match best { Some((k, b)) if key < k => Some((k, b)), _ => Some((key, x)) };
        }
        match best { Some((_, x)) => Some(x), None => None }
    }
    /// The item whose key is smallest; the first of several equal ones.
    fn ne<I>(self, other: I) -> bool where I: IntoIterator, Self::Item: PartialEq<I::Item>, Self: Sized { !self.eq(other) }
    fn reduce<F: FnMut(Self::Item, Self::Item) -> Self::Item>(mut self, mut f: F) -> Option<Self::Item> where Self: Sized {
        let mut accum = self.next()?;
        while let Some(item) = self.next() { accum = f(accum, item); }
        Some(accum)
    }
    fn partition<B, F>(mut self, mut f: F) -> (B, B)
    where Self: Sized, B: Default + Extend<Self::Item>, F: FnMut(&Self::Item) -> bool {
        let mut left = B::default();
        let mut right = B::default();
        while let Some(item) = self.next() {
            if f(&item) { left.extend(Some(item)); } else { right.extend(Some(item)); }
        }
        (left, right)
    }
    fn max_by<F: FnMut(&Self::Item, &Self::Item) -> crate::cmp::Ordering>(mut self, mut compare: F) -> Option<Self::Item> where Self: Sized {
        let mut best = self.next()?;
        while let Some(item) = self.next() {
            // The last of several equal maxima wins.
            if compare(&best, &item) != crate::cmp::Ordering::Greater { best = item; }
        }
        Some(best)
    }
    fn min_by<F: FnMut(&Self::Item, &Self::Item) -> crate::cmp::Ordering>(mut self, mut compare: F) -> Option<Self::Item> where Self: Sized {
        let mut best = self.next()?;
        while let Some(item) = self.next() {
            if compare(&best, &item) == crate::cmp::Ordering::Greater { best = item; }
        }
        Some(best)
    }
    fn unzip<A, B, FromA, FromB>(self) -> (FromA, FromB)
    where FromA: Default + Extend<A>, FromB: Default + Extend<B>, Self: Sized + Iterator<Item = (A, B)> {
        let mut a = FromA::default();
        let mut b = FromB::default();
        for (x, y) in self {
            a.extend(Some(x));
            b.extend(Some(y));
        }
        (a, b)
    }
    fn min_by_key<K: crate::cmp::Ord, F: FnMut(&Self::Item) -> K>(mut self, mut f: F) -> Option<Self::Item> where Self: Sized {
        let mut best: Option<(K, Self::Item)> = None;
        while let Some(x) = self.next() {
            let key = f(&x);
            best = match best { Some((k, b)) if !(key < k) => Some((k, b)), _ => Some((key, x)) };
        }
        match best { Some((_, x)) => Some(x), None => None }
    }
    /// Folds until `f` first reports failure, and returns that failure.
    fn try_fold<B, F, R>(&mut self, init: B, mut f: F) -> R
    where Self: Sized, F: FnMut(B, Self::Item) -> R, R: Try<Output = B> {
        let mut acc = init;
        while let Some(x) = self.next() {
            match f(acc, x).branch() {
                ControlFlow::Continue(value) => acc = value,
                ControlFlow::Break(residual) => return R::from_residual(residual),
            }
        }
        R::from_output(acc)
    }
    /// Calls `f` on each item until one reports failure, and returns that failure.
    fn try_for_each<F, R>(&mut self, mut f: F) -> R
    where Self: Sized, F: FnMut(Self::Item) -> R, R: Try<Output = ()> {
        while let Some(x) = self.next() {
            match f(x).branch() {
                ControlFlow::Continue(()) => {}
                ControlFlow::Break(residual) => return R::from_residual(residual),
            }
        }
        R::from_output(())
    }
    fn flat_map<U: IntoIterator, F: FnMut(Self::Item) -> U>(self, f: F) -> FlatMap<Self, U, F> where Self: Sized {
        FlatMap { iter: self, f, front: None }
    }
    fn flatten(self) -> Flatten<Self> where Self: Sized, Self::Item: IntoIterator {
        Flatten { iter: self, front: None, back: None }
    }
    fn fuse(self) -> Fuse<Self> where Self: Sized { Fuse { iter: Some(self) } }
    fn peekable(self) -> Peekable<Self> where Self: Sized { Peekable { iter: self, peeked: None } }
    fn skip_while<P: FnMut(&Self::Item) -> bool>(self, predicate: P) -> SkipWhile<Self, P> where Self: Sized {
        SkipWhile { iter: self, flag: false, predicate }
    }
    fn map_while<B, P: FnMut(Self::Item) -> Option<B>>(self, predicate: P) -> MapWhile<Self, P> where Self: Sized {
        MapWhile { iter: self, predicate }
    }
    fn take_while<P: FnMut(&Self::Item) -> bool>(self, predicate: P) -> TakeWhile<Self, P> where Self: Sized {
        TakeWhile { iter: self, predicate, done: false }
    }
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
    #[inline]
    fn nth_back(&mut self, mut n: usize) -> Option<Self::Item> {
        while n > 0 {
            self.next_back()?;
            n -= 1;
        }
        self.next_back()
    }
    fn rfind<P: FnMut(&Self::Item) -> bool>(&mut self, mut predicate: P) -> Option<Self::Item> where Self: Sized {
        while let Some(x) = self.next_back() { if predicate(&x) { return Some(x); } } None
    }
    fn rfold<B, F>(mut self, init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let mut accum = init;
        while let Some(x) = self.next_back() {
            accum = f(accum, x);
        }
        accum
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

pub struct Enumerate<I> { pub(crate) iter: I, pub(crate) count: usize }
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
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a_lower, a_upper) = self.a.size_hint();
        let (b_lower, b_upper) = self.b.size_hint();
        let lower = if a_lower < b_lower { a_lower } else { b_lower };
        let upper = match (a_upper, b_upper) {
            (Some(x), Some(y)) => Some(if x < y { x } else { y }),
            (Some(x), None) => Some(x),
            (None, Some(y)) => Some(y),
            (None, None) => None,
        };
        (lower, upper)
    }
}
impl<A: ExactSizeIterator, B: ExactSizeIterator> ExactSizeIterator for Zip<A, B> {}
impl<A: DoubleEndedIterator + ExactSizeIterator, B: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for Zip<A, B> {
    fn next_back(&mut self) -> Option<(A::Item, B::Item)> {
        let a_len = self.a.len();
        let b_len = self.b.len();
        // Drop the unmatched tail of the longer side first.
        if a_len > b_len { for _ in 0..a_len - b_len { self.a.next_back(); } }
        if b_len > a_len { for _ in 0..b_len - a_len { self.b.next_back(); } }
        match (self.a.next_back(), self.b.next_back()) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        }
    }
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
impl<I: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for Take<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        if self.n == 0 { return None; }
        // Drop what lies beyond the first `n` items, then the next from the back is ours.
        let mut extra = self.iter.len().saturating_sub(self.n);
        while extra > 0 { self.iter.next_back(); extra -= 1; }
        self.n -= 1;
        self.iter.next_back()
    }
}

pub struct Skip<I> { iter: I, n: usize }
impl<I: Clone> Clone for Skip<I> { fn clone(&self) -> Self { Self { iter: self.iter.clone(), n: self.n } } }
impl<I: Iterator> Iterator for Skip<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        if self.n > 0 { let n = self.n; self.n = 0; self.iter.nth(n) } else { self.iter.next() }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lo, hi) = self.iter.size_hint();
        (lo.saturating_sub(self.n), hi.map(|h| h.saturating_sub(self.n)))
    }
    fn nth(&mut self, k: usize) -> Option<I::Item> {
        // Drop the skipped prefix first, then seek `k` items further.
        if self.n > 0 { let n = self.n; self.n = 0; self.iter.nth(n - 1)?; }
        self.iter.nth(k)
    }
}
impl<I: ExactSizeIterator> ExactSizeIterator for Skip<I> {}
impl<I: DoubleEndedIterator + ExactSizeIterator> DoubleEndedIterator for Skip<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        // Items in the skipped prefix are never yielded from the back.
        if self.len() > 0 { self.iter.next_back() } else { None }
    }
}
impl<I: FusedIterator> FusedIterator for Skip<I> {}

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

pub struct RepeatN<T> {
    element: Option<T>,
    count: usize,
}
impl<T: Clone> Clone for RepeatN<T> {
    fn clone(&self) -> Self {
        Self { element: self.element.clone(), count: self.count }
    }
}
impl<T: crate::fmt::Debug> crate::fmt::Debug for RepeatN<T> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("RepeatN")
    }
}
impl<T: Clone> Iterator for RepeatN<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.count > 0 {
            self.count -= 1;
            if self.count == 0 {
                self.element.take()
            } else {
                self.element.clone()
            }
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}
impl<T: Clone> ExactSizeIterator for RepeatN<T> {
    fn len(&self) -> usize { self.count }
}
impl<T: Clone> DoubleEndedIterator for RepeatN<T> {
    fn next_back(&mut self) -> Option<T> { self.next() }
}
pub fn repeat_n<T: Clone>(element: T, count: usize) -> RepeatN<T> {
    let element = if count == 0 { None } else { Some(element) };
    RepeatN { element, count }
}

impl<T, E> IntoIterator for Result<T, E> {
    type Item = T;
    type IntoIter = Once<T>;
    fn into_iter(self) -> Once<T> { Once { inner: self.ok() } }
}

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
    /// Number of `forward_one` steps from `start` to `end`: `(n, Some(n))` when it fits in
    /// usize, `(usize::MAX, None)` when it does not, `(0, Some(0))` when `start >= end`.
    fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>);
}

macro_rules! impl_step {
    ($($ty:ty => $u:ty),* $(,)?) => { $(
        impl Step for $ty {
            #[inline(always)]
            fn forward_one(start: Self) -> Self { start + 1 }
            #[inline(always)]
            fn backward_one(end: Self) -> Self { end - 1 }
            /// The distance is taken in the unsigned type of the same width, where it cannot overflow.
            #[inline(always)]
            fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
                if *start < *end {
                    let steps = (*end as $u).wrapping_sub(*start as $u);
                    if steps <= usize::MAX as $u {
                        (steps as usize, Some(steps as usize))
                    } else {
                        (usize::MAX, None)
                    }
                } else {
                    (0, Some(0))
                }
            }
        }
    )* };
}

impl_step!(
    u8 => u8, u16 => u16, u32 => u32, u64 => u64, u128 => u128, usize => usize,
    i8 => u8, i16 => u16, i32 => u32, i64 => u64, i128 => u128, isize => usize,
);

impl<A: Step> Iterator for crate::ops::RangeFrom<A> {
    type Item = A;
    fn next(&mut self) -> Option<A> {
        let v = self.start;
        self.start = Step::forward_one(self.start);
        Some(v)
    }
    fn size_hint(&self) -> (usize, Option<usize>) { (usize::MAX, None) }
}

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
        Step::steps_between(&self.start, &self.end)
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
    /// One more than the steps from `start` to `end`; nothing once exhausted or when `start > end`.
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.exhausted || self.start > self.end {
            return (0, Some(0));
        }
        let (lower, upper) = Step::steps_between(&self.start, &self.end);
        let upper = match upper {
            Some(n) => n.checked_add(1),
            None => None,
        };
        (lower.saturating_add(1), upper)
    }
}
impl<A: Step> DoubleEndedIterator for RangeInclusive<A> {
    #[inline]
    fn next_back(&mut self) -> Option<A> {
        if self.exhausted || self.start > self.end {
            self.exhausted = true;
            None
        } else {
            let v = self.end;
            if self.start == self.end {
                self.exhausted = true;
            } else {
                self.end = Step::backward_one(self.end);
            }
            Some(v)
        }
    }
}
impl<A: Step> FusedIterator for RangeInclusive<A> {}
/// Only the types whose full inclusive range still fits in usize on every target, as in Rust.
impl ExactSizeIterator for RangeInclusive<u8> {}
impl ExactSizeIterator for RangeInclusive<i8> {}
impl ExactSizeIterator for RangeInclusive<u16> {}
impl ExactSizeIterator for RangeInclusive<i16> {}

pub struct StepBy<I> { iter: I, step: usize, first: bool }
impl<I: Iterator> Iterator for StepBy<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        if self.first {
            self.first = false;
            return self.iter.next();
        }
        let mut skipped = 1;
        while skipped < self.step {
            match self.iter.next() { Some(_) => skipped += 1, None => return None }
        }
        self.iter.next()
    }
}

/// Types that can be made by adding up an iterator. `Iterator::sum` goes through this.
pub trait Sum<A = Self>: Sized {
    fn sum<I: Iterator<Item = A>>(iter: I) -> Self;
}
macro_rules! integer_sum {
    ($($t:ty)*) => {$(
        impl Sum for $t {
            fn sum<I: Iterator<Item = $t>>(mut iter: I) -> $t {
                let mut total: $t = 0;
                loop { match iter.next() { Some(item) => total += item, None => return total } }
            }
        }
        impl<'a> Sum<&'a $t> for $t {
            fn sum<I: Iterator<Item = &'a $t>>(mut iter: I) -> $t {
                let mut total: $t = 0;
                loop { match iter.next() { Some(item) => total += *item, None => return total } }
            }
        }
    )*};
}
integer_sum!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);
macro_rules! float_sum {
    ($($t:ty)*) => {$(
        impl Sum for $t {
            fn sum<I: Iterator<Item = $t>>(mut iter: I) -> $t {
                let mut total: $t = 0.0;
                loop { match iter.next() { Some(item) => total += item, None => return total } }
            }
        }
        impl<'a> Sum<&'a $t> for $t {
            fn sum<I: Iterator<Item = &'a $t>>(mut iter: I) -> $t {
                let mut total: $t = 0.0;
                loop { match iter.next() { Some(item) => total += *item, None => return total } }
            }
        }
    )*};
}
float_sum!(f32 f64);

/// Types that can be made by multiplying an iterator. `Iterator::product` goes through this.
pub trait Product<A = Self>: Sized {
    fn product<I: Iterator<Item = A>>(iter: I) -> Self;
}
macro_rules! integer_product {
    ($($t:ty)*) => {$(
        impl Product for $t {
            fn product<I: Iterator<Item = $t>>(mut iter: I) -> $t {
                let mut total: $t = 1;
                loop { match iter.next() { Some(item) => total *= item, None => return total } }
            }
        }
        impl<'a> Product<&'a $t> for $t {
            fn product<I: Iterator<Item = &'a $t>>(mut iter: I) -> $t {
                let mut total: $t = 1;
                loop { match iter.next() { Some(item) => total *= *item, None => return total } }
            }
        }
    )*};
}
integer_product!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);
macro_rules! float_product {
    ($($t:ty)*) => {$(
        impl Product for $t {
            fn product<I: Iterator<Item = $t>>(mut iter: I) -> $t {
                let mut total: $t = 1.0;
                loop { match iter.next() { Some(item) => total *= item, None => return total } }
            }
        }
        impl<'a> Product<&'a $t> for $t {
            fn product<I: Iterator<Item = &'a $t>>(mut iter: I) -> $t {
                let mut total: $t = 1.0;
                loop { match iter.next() { Some(item) => total *= *item, None => return total } }
            }
        }
    )*};
}
float_product!(f32 f64);

/// The items of every iterable `f` produces, one iterable after the other.
pub struct FlatMap<I, U: IntoIterator, F> { iter: I, f: F, front: Option<U::IntoIter> }
impl<I: Iterator, U: IntoIterator, F: FnMut(I::Item) -> U> Iterator for FlatMap<I, U, F> {
    type Item = U::Item;
    fn next(&mut self) -> Option<U::Item> {
        loop {
            if let Some(inner) = &mut self.front {
                if let Some(x) = inner.next() { return Some(x); }
                self.front = None;
            }
            match self.iter.next() {
                Some(outer) => self.front = Some((self.f)(outer).into_iter()),
                None => return None,
            }
        }
    }
}

/// The items of every iterable an iterator yields, one iterable after the other.
pub struct Flatten<I: Iterator> where I::Item: IntoIterator {
    iter: I,
    front: Option<<I::Item as IntoIterator>::IntoIter>,
    back: Option<<I::Item as IntoIterator>::IntoIter>,
}
impl<I: DoubleEndedIterator> DoubleEndedIterator for Flatten<I>
where I::Item: IntoIterator, <I::Item as IntoIterator>::IntoIter: DoubleEndedIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(inner) = &mut self.back {
                if let Some(x) = inner.next_back() { return Some(x); }
                self.back = None;
            }
            match self.iter.next_back() {
                Some(outer) => self.back = Some(outer.into_iter()),
                None => {
                    return match &mut self.front {
                        Some(inner) => {
                            let x = inner.next_back();
                            if x.is_none() { self.front = None; }
                            x
                        }
                        None => None,
                    };
                }
            }
        }
    }
}
impl<I: Iterator> Iterator for Flatten<I> where I::Item: IntoIterator {
    type Item = <I::Item as IntoIterator>::Item;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(inner) = &mut self.front {
                if let Some(x) = inner.next() { return Some(x); }
                self.front = None;
            }
            match self.iter.next() {
                Some(outer) => self.front = Some(outer.into_iter()),
                None => {
                    return match &mut self.back {
                        Some(inner) => {
                            let x = inner.next();
                            if x.is_none() { self.back = None; }
                            x
                        }
                        None => None,
                    };
                }
            }
        }
    }
}

/// An iterator whose next item can be looked at before it is taken.
pub struct Peekable<I: Iterator> { iter: I, peeked: Option<Option<I::Item>> }
impl<I: Iterator> Peekable<I> {
    pub fn peek(&mut self) -> Option<&I::Item> {
        if self.peeked.is_none() { self.peeked = Some(self.iter.next()); }
        match &self.peeked { Some(Some(x)) => Some(x), _ => None }
    }
    /// Takes the next item only when `accept` approves of it.
    pub fn next_if<F: crate::ops::FnOnce(&I::Item) -> bool>(&mut self, accept: F) -> Option<I::Item> {
        match self.next() {
            Some(x) if accept(&x) => Some(x),
            other => { self.peeked = Some(other); None }
        }
    }
}
impl<I: Iterator> Iterator for Peekable<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        match self.peeked.take() { Some(x) => x, None => self.iter.next() }
    }
}

/// The leading items a predicate accepts.
pub struct TakeWhile<I, P> { iter: I, predicate: P, done: bool }
impl<I: Iterator, P: FnMut(&I::Item) -> bool> Iterator for TakeWhile<I, P> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        if self.done { return None; }
        match self.iter.next() {
            Some(x) if (self.predicate)(&x) => Some(x),
            _ => { self.done = true; None }
        }
    }
}

/// Feeds the successes of a fallible iterator onward and remembers the first failure.
struct UntilFailure<'a, I, R> { iter: I, failure: &'a mut Option<R> }
impl<I: Iterator, R> Iterator for UntilFailure<'_, I, R> where I::Item: Try<Residual = R> {
    type Item = <I::Item as Try>::Output;
    fn next(&mut self) -> Option<Self::Item> {
        if self.failure.is_some() { return None; }
        match self.iter.next() {
            Some(x) => match x.branch() {
                ControlFlow::Continue(value) => Some(value),
                ControlFlow::Break(residual) => { *self.failure = Some(residual); None }
            },
            None => None,
        }
    }
}

/// Collects the successes, or gives back the first error.
impl<A, E, V: FromIterator<A>> FromIterator<Result<A, E>> for Result<V, E> {
    fn from_iter<I: IntoIterator<Item = Result<A, E>>>(iter: I) -> Self {
        let mut failure = None;
        let collected = V::from_iter(UntilFailure { iter: iter.into_iter(), failure: &mut failure });
        match failure {
            Some(Result::Err(error)) => Result::Err(error),
            _ => Result::Ok(collected),
        }
    }
}

/// Collects the values, or gives `None` when any item is `None`.
impl<A, V: FromIterator<A>> FromIterator<Option<A>> for Option<V> {
    fn from_iter<I: IntoIterator<Item = Option<A>>>(iter: I) -> Self {
        let mut failure = None;
        let collected = V::from_iter(UntilFailure { iter: iter.into_iter(), failure: &mut failure });
        match failure { Some(_) => None, None => Some(collected) }
    }
}

impl<I: Clone, U: IntoIterator, F: Clone> Clone for FlatMap<I, U, F> where U::IntoIter: Clone {
    fn clone(&self) -> Self { Self { iter: self.iter.clone(), f: self.f.clone(), front: self.front.clone() } }
}

/// An iterator that yields `None` forever once the underlying one has yielded `None` once.
pub struct Fuse<I> { iter: Option<I> }
impl<I: Clone> Clone for Fuse<I> { fn clone(&self) -> Self { Self { iter: self.iter.clone() } } }
impl<I: Iterator> Iterator for Fuse<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        match &mut self.iter {
            Some(iter) => {
                let item = iter.next();
                if item.is_none() { self.iter = None; }
                item
            }
            None => None,
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.iter { Some(iter) => iter.size_hint(), None => (0, Some(0)) }
    }
}
impl<I: DoubleEndedIterator> DoubleEndedIterator for Fuse<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        match &mut self.iter {
            Some(iter) => {
                let item = iter.next_back();
                if item.is_none() { self.iter = None; }
                item
            }
            None => None,
        }
    }
}
impl<I: ExactSizeIterator> ExactSizeIterator for Fuse<I> {}

/// An iterator that repeats one element forever.
pub struct Repeat<A> { element: A }
impl<A: Clone> Clone for Repeat<A> { fn clone(&self) -> Self { Self { element: self.element.clone() } } }
impl<A: Clone> Iterator for Repeat<A> {
    type Item = A;
    fn next(&mut self) -> Option<A> { Some(self.element.clone()) }
    fn size_hint(&self) -> (usize, Option<usize>) { (usize::MAX, None) }
}
impl<A: Clone> DoubleEndedIterator for Repeat<A> {
    fn next_back(&mut self) -> Option<A> { Some(self.element.clone()) }
}
pub fn repeat<T: Clone>(elt: T) -> Repeat<T> { Repeat { element: elt } }

pub struct SkipWhile<I, P> { iter: I, flag: bool, predicate: P }
impl<I: Clone, P: Clone> Clone for SkipWhile<I, P> {
    fn clone(&self) -> Self { Self { iter: self.iter.clone(), flag: self.flag, predicate: self.predicate.clone() } }
}
impl<I: Iterator, P: FnMut(&I::Item) -> bool> Iterator for SkipWhile<I, P> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        loop {
            let item = self.iter.next()?;
            if self.flag || !(self.predicate)(&item) {
                self.flag = true;
                return Some(item);
            }
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }
}

impl<I: crate::fmt::Debug> crate::fmt::Debug for Fuse<I> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.debug_struct("Fuse").field("iter", &self.iter).finish()
    }
}

/// Yields what `predicate` maps to `Some`, and ends at the first `None`.
pub struct MapWhile<I, P> { iter: I, predicate: P }
impl<B, I: Iterator, P: FnMut(I::Item) -> Option<B>> Iterator for MapWhile<I, P> {
    type Item = B;
    fn next(&mut self) -> Option<B> {
        let x = self.iter.next()?;
        (self.predicate)(x)
    }
    fn size_hint(&self) -> (usize, Option<usize>) { let (_, upper) = self.iter.size_hint(); (0, upper) }
}
