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
