//! Compiler-recognized declarations matching the selected rustc's core source.
use crate::clone::Clone;
use crate::cmp::{Eq, PartialEq};
use crate::marker::{Copy, Sized};

#[repr(u32)]
pub enum AtomicOrdering { Relaxed = 0, Release = 1, Acquire = 2, AcqRel = 3, SeqCst = 4 }
impl Copy for AtomicOrdering {}
impl Clone for AtomicOrdering { fn clone(&self) -> Self { *self } }
impl PartialEq for AtomicOrdering {
    fn eq(&self, other: &Self) -> bool { (*self as u32) == (*other as u32) }
}
impl Eq for AtomicOrdering {}
impl crate::marker::StructuralPartialEq for AtomicOrdering {}
impl crate::marker::ConstParamTy_ for AtomicOrdering {}

#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn atomic_load<T: Copy, const ORD: AtomicOrdering>(src: *const T) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn atomic_store<T: Copy, const ORD: AtomicOrdering>(dst: *mut T, val: T);
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn atomic_xchg<T: Copy, const ORD: AtomicOrdering>(dst: *mut T, src: T) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn atomic_cxchg<T: Copy, const SUCCESS: AtomicOrdering, const FAILURE: AtomicOrdering>(dst: *mut T, old: T, src: T) -> (T, bool);
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn atomic_cxchgweak<T: Copy, const SUCCESS: AtomicOrdering, const FAILURE: AtomicOrdering>(dst: *mut T, old: T, src: T) -> (T, bool);
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn atomic_xadd<T: Copy, U: Copy, const ORD: AtomicOrdering>(dst: *mut T, src: U) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn atomic_xsub<T: Copy, U: Copy, const ORD: AtomicOrdering>(dst: *mut T, src: U) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn atomic_and<T: Copy, U: Copy, const ORD: AtomicOrdering>(dst: *mut T, src: U) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn atomic_or<T: Copy, U: Copy, const ORD: AtomicOrdering>(dst: *mut T, src: U) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn atomic_xor<T: Copy, U: Copy, const ORD: AtomicOrdering>(dst: *mut T, src: U) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn atomic_fence<const ORD: AtomicOrdering>();
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn atomic_singlethreadfence<const ORD: AtomicOrdering>();
#[rustc_intrinsic]
#[rustc_nounwind]
pub fn wrapping_add<T: Copy>(a: T, b: T) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
pub fn wrapping_sub<T: Copy>(a: T, b: T) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
pub fn wrapping_mul<T: Copy>(a: T, b: T) -> T;

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
pub const fn abort() -> !;

#[lang = "discriminant_kind"]
pub trait DiscriminantKind {
    #[lang = "discriminant_type"]
    type Discriminant: Clone + Copy + crate::fmt::Debug + Eq + PartialEq + crate::hash::Hash + crate::cmp::PartialOrd + crate::cmp::Ord;
}

#[rustc_intrinsic]
#[rustc_nounwind]
pub const fn discriminant_value<T>(v: &T) -> <T as DiscriminantKind>::Discriminant;
