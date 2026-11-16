use crate::marker::{Destruct, PointeeSized};
use crate::ops::{Deref, DerefMut};

#[lang = "maybe_dangling"]
#[repr(transparent)]
pub struct MaybeDangling<P: PointeeSized>(P);

impl<P: Copy> Copy for MaybeDangling<P> {}

impl<P: Clone> Clone for MaybeDangling<P> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[lang = "manually_drop"]
#[repr(transparent)]
pub struct ManuallyDrop<T: PointeeSized> {
    value: MaybeDangling<T>,
}

impl<T: Copy> Copy for ManuallyDrop<T> {}

impl<T: Clone> Clone for ManuallyDrop<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self { value: self.value.clone() }
    }
}

impl<T: crate::marker::Sized> ManuallyDrop<T> {
    pub const fn new(value: T) -> Self { Self { value: MaybeDangling(value) } }

    pub const unsafe fn take(slot: &mut Self) -> T {
        unsafe { crate::intrinsics::read_via_copy(&slot.value.0) }
    }

    pub const fn into_inner(self) -> T { self.value.0 }

    /// Drops the value in place. The caller must not use the slot afterwards.
    pub unsafe fn drop(slot: &mut Self) {
        unsafe { crate::ptr::drop_in_place(&mut slot.value.0) }
    }
}

impl<T: ?Sized> Deref for ManuallyDrop<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T { &self.value.0 }
}

impl<T: ?Sized> DerefMut for ManuallyDrop<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T { &mut self.value.0 }
}

#[lang = "maybe_uninit"]
#[repr(transparent)]
pub union MaybeUninit<T> {
    uninit: (),
    value: ManuallyDrop<T>,
}

impl<T: Copy> Copy for MaybeUninit<T> {}

impl<T: Copy> Clone for MaybeUninit<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> MaybeUninit<T> {
    pub const fn uninit() -> Self { Self { uninit: () } }

    pub const fn new(value: T) -> Self {
        Self { value: ManuallyDrop::new(value) }
    }

    pub const unsafe fn assume_init(self) -> T {
        unsafe { ManuallyDrop::into_inner(self.value) }
    }

    /// A value whose bytes are all zero. Still uninitialized as far as the type is concerned.
    pub const fn zeroed() -> Self {
        let mut value = Self::uninit();
        unsafe { crate::intrinsics::write_bytes(value.as_mut_ptr(), 0, 1) };
        value
    }

    pub const fn as_ptr(&self) -> *const T { self as *const Self as *const T }

    pub const unsafe fn assume_init_ref(&self) -> &T { unsafe { &*self.as_ptr() } }
    pub const unsafe fn assume_init_mut(&mut self) -> &mut T { unsafe { &mut *self.as_mut_ptr() } }
    pub unsafe fn assume_init_drop(&mut self) { unsafe { crate::ptr::drop_in_place(self.as_mut_ptr()) } }

    /// Reads the value out by copy. The caller must not let it be read or dropped twice.
    pub const unsafe fn assume_init_read(&self) -> T { unsafe { crate::ptr::read(self.as_ptr()) } }

    pub const fn as_mut_ptr(&mut self) -> *mut T { self as *mut Self as *mut T }

    pub const fn write(&mut self, value: T) -> &mut T {
        self.value = ManuallyDrop::new(value);
        unsafe { &mut *(&mut self.value as *mut ManuallyDrop<T> as *mut T) }
    }
}

/// The size in bytes of the value behind the reference.
#[inline(always)]
pub const fn size_of_val<T: ?Sized>(value: &T) -> usize {
    // SAFETY: A reference points to a valid value.
    unsafe { crate::intrinsics::size_of_val(value) }
}

/// The alignment in bytes of the value behind the reference.
#[inline(always)]
pub const fn align_of_val<T: ?Sized>(value: &T) -> usize {
    // SAFETY: A reference points to a valid value.
    unsafe { crate::intrinsics::align_of_val(value) }
}

#[inline(always)]
pub const unsafe fn size_of_val_raw<T: ?Sized>(value: *const T) -> usize {
    unsafe { crate::intrinsics::size_of_val(value) }
}

#[inline(always)]
pub const unsafe fn align_of_val_raw<T: ?Sized>(value: *const T) -> usize {
    unsafe { crate::intrinsics::align_of_val(value) }
}

trait SizedTypeProperties: crate::marker::Sized {
    #[lang = "mem_size_const"]
    const SIZE: usize = crate::intrinsics::size_of::<Self>();
    #[lang = "mem_align_const"]
    const ALIGN: usize = crate::intrinsics::align_of::<Self>();
}
impl<T> SizedTypeProperties for T {}

trait TypeProperties {
    const NEEDS_DROP: bool = crate::intrinsics::needs_drop::<Self>();
}
impl<T: ?Sized> TypeProperties for T {}

pub const fn size_of<T>() -> usize { <T as SizedTypeProperties>::SIZE }
pub const fn align_of<T>() -> usize { <T as SizedTypeProperties>::ALIGN }

pub const fn needs_drop<T: ?Sized>() -> bool { <T as TypeProperties>::NEEDS_DROP }
pub const fn forget<T: ?Sized>(value: T) { crate::intrinsics::forget(value) }

pub fn replace<T>(dest: &mut T, src: T) -> T {
    unsafe {
        let old = crate::intrinsics::read_via_copy(dest);
        crate::intrinsics::write_via_move(dest, src);
        old
    }
}

pub fn swap<T>(x: &mut T, y: &mut T) {
    unsafe {
        let tmp = crate::intrinsics::read_via_copy(x);
        crate::intrinsics::write_via_move(x, crate::intrinsics::read_via_copy(y));
        crate::intrinsics::write_via_move(y, tmp);
    }
}

pub fn take<T: crate::default::Default>(dest: &mut T) -> T {
    replace(dest, T::default())
}

pub const fn drop<T: [const] Destruct>(value: T) { let _ = value; }

pub use crate::intrinsics::transmute;

/// A value of `T` whose bytes are all zero. Sound only for types where that is a valid value.
pub const unsafe fn zeroed<T>() -> T {
    unsafe { MaybeUninit::<T>::zeroed().assume_init() }
}

pub unsafe fn transmute_copy<Src, Dst>(src: &Src) -> Dst {
    unsafe { crate::intrinsics::read_via_copy(src as *const Src as *const Dst) }
}

/// The byte offset of a field in a type. Expanded by the compiler.
#[allow_internal_unstable(builtin_syntax)]
pub macro offset_of($Container:ty, $($fields:expr)+ $(,)?) {
    {builtin # offset_of($Container, $($fields)+)}
}

/// An alignment in bytes: a power of two. The `global_allocator` expansion passes alignments to
/// the allocator entry points as this type, which has the representation of `usize`.
#[repr(transparent)]
pub struct Alignment(usize);

impl crate::marker::Copy for Alignment {}
impl crate::clone::Clone for Alignment {
    fn clone(&self) -> Self { *self }
}

impl Alignment {
    pub const fn as_usize(self) -> usize { self.0 }
}

/// What `discriminant` returns: comparable, hashable, copyable, and nothing else.
pub struct Discriminant<T>(<T as crate::intrinsics::DiscriminantKind>::Discriminant);
impl<T> crate::marker::Copy for Discriminant<T> {}
impl<T> crate::clone::Clone for Discriminant<T> { fn clone(&self) -> Self { *self } }
impl<T> crate::cmp::PartialEq for Discriminant<T> { fn eq(&self, rhs: &Self) -> bool { self.0 == rhs.0 } }
impl<T> crate::cmp::Eq for Discriminant<T> {}
impl<T> crate::hash::Hash for Discriminant<T> { fn hash<H: crate::hash::Hasher>(&self, state: &mut H) { self.0.hash(state); } }
impl<T> crate::fmt::Debug for Discriminant<T> {
    fn fmt(&self, fmt: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { fmt.debug_tuple("Discriminant").field(&self.0).finish() }
}
pub const fn discriminant<T>(v: &T) -> Discriminant<T> { Discriminant(crate::intrinsics::discriminant_value(v)) }
