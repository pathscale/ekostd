//! Interior mutability primitives.

use crate::marker::{Copy, Freeze, PointeeSized, Send, Sized, Sync};

#[lang = "unsafe_cell"]
#[repr(transparent)]
pub struct UnsafeCell<T: ?Sized> {
    value: T,
}

impl<T: PointeeSized> !Sync for UnsafeCell<T> {}
impl<T: PointeeSized> !Freeze for UnsafeCell<T> {}

unsafe impl<T: ?Sized + Send> Send for UnsafeCell<T> {}

impl<T> UnsafeCell<T> {
    #[inline]
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    #[inline]
    pub const fn into_inner(self) -> T {
        self.value
    }
}

impl<T: ?Sized> UnsafeCell<T> {
    #[inline]
    pub const fn get(&self) -> *mut T {
        self as *const UnsafeCell<T> as *const T as *mut T
    }

    #[inline]
    pub const fn raw_get(this: *const Self) -> *mut T {
        this as *const T as *mut T
    }
}

#[repr(transparent)]
pub struct Cell<T: ?Sized> {
    value: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for Cell<T> {}
impl<T: ?Sized> !Sync for Cell<T> {}

impl<T> Cell<T> {
    #[inline]
    pub const fn new(value: T) -> Self {
        Self { value: UnsafeCell::new(value) }
    }

    #[inline]
    pub fn set(&self, value: T) {
        let old = self.replace(value);
        crate::mem::drop(old);
    }

    #[inline]
    pub fn replace(&self, value: T) -> T {
        // SAFETY: Cell is !Sync, and safe APIs never return a reference to its value.
        unsafe { crate::mem::replace(&mut *self.value.get(), value) }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        // SAFETY: exclusive access to Cell grants exclusive access to its contents.
        unsafe { &mut *self.value.get() }
    }
}

impl<T: Copy> Cell<T> {
    #[inline]
    pub fn get(&self) -> T {
        // SAFETY: Cell is !Sync and no safe API exposes a shared reference to the value.
        unsafe { *self.value.get() }
    }
}
