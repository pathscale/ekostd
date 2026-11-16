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
    pub const fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

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
    pub const fn into_inner(self) -> T {
        self.value.into_inner()
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        // SAFETY: exclusive access to Cell grants exclusive access to its contents.
        unsafe { &mut *self.value.get() }
    }
}

impl<T: ?Sized> Cell<T> {
    #[inline]
    pub const fn as_ptr(&self) -> *mut T { self.value.get() }
}

impl<T: Copy> Cell<T> {
    #[inline]
    pub fn get(&self) -> T {
        // SAFETY: Cell is !Sync and no safe API exposes a shared reference to the value.
        unsafe { *self.value.get() }
    }
}

impl<T: crate::default::Default> Cell<T> {
    /// Moves the value out, leaving the default.
    pub fn take(&self) -> T { self.replace(crate::default::Default::default()) }
}

/// A cell whose borrows are checked when they are taken, not by the compiler.
pub struct RefCell<T: ?Sized> {
    // 0: not borrowed. Positive: that many shared borrows. -1: one exclusive borrow.
    state: Cell<isize>,
    value: UnsafeCell<T>,
}

pub struct BorrowError;
pub struct BorrowMutError;
impl crate::fmt::Debug for BorrowError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { f.write_str("already mutably borrowed") }
}
impl crate::fmt::Debug for BorrowMutError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { f.write_str("already borrowed") }
}

impl<T> RefCell<T> {
    pub const fn new(value: T) -> Self { Self { state: Cell::new(0), value: UnsafeCell::new(value) } }
    pub fn into_inner(self) -> T { self.value.into_inner() }
    pub fn replace(&self, value: T) -> T { crate::mem::replace(&mut *self.borrow_mut(), value) }
    pub fn take(&self) -> T where T: crate::default::Default { self.replace(crate::default::Default::default()) }
}

impl<T: ?Sized> RefCell<T> {
    pub fn try_borrow(&self) -> Result<Ref<'_, T>, BorrowError> {
        let state = self.state.get();
        if state < 0 { return Err(BorrowError); }
        self.state.set(state + 1);
        Ok(Ref { value: unsafe { &*self.value.get() }, state: &self.state })
    }
    pub fn try_borrow_mut(&self) -> Result<RefMut<'_, T>, BorrowMutError> {
        if self.state.get() != 0 { return Err(BorrowMutError); }
        self.state.set(-1);
        Ok(RefMut { value: unsafe { &mut *self.value.get() }, state: &self.state })
    }
    pub fn borrow(&self) -> Ref<'_, T> {
        match self.try_borrow() {
            Ok(guard) => guard,
            Err(_) => crate::panic::panic_nounwind("RefCell already mutably borrowed"),
        }
    }
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        match self.try_borrow_mut() {
            Ok(guard) => guard,
            Err(_) => crate::panic::panic_nounwind("RefCell already borrowed"),
        }
    }
    pub fn get_mut(&mut self) -> &mut T { self.value.get_mut() }
    pub const fn as_ptr(&self) -> *mut T { self.value.get() }
}

/// A shared borrow of a `RefCell`'s value. Released when dropped.
pub struct Ref<'b, T: ?Sized> { value: &'b T, state: &'b Cell<isize> }
impl<T: ?Sized> crate::ops::Deref for Ref<'_, T> {
    type Target = T;
    fn deref(&self) -> &T { self.value }
}
impl<T: ?Sized> crate::ops::Drop for Ref<'_, T> {
    fn drop(&mut self) { self.state.set(self.state.get() - 1); }
}

/// The exclusive borrow of a `RefCell`'s value. Released when dropped.
pub struct RefMut<'b, T: ?Sized> { value: &'b mut T, state: &'b Cell<isize> }
impl<T: ?Sized> crate::ops::Deref for RefMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &T { self.value }
}
impl<T: ?Sized> crate::ops::DerefMut for RefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T { self.value }
}
impl<T: ?Sized> crate::ops::Drop for RefMut<'_, T> {
    fn drop(&mut self) { self.state.set(0); }
}

/// A cell written at most once.
pub struct OnceCell<T> { inner: UnsafeCell<Option<T>> }
impl<T> OnceCell<T> {
    pub const fn new() -> OnceCell<T> { OnceCell { inner: UnsafeCell::new(None) } }
    pub fn get(&self) -> Option<&T> {
        // SAFETY: the value is only ever written while the cell is empty, and no reference to the empty cell is lent out.
        unsafe { &*self.inner.get() }.as_ref()
    }
    pub fn set(&self, value: T) -> Result<(), T> {
        if self.get().is_some() { return Err(value); }
        unsafe { *self.inner.get() = Some(value); }
        Ok(())
    }
    /// Initialises with `f` if empty. Panics when `f` itself initialised the cell.
    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T {
        if let Some(value) = self.get() { return value; }
        let value = f();
        if self.get().is_some() { panic!("reentrant init"); }
        unsafe { *self.inner.get() = Some(value); }
        match self.get() { Some(value) => value, None => unsafe { crate::hint::unreachable_unchecked() } }
    }
}
impl<T> crate::default::Default for OnceCell<T> { fn default() -> OnceCell<T> { OnceCell::new() } }

impl<T: Clone> Clone for RefCell<T> {
    /// Panics when the value is mutably borrowed.
    fn clone(&self) -> RefCell<T> { RefCell::new(self.borrow().clone()) }
}

impl<T: crate::default::Default> crate::default::Default for RefCell<T> {
    fn default() -> RefCell<T> { RefCell::new(T::default()) }
}

impl<T: crate::default::Default> crate::default::Default for Cell<T> {
    fn default() -> Cell<T> { Cell::new(T::default()) }
}
