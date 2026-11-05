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
#![feature(decl_macro, allow_internal_unstable, structural_match)]
#![feature(adt_const_params, generic_const_parameter_types)]

#[macro_use]
pub mod macros;

pub mod alloc;
pub mod any;
pub mod borrow;
pub mod cell;
pub mod char;
pub mod clone;
pub mod cmp;
pub mod convert;
pub mod default;
pub mod ffi;
pub mod fmt;
pub mod hash;
pub mod hint;
pub mod intrinsics;
pub mod iter;
pub mod marker;
pub mod mem;
pub mod num;
pub mod ops;
pub mod option;
pub mod panic;
pub mod pin;
pub mod prelude;
pub mod ptr;
pub mod result;
pub mod slice;
pub mod str;
pub mod sync;
pub mod time;

pub mod task {
    pub enum Poll<T> {
        #[lang = "Ready"]
        Ready(T),
        #[lang = "Pending"]
        Pending,
    }
    pub struct Context<'a> {
        _marker: crate::marker::PhantomData<&'a ()>,
    }
}

pub mod future {
    #[lang = "future_trait"]
    pub trait Future {
        type Output;
        #[lang = "poll"]
        fn poll(self: crate::pin::Pin<&mut Self>, cx: &mut crate::task::Context<'_>) -> crate::task::Poll<Self::Output>;
    }
}

pub mod error {
    pub trait Error: crate::fmt::Debug + crate::fmt::Display {
        fn source(&self) -> crate::option::Option<&(dyn Error + 'static)> {
            crate::option::Option::None
        }
    }
}

pub use crate::panic::panicking;
pub use crate::macros::{format_args, cfg, env, option_env, line, column, file, stringify, concat, include_str, include_bytes, derive};

#[prelude_import]
#[allow(unused_imports)]
use crate::prelude::rust_2024::*;
