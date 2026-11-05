use core::cell::Cell;
use core::cmp::Ordering;
use core::mem::ManuallyDrop;
use core::ptr::NonNull;

struct Counts { strong: Cell<usize>, weak: Cell<usize> }

pub struct Rc<T: ?Sized> { value: NonNull<T>, counts: NonNull<Counts> }
pub struct Weak<T: ?Sized> { value: NonNull<T>, counts: NonNull<Counts> }

impl<T: ?Sized> core::ops::LegacyReceiver for Rc<T> {}

impl<T> Rc<T> {
    pub fn new(value: T) -> Self {
        let value = crate::boxed::Box::into_raw(crate::boxed::Box::new(value));
        let counts = crate::boxed::Box::into_raw(crate::boxed::Box::new(Counts { strong: Cell::new(1), weak: Cell::new(0) }));
        Self { value: unsafe { NonNull::new_unchecked(value) }, counts: unsafe { NonNull::new_unchecked(counts) } }
    }
    pub fn from_raw(value: *const T) -> Self {
        let counts = crate::boxed::Box::into_raw(crate::boxed::Box::new(Counts { strong: Cell::new(1), weak: Cell::new(0) }));
        Self { value: unsafe { NonNull::new_unchecked(value as *mut T) }, counts: unsafe { NonNull::new_unchecked(counts) } }
    }
    pub fn into_raw(value: Self) -> *const T {
        let value = ManuallyDrop::new(value);
        value.value.as_ptr()
    }
    pub fn make_mut(this: &mut Self) -> &mut T where T: Clone {
        if Self::strong_count(this) != 1 || Self::weak_count(this) != 0 {
            *this = Self::new((**this).clone());
        }
        unsafe { this.value.as_mut() }
    }
}

impl<T: ?Sized> Rc<T> {
    pub fn downgrade(this: &Self) -> Weak<T> {
        let counts = unsafe { this.counts.as_ref() };
        counts.weak.set(counts.weak.get() + 1);
        Weak { value: this.value, counts: this.counts }
    }
    pub fn strong_count(this: &Self) -> usize { unsafe { this.counts.as_ref().strong.get() } }
    pub fn weak_count(this: &Self) -> usize { unsafe { this.counts.as_ref().weak.get() } }
    pub fn as_ptr(this: &Self) -> *const T { this.value.as_ptr() }
    pub fn ptr_eq(this: &Self, other: &Self) -> bool { this.value == other.value }
    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        if Self::strong_count(this) == 1 && Self::weak_count(this) == 0 { Some(unsafe { this.value.as_mut() }) } else { None }
    }
}

impl<T> Weak<T> {
    pub fn new() -> Self {
        let counts = crate::boxed::Box::into_raw(crate::boxed::Box::new(Counts { strong: Cell::new(0), weak: Cell::new(1) }));
        Self { value: NonNull::dangling(), counts: unsafe { NonNull::new_unchecked(counts) } }
    }
}

impl<T: ?Sized> Weak<T> {
    pub fn upgrade(&self) -> Option<Rc<T>> {
        let counts = unsafe { self.counts.as_ref() };
        let strong = counts.strong.get();
        if strong == 0 { None } else {
            counts.strong.set(strong + 1);
            Some(Rc { value: self.value, counts: self.counts })
        }
    }
}

impl<T: ?Sized> Clone for Rc<T> {
    fn clone(&self) -> Self {
        let counts = unsafe { self.counts.as_ref() };
        counts.strong.set(counts.strong.get() + 1);
        Self { value: self.value, counts: self.counts }
    }
}
impl<T: ?Sized> Clone for Weak<T> {
    fn clone(&self) -> Self {
        let counts = unsafe { self.counts.as_ref() };
        counts.weak.set(counts.weak.get() + 1);
        Self { value: self.value, counts: self.counts }
    }
}
impl<T: ?Sized> Drop for Rc<T> {
    fn drop(&mut self) {
        let counts = unsafe { self.counts.as_ref() };
        let strong = counts.strong.get() - 1;
        counts.strong.set(strong);
        if strong == 0 {
            unsafe { drop(crate::boxed::Box::from_raw(self.value.as_ptr())) };
            if counts.weak.get() == 0 { unsafe { drop(crate::boxed::Box::from_raw(self.counts.as_ptr())) }; }
        }
    }
}
impl<T: ?Sized> Drop for Weak<T> {
    fn drop(&mut self) {
        let counts = unsafe { self.counts.as_ref() };
        let weak = counts.weak.get() - 1;
        counts.weak.set(weak);
        if weak == 0 && counts.strong.get() == 0 { unsafe { drop(crate::boxed::Box::from_raw(self.counts.as_ptr())) }; }
    }
}
impl<T: ?Sized> core::ops::Deref for Rc<T> {
    type Target = T;
    fn deref(&self) -> &T { unsafe { self.value.as_ref() } }
}
impl<T: ?Sized + PartialEq> PartialEq for Rc<T> { fn eq(&self, other: &Self) -> bool { **self == **other } }
impl<T: ?Sized + Eq> Eq for Rc<T> {}
impl<T: ?Sized + PartialOrd> PartialOrd for Rc<T> { fn partial_cmp(&self, other: &Self) -> Option<Ordering> { (**self).partial_cmp(&**other) } }
impl<T: ?Sized + Ord> Ord for Rc<T> { fn cmp(&self, other: &Self) -> Ordering { (**self).cmp(&**other) } }
impl<T: ?Sized + core::fmt::Debug> core::fmt::Debug for Rc<T> { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { core::fmt::Debug::fmt(&**self, f) } }
impl<T: ?Sized + core::fmt::Display> core::fmt::Display for Rc<T> { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { core::fmt::Display::fmt(&**self, f) } }
impl<T: Default> Default for Rc<T> { fn default() -> Self { Self::new(T::default()) } }
impl<T> From<T> for Rc<T> { fn from(value: T) -> Self { Self::new(value) } }
