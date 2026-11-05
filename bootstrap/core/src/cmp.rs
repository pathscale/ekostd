use crate::clone::Clone;
use crate::marker::{Copy, Destruct, PointeeSized, Sized};
use crate::option::Option::{self, None, Some};

#[lang = "eq"]
#[rustc_diagnostic_item = "PartialEq"]
pub const trait PartialEq<Rhs: PointeeSized = Self>: PointeeSized {
    fn eq(&self, other: &Rhs) -> bool;
    fn ne(&self, other: &Rhs) -> bool { !self.eq(other) }
}

#[rustc_diagnostic_item = "Eq"]
pub const trait Eq: [const] PartialEq<Self> + PointeeSized {
    fn assert_fields_are_eq(&self) {}
}

pub struct AssertParamIsEq<T: Eq + PointeeSized> {
    _field: crate::marker::PhantomData<T>,
}

#[lang = "Ordering"]
#[repr(i8)]
pub enum Ordering {
    Less = 0xff_u8 as i8,
    Equal = 0,
    Greater = 1,
}
use self::Ordering::{Equal, Greater, Less};

impl Copy for Ordering {}
const impl Clone for Ordering {
    fn clone(&self) -> Self { *self }
}

#[lang = "partial_ord"]
#[rustc_diagnostic_item = "PartialOrd"]
pub const trait PartialOrd<Rhs: PointeeSized = Self>:
    [const] PartialEq<Rhs> + PointeeSized
{
    fn partial_cmp(&self, other: &Rhs) -> Option<Ordering>;

    fn lt(&self, other: &Rhs) -> bool {
        match self.partial_cmp(other) { Some(Less) => true, _ => false }
    }
    fn le(&self, other: &Rhs) -> bool {
        match self.partial_cmp(other) { Some(Less | Equal) => true, _ => false }
    }
    fn gt(&self, other: &Rhs) -> bool {
        match self.partial_cmp(other) { Some(Greater) => true, _ => false }
    }
    fn ge(&self, other: &Rhs) -> bool {
        match self.partial_cmp(other) { Some(Greater | Equal) => true, _ => false }
    }
}

#[rustc_diagnostic_item = "Ord"]
pub const trait Ord: [const] Eq + [const] PartialOrd<Self> + PointeeSized {
    fn cmp(&self, other: &Self) -> Ordering;

    fn min(self, other: Self) -> Self
    where
        Self: Sized + [const] Destruct,
    {
        if other < self { other } else { self }
    }

    fn max(self, other: Self) -> Self
    where
        Self: Sized + [const] Destruct,
    {
        if other < self { self } else { other }
    }
}

impl PartialEq for () {
    #[inline(always)]
    fn eq(&self, _other: &()) -> bool { true }
}
impl Eq for () {}
impl PartialOrd for () {
    #[inline(always)]
    fn partial_cmp(&self, _other: &()) -> Option<Ordering> { Some(Equal) }
}
impl Ord for () {
    #[inline(always)]
    fn cmp(&self, _other: &()) -> Ordering { Equal }
}

impl<A: PointeeSized, B: PointeeSized> PartialEq<&B> for &A
where
    A: PartialEq<B>,
{
    #[inline(always)]
    fn eq(&self, other: &&B) -> bool { PartialEq::eq(*self, *other) }
    #[inline(always)]
    fn ne(&self, other: &&B) -> bool { PartialEq::ne(*self, *other) }
}
impl<A: PointeeSized + Eq> Eq for &A {}
impl<A: PointeeSized, B: PointeeSized> PartialOrd<&B> for &A
where
    A: PartialOrd<B>,
{
    #[inline(always)]
    fn partial_cmp(&self, other: &&B) -> Option<Ordering> { PartialOrd::partial_cmp(*self, *other) }
}
impl<A: PointeeSized + Ord> Ord for &A {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering { Ord::cmp(*self, *other) }
}

impl<A: PointeeSized, B: PointeeSized> PartialEq<&mut B> for &mut A
where
    A: PartialEq<B>,
{
    #[inline(always)]
    fn eq(&self, other: &&mut B) -> bool { PartialEq::eq(*self, *other) }
    #[inline(always)]
    fn ne(&self, other: &&mut B) -> bool { PartialEq::ne(*self, *other) }
}
impl<A: PointeeSized + Eq> Eq for &mut A {}
impl<A: PointeeSized, B: PointeeSized> PartialOrd<&mut B> for &mut A
where
    A: PartialOrd<B>,
{
    #[inline(always)]
    fn partial_cmp(&self, other: &&mut B) -> Option<Ordering> { PartialOrd::partial_cmp(*self, *other) }
}
impl<A: PointeeSized + Ord> Ord for &mut A {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering { Ord::cmp(*self, *other) }
}

macro_rules! scalar_partial_cmp {
    ($($ty:ty),* $(,)?) => { $(
        const impl PartialEq for $ty {
            #[inline]
            fn eq(&self, other: &Self) -> bool { *self == *other }
            #[inline]
            fn ne(&self, other: &Self) -> bool { *self != *other }
        }
        const impl PartialOrd for $ty {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                if *self < *other { Some(Less) }
                else if *self > *other { Some(Greater) }
                else if *self == *other { Some(Equal) }
                else { None }
            }
            #[inline]
            fn lt(&self, other: &Self) -> bool { *self < *other }
            #[inline]
            fn le(&self, other: &Self) -> bool { *self <= *other }
            #[inline]
            fn gt(&self, other: &Self) -> bool { *self > *other }
            #[inline]
            fn ge(&self, other: &Self) -> bool { *self >= *other }
        }
    )* };
}

scalar_partial_cmp!(bool, char, u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize, f32, f64);

macro_rules! scalar_total_cmp {
    ($($ty:ty),* $(,)?) => { $(
        const impl Eq for $ty {}
        const impl Ord for $ty {
            fn cmp(&self, other: &Self) -> Ordering {
                if *self < *other { Less }
                else if *self > *other { Greater }
                else { Equal }
            }
        }
    )* };
}

scalar_total_cmp!(bool, char, u8, u16, u32, u64, u128, usize,
    i8, i16, i32, i64, i128, isize);

const impl PartialEq for Ordering {
    fn eq(&self, other: &Self) -> bool { (*self as i8) == (*other as i8) }
}
const impl Eq for Ordering {}
const impl PartialOrd for Ordering {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
const impl Ord for Ordering {
    fn cmp(&self, other: &Self) -> Ordering { (*self as i8).cmp(&(*other as i8)) }
}

impl<T: PartialEq> PartialEq for Option<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(left), Some(right)) => left.eq(right),
            (None, None) => true,
            _ => false,
        }
    }
}
impl<T: Eq> Eq for Option<T> {}

pub fn min<T: Ord>(v1: T, v2: T) -> T {
    v1.min(v2)
}

pub fn max<T: Ord>(v1: T, v2: T) -> T {
    v1.max(v2)
}

impl<A: PartialEq, B: PartialEq> PartialEq for (A, B) {
    #[inline(always)]
    fn eq(&self, other: &(A, B)) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}
impl<A: Eq, B: Eq> Eq for (A, B) {}
impl<A: PartialOrd, B: PartialOrd> PartialOrd for (A, B) {
    #[inline(always)]
    fn partial_cmp(&self, other: &(A, B)) -> Option<Ordering> {
        match self.0.partial_cmp(&other.0) {
            Some(Equal) => self.1.partial_cmp(&other.1),
            non_eq => non_eq,
        }
    }
}
impl<A: Ord, B: Ord> Ord for (A, B) {
    #[inline(always)]
    fn cmp(&self, other: &(A, B)) -> Ordering {
        match self.0.cmp(&other.0) {
            Equal => self.1.cmp(&other.1),
            non_eq => non_eq,
        }
    }
}

impl<T: PartialEq<U>, U, const N: usize> PartialEq<[U; N]> for [T; N] {
    #[inline(always)]
    fn eq(&self, other: &[U; N]) -> bool {
        let mut i = 0;
        while i < N {
            if !self[i].eq(&other[i]) {
                return false;
            }
            i += 1;
        }
        true
    }
}
impl<T: Eq, const N: usize> Eq for [T; N] {}
impl<T: PartialOrd, const N: usize> PartialOrd for [T; N] {
    #[inline(always)]
    fn partial_cmp(&self, other: &[T; N]) -> Option<Ordering> {
        self[..].partial_cmp(&other[..])
    }
}
impl<T: Ord, const N: usize> Ord for [T; N] {
    #[inline(always)]
    fn cmp(&self, other: &[T; N]) -> Ordering {
        self[..].cmp(&other[..])
    }
}
