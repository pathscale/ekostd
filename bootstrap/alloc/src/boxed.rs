use crate::alloc;
use core::alloc::Layout;
use core::mem::{self, ManuallyDrop};
use core::ptr::{self, NonNull};

/// The owning pointer inside a `Box`. The compiler reaches the raw pointer through these two fields in order.
#[repr(transparent)]
struct Unique<T: ?Sized> {
    pointer: NonNull<T>,
    _marker: core::marker::PhantomData<T>,
}

/// An owned heap allocation. It is the `owned_box` language item, so `*boxed` can move the value out and the
/// compiler drops the contents before `Drop::drop` runs.
#[lang = "owned_box"]
#[fundamental]
pub struct Box<T: ?Sized>(Unique<T>);

impl<T: ?Sized> core::ops::LegacyReceiver for Box<T> {}

impl<T> Box<T> {
    pub fn new(value: T) -> Self {
        let layout = Layout::new::<T>();
        let raw = unsafe { alloc::alloc(layout).cast::<T>() };
        unsafe { raw.write(value) };
        Self(Unique { pointer: unsafe { NonNull::new_unchecked(raw) }, _marker: core::marker::PhantomData })
    }
}

impl<T: ?Sized> Box<T> {
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        Self(Unique { pointer: unsafe { NonNull::new_unchecked(raw) }, _marker: core::marker::PhantomData })
    }

    pub fn into_raw(value: Self) -> *mut T {
        let value = ManuallyDrop::new(value);
        value.0.pointer.as_ptr()
    }

    pub fn as_ref(&self) -> &T {
        unsafe { self.0.pointer.as_ref() }
    }

    pub fn as_mut(&mut self) -> &mut T {
        unsafe { self.0.pointer.as_mut() }
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

// SAFETY: The value is only dropped, never read; `Unique` tells the drop checker about that drop.
unsafe impl<#[may_dangle] T: ?Sized> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            let raw = self.0.pointer.as_ptr();
            // The compiler has already dropped the contents; only the allocation is left to release.
            let layout = Layout::for_value(&*raw);
            if layout.size() != 0 {
                alloc::dealloc(raw.cast::<u8>(), layout);
            }
        }
    }
}

impl<T: ?Sized> Box<T> {
    pub fn leak<'a>(value: Self) -> &'a mut T
    where
        T: 'a,
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

impl<T: ?Sized + PartialOrd> PartialOrd for Box<T> { fn partial_cmp(&self,other:&Self)->Option<core::cmp::Ordering>{(**self).partial_cmp(&**other)} }
impl<T: ?Sized + Ord> Ord for Box<T> { fn cmp(&self,other:&Self)->core::cmp::Ordering{(**self).cmp(&**other)} }
impl<T: ?Sized + core::hash::Hash> core::hash::Hash for Box<T> { fn hash<H:core::hash::Hasher>(&self,state:&mut H){(**self).hash(state)} }
impl<T: ?Sized + core::fmt::Display> core::fmt::Display for Box<T> { fn fmt(&self,f:&mut core::fmt::Formatter<'_>)->core::fmt::Result{core::fmt::Display::fmt(&**self,f)} }
impl<T: Clone> Clone for Box<T> { fn clone(&self)->Self { Self::new((**self).clone()) } }
impl<T: Clone> Clone for Box<[T]> { fn clone(&self)->Self { crate::vec::Vec::from(&**self).into_boxed_slice() } }
impl Clone for Box<str> { fn clone(&self)->Self { crate::string::String::from(&**self).into_boxed_str() } }
impl<T> Box<T> { pub fn pin(value:T)->core::pin::Pin<Self> { unsafe { core::pin::Pin::new_unchecked(Self::new(value)) } } }
impl<T: ?Sized + core::marker::Unsize<U>, U: ?Sized> core::ops::CoerceUnsized<Box<U>> for Box<T> {}
impl<T: ?Sized + core::marker::Unsize<U>, U: ?Sized> core::ops::DispatchFromDyn<Box<U>> for Box<T> {}

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

impl<T: Clone> From<&[T]> for Box<[T]> {
    fn from(slice: &[T]) -> Self {
        let mut v = crate::vec::Vec::with_capacity(slice.len());
        v.extend_from_slice(slice);
        v.into_boxed_slice()
    }
}

const _: usize = mem::size_of::<NonNull<u8>>();

impl<T: ?Sized + core::marker::Unsize<U>, U: ?Sized> core::ops::CoerceUnsized<Unique<U>> for Unique<T> {}
impl<T: ?Sized + core::marker::Unsize<U>, U: ?Sized> core::ops::DispatchFromDyn<Unique<U>> for Unique<T> {}

// SAFETY: The allocation is owned, so the pointer follows the value it points to.
unsafe impl<T: ?Sized + Send> Send for Unique<T> {}
// SAFETY: Shared access to the pointer is shared access to the value.
unsafe impl<T: ?Sized + Sync> Sync for Unique<T> {}

impl From<&str> for Box<str> {
    fn from(text: &str) -> Self { crate::string::String::from(text).into_boxed_str() }
}

impl<T: ?Sized> core::borrow::Borrow<T> for Box<T> {
    fn borrow(&self) -> &T { &**self }
}

impl<Args: core::marker::Tuple, F: FnOnce<Args> + ?Sized> FnOnce<Args> for Box<F> {
    type Output = <F as FnOnce<Args>>::Output;
    extern "rust-call" fn call_once(self, args: Args) -> Self::Output {
        <F as FnOnce<Args>>::call_once(*self, args)
    }
}

impl<T> Box<T> {
    /// A box with room for one `T`, not yet initialized; a zero-sized `T` gets a dangling pointer and no allocation.
    pub fn new_uninit() -> Box<core::mem::MaybeUninit<T>> {
        let layout = Layout::new::<T>();
        let raw = unsafe { alloc::alloc(layout).cast::<core::mem::MaybeUninit<T>>() };
        // SAFETY: `raw` is non-null and fits `MaybeUninit<T>`, which needs no initialization.
        unsafe { Box::from_raw(raw) }
    }
}

impl<T> Box<core::mem::MaybeUninit<T>> {
    /// Stores `value` in the box and returns it as an initialized `Box<T>`.
    pub fn write(boxed: Self, value: T) -> Box<T> {
        let raw = Box::into_raw(boxed).cast::<T>();
        // SAFETY: `raw` came from a box of `MaybeUninit<T>`, which has the layout of `T`; it is now initialized.
        unsafe {
            raw.write(value);
            Box::from_raw(raw)
        }
    }

    /// # Safety
    /// The value must be initialized.
    pub unsafe fn assume_init(self) -> Box<T> {
        let raw = Box::into_raw(self);
        unsafe { Box::from_raw(raw.cast::<T>()) }
    }
}

impl<T> Box<[T]> {
    /// A boxed slice of `len` uninitialized elements.
    pub fn new_uninit_slice(len: usize) -> Box<[core::mem::MaybeUninit<T>]> {
        let mut vec = crate::vec::Vec::<core::mem::MaybeUninit<T>>::with_capacity(len);
        // SAFETY: `MaybeUninit` needs no initialization.
        unsafe { vec.set_len(len) };
        vec.into_boxed_slice()
    }
}

impl<T> Box<[T]> {
    /// A boxed slice of `len` elements whose bytes are all zero, not yet claimed to be initialized.
    pub fn new_zeroed_slice(len: usize) -> Box<[core::mem::MaybeUninit<T>]> {
        let layout = Layout::array::<T>(len).expect("capacity overflow");
        let raw = unsafe { alloc::alloc_zeroed(layout).cast::<core::mem::MaybeUninit<T>>() };
        // SAFETY: the allocation holds `len` elements of `MaybeUninit<T>`, which have the layout of `T`.
        unsafe { Box::from_raw(ptr::slice_from_raw_parts_mut(raw, len)) }
    }
}
impl<T> Box<[core::mem::MaybeUninit<T>]> {
    /// # Safety
    /// Every element must be initialized.
    pub unsafe fn assume_init(self) -> Box<[T]> {
        let raw = Box::into_raw(self);
        unsafe { Box::from_raw(raw as *mut [T]) }
    }
}

impl<T: Default> Default for Box<T> { fn default() -> Self { Box::new(T::default()) } }
impl<'a, T> core::iter::IntoIterator for &'a Box<[T]> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;
    fn into_iter(self) -> core::slice::Iter<'a, T> { (**self).iter() }
}

impl<T> Default for Box<[T]> { fn default() -> Self { crate::vec::Vec::new().into_boxed_slice() } }

impl<T> core::iter::FromIterator<T> for Box<[T]> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter().collect::<crate::vec::Vec<T>>().into_boxed_slice()
    }
}

impl<T: ?Sized> core::marker::Unpin for Box<T> {}

impl<Args: core::marker::Tuple, F: FnMut<Args> + ?Sized> FnMut<Args> for Box<F> {
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        <F as FnMut<Args>>::call_mut(&mut **self, args)
    }
}
impl<Args: core::marker::Tuple, F: Fn<Args> + ?Sized> Fn<Args> for Box<F> {
    extern "rust-call" fn call(&self, args: Args) -> Self::Output {
        <F as Fn<Args>>::call(&**self, args)
    }
}

impl<I: core::iter::Iterator + ?Sized> core::iter::Iterator for Box<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> { (**self).next() }
    fn size_hint(&self) -> (usize, Option<usize>) { (**self).size_hint() }
}
