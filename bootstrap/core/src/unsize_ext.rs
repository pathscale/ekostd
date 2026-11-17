//! Unsizing coercions from a reference to a raw pointer.
//!
//! A `&mut T` coerces to `*mut U` and `*const U`, and a `&T` to `*const U`, whenever `T: Unsize<U>`
//! (an array to a slice, a sized type to a trait object): the reference becomes a raw pointer and
//! gains the length or vtable word in one coercion. They live here because `ops.rs`, which holds the
//! reference-to-reference and raw-to-raw impls, also holds `const impl`s.

use crate::marker::{PointeeSized, Unsize};
use crate::ops::CoerceUnsized;

impl<'a, T: PointeeSized + Unsize<U>, U: PointeeSized> CoerceUnsized<*mut U> for &'a mut T {}
impl<'a, T: PointeeSized + Unsize<U>, U: PointeeSized> CoerceUnsized<*const U> for &'a mut T {}
impl<'a, T: PointeeSized + Unsize<U>, U: PointeeSized> CoerceUnsized<*const U> for &'a T {}
