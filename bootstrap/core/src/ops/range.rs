#[lang = "RangeFull"]
pub struct RangeFull;

#[lang = "Range"]
pub struct Range<Idx> {
    pub start: Idx,
    pub end: Idx,
}

#[lang = "RangeFrom"]
pub struct RangeFrom<Idx> {
    pub start: Idx,
}

#[lang = "RangeTo"]
pub struct RangeTo<Idx> {
    pub end: Idx,
}

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

#[lang = "RangeToInclusive"]
pub struct RangeToInclusive<Idx> {
    pub end: Idx,
}

pub enum Bound<T> {
    Included(T),
    Excluded(T),
    Unbounded,
}

pub trait RangeBounds<T: ?Sized> {
    fn start_bound(&self) -> Bound<&T>;
    fn end_bound(&self) -> Bound<&T>;
}

impl<Idx: crate::clone::Clone> crate::clone::Clone for Range<Idx> {
    fn clone(&self) -> Self { Range { start: self.start.clone(), end: self.end.clone() } }
}
impl<Idx: crate::cmp::PartialEq> crate::cmp::PartialEq for Range<Idx> {
    fn eq(&self, other: &Self) -> bool { self.start == other.start && self.end == other.end }
}
impl<Idx: crate::cmp::Eq> crate::cmp::Eq for Range<Idx> {}
impl<Idx: crate::fmt::Debug> crate::fmt::Debug for Range<Idx> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        match crate::fmt::Debug::fmt(&self.start, f) {
            crate::result::Result::Ok(()) => {}
            error => return error,
        }
        match f.write_str("..") {
            crate::result::Result::Ok(()) => {}
            error => return error,
        }
        crate::fmt::Debug::fmt(&self.end, f)
    }
}
impl<Idx: crate::cmp::PartialOrd> Range<Idx> {
    pub fn contains(&self, item: &Idx) -> bool { *item >= self.start && *item < self.end }
}
impl<Idx: crate::cmp::PartialOrd> RangeInclusive<Idx> {
    pub fn contains(&self, item: &Idx) -> bool { *item >= self.start && *item <= self.end }
}
