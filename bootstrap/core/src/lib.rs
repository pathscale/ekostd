//! Minimal bootstrap candidate for the selected rustc, independent of installed core.
#![no_core]
#![rustc_coherence_is_core]
#![crate_name = "core"]
#![crate_type = "rlib"]
#![feature(no_core, lang_items, rustc_attrs, intrinsics)]
#![feature(auto_traits, negative_impls, fundamental)]
#![feature(freeze_impls, unsized_fn_params)]
#![feature(const_precise_live_drops, transparent_unions)]
#![feature(const_trait_impl, const_destruct, unboxed_closures, prelude_import)]

pub mod cell;
pub mod clone;
pub mod cmp;
pub mod convert;
pub mod default;
pub mod hint;
pub mod intrinsics;
pub mod marker;
pub mod mem;
pub mod ops;
pub mod option;
pub mod prelude;
pub mod result;

#[prelude_import]
#[allow(unused_imports)]
use crate::prelude::rust_2024::*;
