//! Compiler-recognized declarations matching the selected rustc's core source.
use crate::marker::Sized;

#[rustc_intrinsic]
#[rustc_nounwind]
#[rustc_comptime]
pub fn size_of<T>() -> usize;

#[rustc_intrinsic]
#[rustc_nounwind]
#[rustc_comptime]
pub fn align_of<T>() -> usize;

#[rustc_intrinsic]
#[rustc_nounwind]
#[rustc_comptime]
pub fn needs_drop<T: ?Sized>() -> bool;

#[rustc_intrinsic]
#[rustc_nounwind]
pub const fn forget<T: ?Sized>(_: T);

#[rustc_intrinsic]
#[rustc_nounwind]
pub const unsafe fn transmute<Src, Dst>(src: Src) -> Dst;

#[rustc_intrinsic]
#[rustc_nounwind]
pub const unsafe fn read_via_copy<T>(ptr: *const T) -> T;

#[rustc_intrinsic]
#[rustc_nounwind]
pub const unsafe fn write_via_move<T>(ptr: *mut T, value: T);

#[rustc_intrinsic]
#[rustc_nounwind]
pub const unsafe fn copy_nonoverlapping<T>(src: *const T, dst: *mut T, count: usize);

#[rustc_intrinsic]
#[rustc_nounwind]
pub const unsafe fn copy<T>(src: *const T, dst: *mut T, count: usize);

#[rustc_intrinsic]
#[rustc_nounwind]
pub const unsafe fn write_bytes<T>(dst: *mut T, val: u8, count: usize);

#[rustc_intrinsic]
#[rustc_nounwind]
pub const unsafe fn arith_offset<T>(dst: *const T, offset: isize) -> *const T;

#[rustc_intrinsic]
#[rustc_nounwind]
pub const unsafe fn volatile_load<T>(src: *const T) -> T;

#[rustc_intrinsic]
#[rustc_nounwind]
pub const unsafe fn volatile_store<T>(dst: *mut T, val: T);

#[rustc_intrinsic]
#[rustc_nounwind]
pub const fn assert_inhabited<T>();

#[rustc_intrinsic]
#[rustc_nounwind]
pub const fn assert_zero_valid<T>();

#[rustc_intrinsic]
#[rustc_nounwind]
pub const fn assert_mem_uninitialized_valid<T>();

#[rustc_intrinsic]
#[rustc_nounwind]
pub const unsafe fn unreachable() -> !;

#[rustc_intrinsic]
#[rustc_nounwind]
pub fn abort() -> !;
