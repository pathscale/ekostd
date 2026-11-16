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
impl crate::hash::Hash for Duration {
    fn hash<H: crate::hash::Hasher>(&self, state: &mut H) {
        self.secs.hash(state);
        self.nanos.hash(state);
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
    /// Rust's algorithm (`try_from_secs!` for `f64`): exact, rounding to the nearest nanosecond, ties to even.
    /// Panics for a negative, too large or non-finite value, with Rust's messages.
    pub fn from_secs_f64(secs: f64) -> Self {
        const MIN_EXP: i16 = 1 - (1i16 << 11) / 2;
        const MANT_MASK: u64 = (1 << 52) - 1;
        const EXP_MASK: u64 = (1 << 11) - 1;
        const NANOS_PER_SEC: u32 = 1_000_000_000;
        if secs < 0.0 { panic!("cannot convert float seconds to Duration: value is negative"); }
        let bits = secs.to_bits();
        let mant = (bits & MANT_MASK) | (MANT_MASK + 1);
        let exp = ((bits >> 52) & EXP_MASK) as i16 + MIN_EXP;
        let (secs, nanos) = if exp < -31 {
            (0u64, 0u32)
        } else if exp < 0 {
            let t = (mant as u128) << (44 + exp);
            let nanos_offset = 52 + 44;
            let nanos_tmp = (NANOS_PER_SEC as u128) * t;
            let nanos = (nanos_tmp >> nanos_offset) as u32;
            let rem_mask = (1u128 << nanos_offset) - 1;
            let rem_msb_mask = 1u128 << (nanos_offset - 1);
            let rem = nanos_tmp & rem_mask;
            let is_tie = rem == rem_msb_mask;
            let is_even = (nanos & 1) == 0;
            let rem_msb = nanos_tmp & rem_msb_mask == 0;
            let add_ns = !(rem_msb || (is_even && is_tie));
            let nanos = nanos + add_ns as u32;
            if nanos != NANOS_PER_SEC { (0, nanos) } else { (1, 0) }
        } else if exp < 52 {
            let secs = mant >> (52 - exp);
            let t = ((mant << exp) & MANT_MASK) as u128;
            let nanos_offset = 52;
            let nanos_tmp = (NANOS_PER_SEC as u128) * t;
            let nanos = (nanos_tmp >> nanos_offset) as u32;
            let rem_mask = (1u128 << nanos_offset) - 1;
            let rem_msb_mask = 1u128 << (nanos_offset - 1);
            let rem = nanos_tmp & rem_mask;
            let is_tie = rem == rem_msb_mask;
            let is_even = (nanos & 1) == 0;
            let rem_msb = nanos_tmp & rem_msb_mask == 0;
            let add_ns = !(rem_msb || (is_even && is_tie));
            let nanos = nanos + add_ns as u32;
            if nanos != NANOS_PER_SEC { (secs, nanos) } else { (secs + 1, 0) }
        } else if exp < 64 {
            (mant << (exp - 52), 0)
        } else {
            panic!("cannot convert float seconds to Duration: value is either too big or NaN");
        };
        Duration::new(secs, nanos)
    }
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
    pub const fn as_secs_f64(&self) -> f64 { (self.secs as f64) + (self.nanos as f64) / 1_000_000_000.0 }
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
    pub const fn checked_mul(self, rhs: u32) -> Option<Self> {
        let total_nanos = (self.nanos as u64) * (rhs as u64);
        let extra_secs = total_nanos / 1_000_000_000;
        let nanos = (total_nanos % 1_000_000_000) as u32;
        match self.secs.checked_mul(rhs as u64) {
            Some(secs) => match secs.checked_add(extra_secs) {
                Some(total_secs) => Some(Self { secs: total_secs, nanos }),
                None => None,
            },
            None => None,
        }
    }
    pub const fn checked_div(self, rhs: u32) -> Option<Self> {
        if rhs == 0 {
            return None;
        }
        let secs = self.secs / (rhs as u64);
        let carry = self.secs - secs * (rhs as u64);
        let extra_nanos = carry * 1_000_000_000 / (rhs as u64);
        let nanos = (self.nanos as u64) / (rhs as u64) + extra_nanos;
        let secs = secs + nanos / 1_000_000_000;
        let nanos = (nanos % 1_000_000_000) as u32;
        Some(Self { secs, nanos })
    }
}
impl Default for Duration { fn default() -> Self { Self::ZERO } }

impl crate::fmt::Debug for Duration {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.debug_struct("Duration")
            .field("secs", &self.secs)
            .field("nanos", &self.nanos)
            .finish()
    }
}

impl crate::ops::Add for Duration {
    type Output = Duration;
    fn add(self, rhs: Duration) -> Duration {
        self.checked_add(rhs).expect("overflow when adding durations")
    }
}

impl crate::ops::AddAssign for Duration {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl crate::ops::Sub for Duration {
    type Output = Duration;
    fn sub(self, rhs: Duration) -> Duration {
        self.checked_sub(rhs).expect("overflow when subtracting durations")
    }
}

impl crate::ops::SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl crate::ops::Mul<u32> for Duration {
    type Output = Duration;
    fn mul(self, rhs: u32) -> Duration {
        self.checked_mul(rhs).expect("overflow when multiplying duration by scalar")
    }
}

impl crate::ops::Mul<Duration> for u32 {
    type Output = Duration;
    fn mul(self, rhs: Duration) -> Duration {
        rhs * self
    }
}

impl crate::ops::MulAssign<u32> for Duration {
    fn mul_assign(&mut self, rhs: u32) {
        *self = *self * rhs;
    }
}

impl crate::ops::Div<u32> for Duration {
    type Output = Duration;
    fn div(self, rhs: u32) -> Duration {
        self.checked_div(rhs).expect("divided a Duration by zero")
    }
}

impl crate::ops::DivAssign<u32> for Duration {
    fn div_assign(&mut self, rhs: u32) {
        *self = *self / rhs;
    }
}
