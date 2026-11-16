#[derive(crate::clone::Clone, crate::marker::Copy, crate::cmp::PartialEq, crate::cmp::Eq, crate::fmt::Debug)]
#[lang = "RangeFull"]
pub struct RangeFull;

#[derive(crate::clone::Clone, crate::marker::Copy, crate::cmp::PartialEq, crate::cmp::Eq, crate::fmt::Debug)]
#[lang = "Range"]
pub struct Range<Idx> {
    pub start: Idx,
    pub end: Idx,
}

#[derive(crate::clone::Clone, crate::marker::Copy, crate::cmp::PartialEq, crate::cmp::Eq, crate::fmt::Debug)]
#[lang = "RangeFrom"]
pub struct RangeFrom<Idx> {
    pub start: Idx,
}

#[derive(crate::clone::Clone, crate::marker::Copy, crate::cmp::PartialEq, crate::cmp::Eq, crate::fmt::Debug)]
#[lang = "RangeTo"]
pub struct RangeTo<Idx> {
    pub end: Idx,
}

#[derive(crate::clone::Clone, crate::marker::Copy, crate::cmp::PartialEq, crate::cmp::Eq, crate::fmt::Debug)]
#[lang = "RangeInclusive"]
pub struct RangeInclusive<Idx> {
    pub(crate) start: Idx,
    pub(crate) end: Idx,
    pub(crate) exhausted: bool,
}

impl<Idx> RangeInclusive<Idx> {
    #[lang = "range_inclusive_new"]
    pub const fn new(start: Idx, end: Idx) -> Self {
        Self { start, end, exhausted: false }
    }

    pub const fn start(&self) -> &Idx {
        &self.start
    }

    pub const fn end(&self) -> &Idx {
        &self.end
    }
}

#[derive(crate::clone::Clone, crate::marker::Copy, crate::cmp::PartialEq, crate::cmp::Eq, crate::fmt::Debug)]
#[lang = "RangeToInclusive"]
pub struct RangeToInclusive<Idx> {
    pub end: Idx,
}

#[derive(crate::clone::Clone, crate::marker::Copy, crate::cmp::PartialEq, crate::cmp::Eq, crate::fmt::Debug)]
pub enum Bound<T> {
    Included(T),
    Excluded(T),
    Unbounded,
}

impl<T> Bound<T> {
    #[inline]
    pub const fn as_ref(&self) -> Bound<&T> {
        match *self {
            Bound::Included(ref x) => Bound::Included(x),
            Bound::Excluded(ref x) => Bound::Excluded(x),
            Bound::Unbounded => Bound::Unbounded,
        }
    }

    #[inline]
    pub const fn as_mut(&mut self) -> Bound<&mut T> {
        match *self {
            Bound::Included(ref mut x) => Bound::Included(x),
            Bound::Excluded(ref mut x) => Bound::Excluded(x),
            Bound::Unbounded => Bound::Unbounded,
        }
    }

    #[inline]
    pub fn map<U, F: crate::ops::FnOnce(T) -> U>(self, f: F) -> Bound<U> {
        match self {
            Bound::Included(x) => Bound::Included(f(x)),
            Bound::Excluded(x) => Bound::Excluded(f(x)),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
}

impl<T: crate::clone::Clone> Bound<&T> {
    #[inline]
    pub fn cloned(self) -> Bound<T> {
        match self {
            Bound::Included(x) => Bound::Included(x.clone()),
            Bound::Excluded(x) => Bound::Excluded(x.clone()),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
}

impl<T: crate::marker::Copy> Bound<&T> {
    #[inline]
    pub const fn copied(self) -> Bound<T> {
        match self {
            Bound::Included(&x) => Bound::Included(x),
            Bound::Excluded(&x) => Bound::Excluded(x),
            Bound::Unbounded => Bound::Unbounded,
        }
    }
}

pub trait RangeBounds<T: ?Sized> {
    fn start_bound(&self) -> Bound<&T>;
    fn end_bound(&self) -> Bound<&T>;

    fn contains<U>(&self, item: &U) -> bool
    where
        T: crate::cmp::PartialOrd<U>,
        U: ?Sized + crate::cmp::PartialOrd<T>,
    {
        (match self.start_bound() {
            Bound::Included(start) => start <= item,
            Bound::Excluded(start) => start < item,
            Bound::Unbounded => true,
        }) && (match self.end_bound() {
            Bound::Included(end) => item <= end,
            Bound::Excluded(end) => item < end,
            Bound::Unbounded => true,
        })
    }
}

impl<T: ?Sized> RangeBounds<T> for RangeFull {
    fn start_bound(&self) -> Bound<&T> { Bound::Unbounded }
    fn end_bound(&self) -> Bound<&T> { Bound::Unbounded }
}

impl<T> RangeBounds<T> for RangeFrom<T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Included(&self.start) }
    fn end_bound(&self) -> Bound<&T> { Bound::Unbounded }
}

impl<T> RangeBounds<T> for RangeTo<T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Unbounded }
    fn end_bound(&self) -> Bound<&T> { Bound::Excluded(&self.end) }
}

impl<T> RangeBounds<T> for Range<T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Included(&self.start) }
    fn end_bound(&self) -> Bound<&T> { Bound::Excluded(&self.end) }
}

impl<T> RangeBounds<T> for RangeInclusive<T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Included(&self.start) }
    fn end_bound(&self) -> Bound<&T> {
        if self.exhausted {
            Bound::Excluded(&self.end)
        } else {
            Bound::Included(&self.end)
        }
    }
}

impl<T> RangeBounds<T> for RangeToInclusive<T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Unbounded }
    fn end_bound(&self) -> Bound<&T> { Bound::Included(&self.end) }
}

impl<T> RangeBounds<T> for (Bound<T>, Bound<T>) {
    fn start_bound(&self) -> Bound<&T> { self.0.as_ref() }
    fn end_bound(&self) -> Bound<&T> { self.1.as_ref() }
}

impl<'a, T: ?Sized + 'a> RangeBounds<T> for (Bound<&'a T>, Bound<&'a T>) {
    fn start_bound(&self) -> Bound<&T> { self.0 }
    fn end_bound(&self) -> Bound<&T> { self.1 }
}

impl<T: ?Sized> RangeBounds<T> for RangeFrom<&T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Included(self.start) }
    fn end_bound(&self) -> Bound<&T> { Bound::Unbounded }
}

impl<T: ?Sized> RangeBounds<T> for RangeTo<&T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Unbounded }
    fn end_bound(&self) -> Bound<&T> { Bound::Excluded(self.end) }
}

impl<T: ?Sized> RangeBounds<T> for Range<&T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Included(self.start) }
    fn end_bound(&self) -> Bound<&T> { Bound::Excluded(self.end) }
}

impl<T: ?Sized> RangeBounds<T> for RangeInclusive<&T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Included(self.start) }
    fn end_bound(&self) -> Bound<&T> {
        if self.exhausted {
            Bound::Excluded(self.end)
        } else {
            Bound::Included(self.end)
        }
    }
}

impl<T: ?Sized> RangeBounds<T> for RangeToInclusive<&T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Unbounded }
    fn end_bound(&self) -> Bound<&T> { Bound::Included(self.end) }
}
impl<Idx: crate::cmp::PartialOrd> Range<Idx> {
    pub fn contains(&self, item: &Idx) -> bool { *item >= self.start && *item < self.end }
}
impl<Idx: crate::cmp::PartialOrd> RangeInclusive<Idx> {
    pub fn contains(&self, item: &Idx) -> bool { *item >= self.start && *item <= self.end }
}

impl<Idx: crate::default::Default> crate::default::Default for Range<Idx> {
    fn default() -> Self { Range { start: Idx::default(), end: Idx::default() } }
}
