use crate::marker::{Destruct, PointeeSized};

#[lang = "maybe_dangling"]
#[repr(transparent)]
pub struct MaybeDangling<P: PointeeSized>(P);

#[lang = "manually_drop"]
#[repr(transparent)]
pub struct ManuallyDrop<T: PointeeSized> {
    value: MaybeDangling<T>,
}

impl<T: crate::marker::Sized> ManuallyDrop<T> {
    pub const fn new(value: T) -> Self { Self { value: MaybeDangling(value) } }

    pub const unsafe fn take(slot: &mut Self) -> T {
        unsafe { crate::intrinsics::read_via_copy(&slot.value.0) }
    }

    pub const fn into_inner(self) -> T { self.value.0 }
}

#[lang = "maybe_uninit"]
#[repr(transparent)]
pub union MaybeUninit<T> {
    uninit: (),
    value: ManuallyDrop<T>,
}

impl<T> MaybeUninit<T> {
    pub const fn uninit() -> Self { Self { uninit: () } }

    pub const fn new(value: T) -> Self {
        Self { value: ManuallyDrop::new(value) }
    }

    pub const unsafe fn assume_init(self) -> T {
        unsafe { ManuallyDrop::into_inner(self.value) }
    }

    pub const fn write(&mut self, value: T) -> &mut T {
        self.value = ManuallyDrop::new(value);
        unsafe { &mut *(&mut self.value as *mut ManuallyDrop<T> as *mut T) }
    }
}

#[rustc_intrinsic]
#[rustc_nounwind]
pub const unsafe fn size_of_val<T: ?Sized>(value: *const T) -> usize;

#[rustc_intrinsic]
#[rustc_nounwind]
pub const unsafe fn align_of_val<T: ?Sized>(value: *const T) -> usize;

trait SizedTypeProperties: crate::marker::Sized {
    const SIZE: usize = crate::intrinsics::size_of::<Self>();
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

pub unsafe fn transmute_copy<Src, Dst>(src: &Src) -> Dst {
    unsafe { crate::intrinsics::read_via_copy(src as *const Src as *const Dst) }
}
