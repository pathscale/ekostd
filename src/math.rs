//! The platform's `libm`, which is where `std`'s float methods go anyway.
//!
//! `f64::ln` and `f64::powf` are `std` rather than `core`, and the reason is not that they are
//! hard: it is that they are not Rust. `std` declares the C symbols and calls them, and every
//! process that links `libc` already has them. So a `no_std` caller that wants a logarithm has
//! two choices, and one of them is a second implementation of the same arithmetic.
//!
//! **The Rust `libm` crate is that second implementation.** It is a port of MUSL's, it is
//! correct, and it is still a different set of bits from the one `std` would have called on this
//! machine. Two implementations agreeing to the last ulp is a thing to verify rather than assume,
//! and a scoring function that changes answers when a crate stops linking `std` is a bug nobody
//! goes looking for. Declaring the symbol is smaller than vendoring the algorithm.
//!
//! Only the functions a caller here has actually needed. A math module that wraps all of
//! `math.h` on speculation is the generality this crate exists to refuse.

// libm is a separate library on Linux and part of libSystem on Apple targets, so `m` is linked
// only off Apple. On Apple the symbols come from libSystem, which every image already links.
#[cfg(not(feature = "libc-rs"))]
#[cfg_attr(not(target_vendor = "apple"), link(name = "m"))]
extern "C" {
    fn log(x: f64) -> f64;
    fn pow(x: f64, y: f64) -> f64;
    #[link_name = "floor"]
    fn c_floor(x: f64) -> f64;
    #[link_name = "ceil"]
    fn c_ceil(x: f64) -> f64;
    #[link_name = "trunc"]
    fn c_trunc(x: f64) -> f64;
}

/// The natural logarithm.
///
/// Rust calls this `ln` and C calls it `log`, which is worth the rename: C's `log10` and Rust's
/// `log` mean different things, so a binding that kept the C spelling would read as the wrong
/// function to anyone who knows the other language.
///
/// Edge cases are the platform's and are the ones `std` documents, because they are the same
/// call: `ln(0.0)` is negative infinity, a negative argument is NaN.
pub fn ln(x: f64) -> f64 {
    #[cfg(feature = "libc-rs")]
    {
        ::libc_rs::math::ln(x)
    }
    #[cfg(not(feature = "libc-rs"))]
    {
        unsafe { log(x) }
    }
}

/// `x` raised to `y`.
///
/// There is no integer-exponent variant here. C has none either, and `f64::powi` is a compiler
/// intrinsic that lowers to this for anything the optimiser cannot unroll, so a caller with an
/// `i32` exponent converts it and loses nothing.
pub fn powf(x: f64, y: f64) -> f64 {
    #[cfg(feature = "libc-rs")]
    {
        ::libc_rs::math::powf(x, y)
    }
    #[cfg(not(feature = "libc-rs"))]
    {
        unsafe { pow(x, y) }
    }
}

/// The largest integer not above `x`, as `f64::floor` would give it with `std`.
pub fn floor(x: f64) -> f64 {
    #[cfg(feature = "libc-rs")]
    {
        ::libc_rs::math::floor(x)
    }
    #[cfg(not(feature = "libc-rs"))]
    {
        unsafe { c_floor(x) }
    }
}

/// The smallest integer not below `x`, as `f64::ceil` would give it with `std`.
pub fn ceil(x: f64) -> f64 {
    #[cfg(feature = "libc-rs")]
    {
        ::libc_rs::math::ceil(x)
    }
    #[cfg(not(feature = "libc-rs"))]
    {
        unsafe { c_ceil(x) }
    }
}

/// `x` with its fractional part dropped, rounding toward zero, as `f64::trunc` would.
pub fn trunc(x: f64) -> f64 {
    #[cfg(feature = "libc-rs")]
    {
        ::libc_rs::math::trunc(x)
    }
    #[cfg(not(feature = "libc-rs"))]
    {
        unsafe { c_trunc(x) }
    }
}
