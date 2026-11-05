use crate::alloc;
use core::alloc::Layout;
use core::mem::{self, ManuallyDrop};
use core::ptr::{self, NonNull};

pub struct Box<T: ?Sized> {
    pointer: NonNull<T>,
}

impl<T> Box<T> {
    pub fn new(value: T) -> Self {
        let layout = Layout::new::<T>();
        let raw = unsafe { alloc::alloc(layout).cast::<T>() };
        unsafe { raw.write(value) };
        Self { pointer: unsafe { NonNull::new_unchecked(raw) } }
    }
}

impl<T: ?Sized> Box<T> {
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        Self { pointer: unsafe { NonNull::new_unchecked(raw) } }
    }

    pub fn into_raw(value: Self) -> *mut T {
        let value = ManuallyDrop::new(value);
        value.pointer.as_ptr()
    }

    pub fn as_ref(&self) -> &T {
        unsafe { self.pointer.as_ref() }
    }

    pub fn as_mut(&mut self) -> &mut T {
        unsafe { self.pointer.as_mut() }
    }
}

impl<T: ?Sized> core::ops::Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T: ?Sized> core::ops::DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}

impl<T: ?Sized> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            let raw = self.pointer.as_ptr();
            let layout = Layout::for_value(&*raw);
            ptr::drop_in_place(raw);
            if layout.size() != 0 {
                alloc::dealloc(raw.cast::<u8>(), layout);
            }
        }
    }
}

impl<T: ?Sized> Box<T> {
    pub fn leak(value: Self) -> &'static mut T
    where
        T: 'static,
    {
        let raw = Self::into_raw(value);
        unsafe { &mut *raw }
    }
}

impl<T: ?Sized + core::fmt::Debug> core::fmt::Debug for Box<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + PartialEq> PartialEq for Box<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: ?Sized + Eq> Eq for Box<T> {}

impl<T> From<T> for Box<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: ?Sized> AsRef<T> for Box<T> {
    fn as_ref(&self) -> &T {
        self.as_ref()
    }
}

impl<T: ?Sized> AsMut<T> for Box<T> {
    fn as_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}

impl<T> Box<[T]> {
    pub fn into_vec(value: Self) -> crate::vec::Vec<T> {
        let len = value.len();
        let raw = Self::into_raw(value);
        unsafe { crate::vec::Vec::from_raw_parts(raw.cast::<T>(), len, len) }
    }
}

impl<T> From<crate::vec::Vec<T>> for Box<[T]> {
    fn from(value: crate::vec::Vec<T>) -> Self {
        value.into_boxed_slice()
    }
}

const _: usize = mem::size_of::<NonNull<u8>>();
