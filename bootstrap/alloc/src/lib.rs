#![no_std]
#![feature(lang_items, rustc_attrs, fundamental)]

pub mod alloc;
pub mod borrow;
pub mod boxed;
pub mod collections;
pub mod fmt;
pub mod rc;
pub mod slice;
pub mod str;
pub mod string;
pub mod sync;
pub mod vec;

#[macro_export]
macro_rules! vec {
    () => { $crate::vec::Vec::new() };
    ($elem:expr; $n:expr) => { $crate::vec::Vec::from_elem($elem, $n) };
    ($($item:expr),+ $(,)?) => {{
        let mut result = $crate::vec::Vec::new();
        $(result.push($item);)+
        result
    }};
}