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
    // 0: not borrowed. Positive: that many shared borrows. Negative: that many live exclusive borrows
    // (one, or more after `RefMut::map_split`), all on disjoint parts of the value.
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
        if state < 0 || state == isize::MAX { return Err(BorrowError); }
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
    fn drop(&mut self) { self.state.set(self.state.get() + 1); }
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

impl<T> Cell<T> {
    /// Exchanges the values of two cells. Does nothing when both are the same cell.
    #[inline]
    pub fn swap(&self, other: &Self) {
        if self as *const Cell<T> == other as *const Cell<T> { return; }
        // SAFETY: Cell is !Sync and both cells are distinct, so no other reference to either value exists.
        unsafe { crate::mem::swap(&mut *self.value.get(), &mut *other.value.get()) }
    }
}

impl<T: ?Sized> Cell<T> {
    /// Views a mutable reference as a cell over the same place.
    #[inline]
    pub fn from_mut(t: &mut T) -> &Cell<T> {
        // SAFETY: Cell<T> is repr(transparent) over T, and `&mut T` grants unique access for the returned lifetime.
        unsafe { &*(t as *mut T as *const Cell<T>) }
    }
}

impl<T: Copy> Cell<T> {
    /// Replaces the value with `f` applied to the current value.
    #[inline]
    pub fn update(&self, f: impl FnOnce(T) -> T) {
        let old = self.get();
        self.set(f(old));
    }
}

impl<T> Cell<[T]> {
    /// Views a cell of a slice as a slice of cells.
    pub fn as_slice_of_cells(&self) -> &[Cell<T>] {
        // SAFETY: Cell<T> has the same layout as T, so Cell<[T]> has the same layout as [Cell<T>].
        unsafe { &*(self as *const Cell<[T]> as *const [Cell<T>]) }
    }
}

impl<T, const N: usize> Cell<[T; N]> {
    /// Views a cell of an array as an array of cells.
    pub fn as_array_of_cells(&self) -> &[Cell<T>; N] {
        // SAFETY: Cell<T> has the same layout as T, so Cell<[T; N]> has the same layout as [Cell<T>; N].
        unsafe { &*(self as *const Cell<[T; N]> as *const [Cell<T>; N]) }
    }
}

impl<T: Copy> Clone for Cell<T> {
    fn clone(&self) -> Cell<T> { Cell::new(self.get()) }
}

impl<T: crate::cmp::PartialEq + Copy> crate::cmp::PartialEq for Cell<T> {
    fn eq(&self, other: &Cell<T>) -> bool { self.get() == other.get() }
}

impl<T: crate::cmp::Eq + Copy> crate::cmp::Eq for Cell<T> {}

impl<T: crate::cmp::PartialOrd + Copy> crate::cmp::PartialOrd for Cell<T> {
    fn partial_cmp(&self, other: &Cell<T>) -> Option<crate::cmp::Ordering> {
        crate::cmp::PartialOrd::partial_cmp(&self.get(), &other.get())
    }
}

impl<T: crate::cmp::Ord + Copy> crate::cmp::Ord for Cell<T> {
    fn cmp(&self, other: &Cell<T>) -> crate::cmp::Ordering { crate::cmp::Ord::cmp(&self.get(), &other.get()) }
}

impl<T> crate::convert::From<T> for Cell<T> {
    fn from(t: T) -> Cell<T> { Cell::new(t) }
}

impl crate::fmt::Display for BorrowError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { f.write_str("already mutably borrowed") }
}
impl crate::fmt::Display for BorrowMutError {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { f.write_str("already borrowed") }
}

impl<T> RefCell<T> {
    /// Replaces the value with the one `f` computes from it, returning the old value. Panics when borrowed.
    pub fn replace_with<F: FnOnce(&mut T) -> T>(&self, f: F) -> T {
        let mut guard = self.borrow_mut();
        let replacement = f(&mut *guard);
        crate::mem::replace(&mut *guard, replacement)
    }

    /// Exchanges the values of two cells. Panics when either is borrowed, including when both are the same cell.
    pub fn swap(&self, other: &Self) {
        crate::mem::swap(&mut *self.borrow_mut(), &mut *other.borrow_mut())
    }
}

impl<T: ?Sized> RefCell<T> {
    /// Borrows the value without a guard. Fails when the value is mutably borrowed.
    ///
    /// # Safety
    /// The caller must not take a mutable borrow while the returned reference is alive.
    pub unsafe fn try_borrow_unguarded(&self) -> Result<&T, BorrowError> {
        if self.state.get() < 0 { return Err(BorrowError); }
        // SAFETY: no exclusive borrow is live, and the caller promises none starts while the reference lives.
        Ok(unsafe { &*self.value.get() })
    }
}

impl<T> crate::convert::From<T> for RefCell<T> {
    fn from(t: T) -> RefCell<T> { RefCell::new(t) }
}

impl<T: ?Sized + crate::cmp::PartialEq> crate::cmp::PartialEq for RefCell<T> {
    /// Panics when either value is mutably borrowed.
    fn eq(&self, other: &RefCell<T>) -> bool { *self.borrow() == *other.borrow() }
}

impl<T: ?Sized + crate::cmp::Eq> crate::cmp::Eq for RefCell<T> {}

impl<T: ?Sized + crate::cmp::PartialOrd> crate::cmp::PartialOrd for RefCell<T> {
    /// Panics when either value is mutably borrowed.
    fn partial_cmp(&self, other: &RefCell<T>) -> Option<crate::cmp::Ordering> {
        crate::cmp::PartialOrd::partial_cmp(&*self.borrow(), &*other.borrow())
    }
}

impl<T: ?Sized + crate::cmp::Ord> crate::cmp::Ord for RefCell<T> {
    /// Panics when either value is mutably borrowed.
    fn cmp(&self, other: &RefCell<T>) -> crate::cmp::Ordering { crate::cmp::Ord::cmp(&*self.borrow(), &*other.borrow()) }
}

impl<'b, T: ?Sized> Ref<'b, T> {
    /// Takes one more shared borrow of the same value.
    pub fn clone(orig: &Ref<'b, T>) -> Ref<'b, T> {
        orig.state.set(orig.state.get() + 1);
        Ref { value: orig.value, state: orig.state }
    }

    /// Narrows the borrow to a part of the value.
    pub fn map<U: ?Sized, F: FnOnce(&T) -> &U>(orig: Ref<'b, T>, f: F) -> Ref<'b, U> {
        let value = f(orig.value);
        let state = orig.state;
        crate::mem::forget(orig);
        Ref { value, state }
    }

    /// Narrows the borrow to a part of the value, or gives the borrow back when `f` returns `None`.
    pub fn filter_map<U: ?Sized, F: FnOnce(&T) -> Option<&U>>(orig: Ref<'b, T>, f: F) -> Result<Ref<'b, U>, Self> {
        match f(orig.value) {
            Some(value) => {
                let state = orig.state;
                crate::mem::forget(orig);
                Ok(Ref { value, state })
            }
            None => Err(orig),
        }
    }

    /// Splits the borrow into two borrows of parts of the value.
    pub fn map_split<U: ?Sized, V: ?Sized, F: FnOnce(&T) -> (&U, &V)>(orig: Ref<'b, T>, f: F) -> (Ref<'b, U>, Ref<'b, V>) {
        let (a, b) = f(orig.value);
        let state = orig.state;
        crate::mem::forget(orig);
        state.set(state.get() + 1);
        (Ref { value: a, state }, Ref { value: b, state })
    }
}

impl<T: ?Sized + crate::fmt::Display> crate::fmt::Display for Ref<'_, T> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { crate::fmt::Display::fmt(self.value, f) }
}

impl<'b, T: ?Sized> RefMut<'b, T> {
    /// Narrows the exclusive borrow to a part of the value.
    pub fn map<U: ?Sized, F: FnOnce(&mut T) -> &mut U>(mut orig: RefMut<'b, T>, f: F) -> RefMut<'b, U> {
        let state = orig.state;
        let ptr: *mut T = &mut *orig.value;
        crate::mem::forget(orig);
        // SAFETY: the guard was forgotten without releasing the borrow, so `ptr` stays uniquely borrowed for 'b.
        RefMut { value: f(unsafe { &mut *ptr }), state }
    }

    /// Narrows the exclusive borrow to a part of the value, or gives the borrow back when `f` returns `None`.
    pub fn filter_map<U: ?Sized, F: FnOnce(&mut T) -> Option<&mut U>>(mut orig: RefMut<'b, T>, f: F) -> Result<RefMut<'b, U>, Self> {
        let state = orig.state;
        let ptr: *mut T = &mut *orig.value;
        // SAFETY: `orig` is not used while the reference from `ptr` is alive. On `None` it is no longer alive.
        match f(unsafe { &mut *ptr }) {
            Some(value) => {
                crate::mem::forget(orig);
                Ok(RefMut { value, state })
            }
            None => Err(orig),
        }
    }

    /// Splits the exclusive borrow into two exclusive borrows of disjoint parts of the value.
    pub fn map_split<U: ?Sized, V: ?Sized, F: FnOnce(&mut T) -> (&mut U, &mut V)>(mut orig: RefMut<'b, T>, f: F) -> (RefMut<'b, U>, RefMut<'b, V>) {
        let state = orig.state;
        let ptr: *mut T = &mut *orig.value;
        crate::mem::forget(orig);
        // SAFETY: the guard was forgotten without releasing the borrow, so `ptr` stays uniquely borrowed for 'b.
        let (a, b) = f(unsafe { &mut *ptr });
        // One more live exclusive borrow: each half adds one back when dropped.
        state.set(state.get() - 1);
        (RefMut { value: a, state }, RefMut { value: b, state })
    }
}

impl<T: ?Sized + crate::fmt::Debug> crate::fmt::Debug for RefMut<'_, T> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { crate::fmt::Debug::fmt(&*self.value, f) }
}

impl<T: ?Sized + crate::fmt::Display> crate::fmt::Display for RefMut<'_, T> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { crate::fmt::Display::fmt(&*self.value, f) }
}

impl<T> OnceCell<T> {
    /// The value, mutably, if set.
    pub fn get_mut(&mut self) -> Option<&mut T> { self.inner.get_mut().as_mut() }

    /// Moves the value out, leaving the cell empty.
    pub fn take(&mut self) -> Option<T> { crate::mem::replace(self, OnceCell::new()).into_inner() }

    /// The value, if set.
    pub fn into_inner(self) -> Option<T> { self.inner.into_inner() }
}

impl<T: Clone> Clone for OnceCell<T> {
    fn clone(&self) -> OnceCell<T> {
        let cell = OnceCell::new();
        if let Some(value) = self.get() {
            let _ = cell.set(value.clone());
        }
        cell
    }
}

impl<T: crate::fmt::Debug> crate::fmt::Debug for OnceCell<T> {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        /// Prints `<uninit>` in place of the missing value.
        struct Uninit;
        impl crate::fmt::Debug for Uninit {
            fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result { f.write_str("<uninit>") }
        }
        let mut d = f.debug_tuple("OnceCell");
        match self.get() {
            Some(value) => { d.field(value); }
            None => { d.field(&Uninit); }
        }
        d.finish()
    }
}

impl<T: crate::cmp::PartialEq> crate::cmp::PartialEq for OnceCell<T> {
    fn eq(&self, other: &OnceCell<T>) -> bool { self.get() == other.get() }
}

impl<T: crate::cmp::Eq> crate::cmp::Eq for OnceCell<T> {}

impl<T> crate::convert::From<T> for OnceCell<T> {
    /// A cell already holding `value`.
    fn from(value: T) -> OnceCell<T> { OnceCell { inner: UnsafeCell::new(Some(value)) } }
}
