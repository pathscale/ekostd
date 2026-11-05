use crate::clone::Clone;
use crate::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use crate::default::Default;
use crate::marker::Copy;
use crate::option::Option::{self, None, Some};

pub struct Duration { secs: u64, nanos: u32 }
impl Copy for Duration {}
impl Clone for Duration { fn clone(&self) -> Self { *self } }
impl PartialEq for Duration {
    fn eq(&self, other: &Self) -> bool { self.secs == other.secs && self.nanos == other.nanos }
}
impl Eq for Duration {}
impl PartialOrd for Duration {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for Duration {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.secs.cmp(&other.secs) {
            Ordering::Equal => self.nanos.cmp(&other.nanos),
            ord => ord,
        }
    }
}

impl Duration {
    pub const ZERO: Self = Self { secs: 0, nanos: 0 };
    pub const MAX: Self = Self { secs: u64::MAX, nanos: 999_999_999 };
    pub const fn new(secs: u64, nanos: u32) -> Self {
        let carry = (nanos / 1_000_000_000) as u64;
        let total_secs = match secs.checked_add(carry) { Some(value) => value, None => u64::MAX };
        let total_nanos = if secs.checked_add(carry).is_some() { nanos % 1_000_000_000 } else { 999_999_999 };
        Self { secs: total_secs, nanos: total_nanos }
    }
    pub const fn from_secs(secs: u64) -> Self { Self { secs, nanos: 0 } }
    pub const fn from_millis(millis: u64) -> Self {
        Self { secs: millis / 1_000, nanos: ((millis % 1_000) * 1_000_000) as u32 }
    }
    pub const fn from_micros(micros: u64) -> Self {
        Self { secs: micros / 1_000_000, nanos: ((micros % 1_000_000) * 1_000) as u32 }
    }
    pub const fn from_nanos(nanos: u64) -> Self {
        Self { secs: nanos / 1_000_000_000, nanos: (nanos % 1_000_000_000) as u32 }
    }
    pub const fn as_secs(&self) -> u64 { self.secs }
    pub const fn subsec_nanos(&self) -> u32 { self.nanos }
    pub const fn subsec_millis(&self) -> u32 { self.nanos / 1_000_000 }
    pub const fn subsec_micros(&self) -> u32 { self.nanos / 1_000 }
    pub const fn as_millis(&self) -> u128 { self.secs as u128 * 1_000 + (self.nanos / 1_000_000) as u128 }
    pub const fn as_micros(&self) -> u128 { self.secs as u128 * 1_000_000 + (self.nanos / 1_000) as u128 }
    pub const fn as_nanos(&self) -> u128 { self.secs as u128 * 1_000_000_000 + self.nanos as u128 }
    pub const fn is_zero(&self) -> bool { self.secs == 0 && self.nanos == 0 }
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        let nanos = self.nanos + rhs.nanos;
        let carry = (nanos >= 1_000_000_000) as u64;
        match self.secs.checked_add(rhs.secs) {
            Some(secs) => match secs.checked_add(carry) {
                Some(total) => Some(Self { secs: total, nanos: nanos % 1_000_000_000 }),
                None => None,
            },
            None => None,
        }
    }
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        if self.secs < rhs.secs || (self.secs == rhs.secs && self.nanos < rhs.nanos) {
            return None;
        }
        if self.nanos >= rhs.nanos {
            Some(Self { secs: self.secs - rhs.secs, nanos: self.nanos - rhs.nanos })
        } else {
            Some(Self { secs: self.secs - rhs.secs - 1, nanos: self.nanos + 1_000_000_000 - rhs.nanos })
        }
    }
    pub const fn saturating_add(self, rhs: Self) -> Self {
        match self.checked_add(rhs) { Some(value) => value, None => Self::MAX }
    }
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        match self.checked_sub(rhs) { Some(value) => value, None => Self::ZERO }
    }
}
impl Default for Duration { fn default() -> Self { Self::ZERO } }
