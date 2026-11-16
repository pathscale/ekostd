use crate::alloc;
use crate::boxed::Box;
use core::alloc::Layout;
use core::cell::Cell;
use core::cmp::Ordering;
use core::mem::{self, ManuallyDrop};
use core::ptr::{self, NonNull};

/// The counts and the value in one allocation, so a pointer to the value leads back to its counts.
/// `weak` counts the `Weak`s plus one held together by all the strong references.
#[repr(C)]
struct RcInner<T: ?Sized> {
    strong: Cell<usize>,
    weak: Cell<usize>,
    value: T,
}

pub struct Rc<T: ?Sized> { ptr: NonNull<RcInner<T>> }
/// `Weak::new` points at the address `usize::MAX`, which is never an allocation.
pub struct Weak<T: ?Sized> { ptr: NonNull<RcInner<T>> }

impl<T: ?Sized> core::ops::LegacyReceiver for Rc<T> {}

#[inline]
fn header_layout() -> Layout {
    Layout::new::<RcInner<()>>()
}

/// Bytes from the start of the allocation to the value `ptr` points at.
#[inline]
unsafe fn data_offset_for_ptr<T: ?Sized>(ptr: *const T) -> usize {
    let val_align = unsafe { mem::align_of_val_raw(ptr) };
    let layout = header_layout();
    layout.size() + layout.padding_needed_for(val_align)
}

/// The layout the allocation was made with: header, padding, value, rounded up to the alignment.
#[inline]
unsafe fn layout_for_inner<T: ?Sized>(ptr: *const RcInner<T>) -> Layout {
    let value_ptr = unsafe { &raw const (*ptr).value };
    let size = unsafe { mem::size_of_val_raw(value_ptr) };
    let align = unsafe { mem::align_of_val_raw(value_ptr) };
    let header = header_layout();
    let pad = header.padding_needed_for(align);
    let total_align = if header.align() > align { header.align() } else { align };
    let total_size = match header.size().checked_add(pad) {
        Some(n) => n.checked_add(size).unwrap(),
        None => panic!("layout_for_inner overflow"),
    };
    let total_size = (total_size + total_align - 1) & !(total_align - 1);
    unsafe { Layout::from_size_align_unchecked(total_size, total_align) }
}

impl<T> Rc<T> {
    pub fn new(value: T) -> Self {
        let boxed = Box::new(RcInner { strong: Cell::new(1), weak: Cell::new(1), value });
        Self { ptr: unsafe { NonNull::new_unchecked(Box::into_raw(boxed)) } }
    }
    /// A unique reference to the value: a clone into a new allocation when other strong references share it,
    /// the value moved into a new allocation when only `Weak`s share it (they can no longer upgrade).
    pub fn make_mut(this: &mut Self) -> &mut T where T: Clone {
        if Self::strong_count(this) != 1 {
            *this = Self::new((**this).clone());
        } else if this.inner().weak.get() != 1 {
            let old = this.ptr;
            let value = unsafe { ptr::read(&raw const (*old.as_ptr()).value) };
            unsafe { old.as_ref() }.strong.set(0);
            // The old Rc is overwritten without a drop: its value was moved out and its strong count is 0.
            unsafe { ptr::write(this, Self::new(value)) };
            drop(Weak { ptr: old });
        }
        unsafe { &mut (*this.ptr.as_ptr()).value }
    }
    /// The value, when this is the only strong reference; otherwise the reference back.
    pub fn try_unwrap(this: Self) -> Result<T, Self> {
        if Self::strong_count(&this) != 1 { return Err(this); }
        let this = ManuallyDrop::new(this);
        this.inner().strong.set(0);
        let value = unsafe { ptr::read(&raw const (*this.ptr.as_ptr()).value) };
        // The implicit weak reference that the strong ones shared is released here.
        drop(Weak { ptr: this.ptr });
        Ok(value)
    }
    /// The value, when this was the only strong reference; otherwise this reference is dropped and `None` returned.
    pub fn into_inner(this: Self) -> Option<T> {
        Self::try_unwrap(this).ok()
    }
    /// The value, moved out when this is the only strong reference, cloned otherwise.
    pub fn unwrap_or_clone(this: Self) -> T where T: Clone {
        match Self::try_unwrap(this) { Ok(value) => value, Err(rc) => (*rc).clone() }
    }
}

impl<T: ?Sized> Rc<T> {
    #[inline]
    fn inner(&self) -> &RcInner<T> {
        unsafe { self.ptr.as_ref() }
    }
    pub fn downgrade(this: &Self) -> Weak<T> {
        let inner = this.inner();
        inner.weak.set(inner.weak.get() + 1);
        Weak { ptr: this.ptr }
    }
    pub fn strong_count(this: &Self) -> usize { this.inner().strong.get() }
    pub fn weak_count(this: &Self) -> usize { this.inner().weak.get() - 1 }
    pub fn as_ptr(this: &Self) -> *const T { unsafe { &raw const (*this.ptr.as_ptr()).value } }
    pub fn into_raw(this: Self) -> *const T {
        let this = ManuallyDrop::new(this);
        Self::as_ptr(&*this)
    }
    /// The Rc that `ptr` (from `Rc::into_raw`) was made from, sharing the counts of its allocation.
    pub unsafe fn from_raw(ptr: *const T) -> Self {
        let offset = unsafe { data_offset_for_ptr(ptr) };
        let byte_ptr = (ptr as *const u8).wrapping_sub(offset);
        let inner_ptr = byte_ptr.with_metadata_of(ptr as *const RcInner<T>) as *mut RcInner<T>;
        Self { ptr: unsafe { NonNull::new_unchecked(inner_ptr) } }
    }
    /// Adds one strong reference to the allocation `ptr` came from (`Rc::into_raw`).
    pub unsafe fn increment_strong_count(ptr: *const T) {
        let rc = ManuallyDrop::new(unsafe { Self::from_raw(ptr) });
        let _extra = ManuallyDrop::new(Clone::clone(&*rc));
    }
    /// Drops one strong reference to the allocation `ptr` came from (`Rc::into_raw`).
    pub unsafe fn decrement_strong_count(ptr: *const T) {
        drop(unsafe { Self::from_raw(ptr) });
    }
    /// Whether both point to the same allocation.
    pub fn ptr_eq(this: &Self, other: &Self) -> bool { ptr::addr_eq(this.ptr.as_ptr(), other.ptr.as_ptr()) }
    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        let inner = this.inner();
        if inner.strong.get() == 1 && inner.weak.get() == 1 { Some(unsafe { &mut (*this.ptr.as_ptr()).value }) } else { None }
    }
}

impl<T> Weak<T> {
    pub const fn new() -> Self {
        Self { ptr: unsafe { NonNull::new_unchecked(usize::MAX as *mut RcInner<T>) } }
    }
}

impl<T: ?Sized> Weak<T> {
    #[inline]
    fn inner(&self) -> Option<&RcInner<T>> {
        if (self.ptr.as_ptr() as *const u8).addr() == usize::MAX { None } else { Some(unsafe { self.ptr.as_ref() }) }
    }
    pub fn upgrade(&self) -> Option<Rc<T>> {
        let inner = self.inner()?;
        let strong = inner.strong.get();
        if strong == 0 { return None; }
        inner.strong.set(strong + 1);
        Some(Rc { ptr: self.ptr })
    }
    pub fn strong_count(&self) -> usize {
        match self.inner() { Some(inner) => inner.strong.get(), None => 0 }
    }
    /// The number of `Weak`s, or 0 once no strong reference is left.
    pub fn weak_count(&self) -> usize {
        match self.inner() { Some(inner) if inner.strong.get() > 0 => inner.weak.get() - 1, _ => 0 }
    }
    /// Whether both point to the same allocation (or both are `Weak::new`).
    pub fn ptr_eq(&self, other: &Self) -> bool { ptr::addr_eq(self.ptr.as_ptr(), other.ptr.as_ptr()) }
}

impl<T: ?Sized> Clone for Rc<T> {
    fn clone(&self) -> Self {
        let inner = self.inner();
        inner.strong.set(inner.strong.get() + 1);
        Self { ptr: self.ptr }
    }
}
impl<T: ?Sized> Clone for Weak<T> {
    fn clone(&self) -> Self {
        if let Some(inner) = self.inner() { inner.weak.set(inner.weak.get() + 1); }
        Self { ptr: self.ptr }
    }
}
impl<T: ?Sized> Drop for Rc<T> {
    fn drop(&mut self) {
        let inner = self.inner();
        let strong = inner.strong.get() - 1;
        inner.strong.set(strong);
        if strong == 0 {
            unsafe { ptr::drop_in_place(&raw mut (*self.ptr.as_ptr()).value) };
            // The implicit weak reference that the strong ones shared is released here.
            drop(Weak { ptr: self.ptr });
        }
    }
}
impl<T: ?Sized> Drop for Weak<T> {
    fn drop(&mut self) {
        let Some(inner) = self.inner() else { return };
        let weak = inner.weak.get() - 1;
        inner.weak.set(weak);
        if weak == 0 {
            unsafe {
                let layout = layout_for_inner(self.ptr.as_ptr());
                alloc::dealloc(self.ptr.as_ptr().cast::<u8>(), layout);
            }
        }
    }
}
impl<T: ?Sized> core::ops::Deref for Rc<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.inner().value }
}
impl<T: ?Sized + PartialEq> PartialEq for Rc<T> { fn eq(&self, other: &Self) -> bool { **self == **other } }
impl<T: ?Sized + Eq> Eq for Rc<T> {}
impl<T: ?Sized + PartialOrd> PartialOrd for Rc<T> { fn partial_cmp(&self, other: &Self) -> Option<Ordering> { (**self).partial_cmp(&**other) } }
impl<T: ?Sized + Ord> Ord for Rc<T> { fn cmp(&self, other: &Self) -> Ordering { (**self).cmp(&**other) } }
impl<T: ?Sized + core::fmt::Debug> core::fmt::Debug for Rc<T> { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { core::fmt::Debug::fmt(&**self, f) } }
impl<T: ?Sized + core::fmt::Display> core::fmt::Display for Rc<T> { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { core::fmt::Display::fmt(&**self, f) } }
impl<T: ?Sized + core::hash::Hash> core::hash::Hash for Rc<T> { fn hash<H: core::hash::Hasher>(&self, state: &mut H) { (**self).hash(state) } }
impl<T: ?Sized> core::borrow::Borrow<T> for Rc<T> { fn borrow(&self) -> &T { self } }
impl<T: ?Sized> AsRef<T> for Rc<T> { fn as_ref(&self) -> &T { self } }
impl<T: Default> Default for Rc<T> { fn default() -> Self { Self::new(T::default()) } }
impl<T> Default for Weak<T> { fn default() -> Self { Self::new() } }
impl<T> From<T> for Rc<T> { fn from(value: T) -> Self { Self::new(value) } }

/// One allocation for the counts and `len` values of `T`, both counts at 1 and the values not yet written.
/// Its layout is the one `layout_for_inner` computes for the returned pointer, so the `Weak` drop frees it exactly.
unsafe fn allocate_slice_rc<T>(len: usize) -> *mut RcInner<[T]> {
    let header = header_layout();
    let val_align = mem::align_of::<T>();
    let pad = header.padding_needed_for(val_align);
    let data_offset = header.size().checked_add(pad).unwrap();
    let slice_size = mem::size_of::<T>().checked_mul(len).unwrap();
    let total_size = data_offset.checked_add(slice_size).unwrap();
    let total_align = if header.align() > val_align { header.align() } else { val_align };
    let total_size = total_size.checked_add(total_align - 1).unwrap() & !(total_align - 1);
    let layout = Layout::from_size_align(total_size, total_align).unwrap();
    let raw = unsafe { alloc::alloc(layout) };
    if raw.is_null() { alloc::handle_alloc_error(layout) }
    let inner_ptr = ptr::slice_from_raw_parts_mut(raw.cast::<T>(), len) as *mut RcInner<[T]>;
    unsafe {
        (&raw mut (*inner_ptr).strong).write(Cell::new(1));
        (&raw mut (*inner_ptr).weak).write(Cell::new(1));
    }
    inner_ptr
}

/// A new allocation holding a clone of each element.
impl<T: Clone> From<&[T]> for Rc<[T]> {
    fn from(values: &[T]) -> Self {
        let inner_ptr = unsafe { allocate_slice_rc::<T>(values.len()) };
        let data_ptr = unsafe { (&raw mut (*inner_ptr).value).cast::<T>() };
        for (i, item) in values.iter().enumerate() {
            unsafe { data_ptr.add(i).write(item.clone()) };
        }
        Self { ptr: unsafe { NonNull::new_unchecked(inner_ptr) } }
    }
}

/// The elements moved into a new allocation; the Vec is left empty and frees only its own buffer.
impl<T> From<crate::vec::Vec<T>> for Rc<[T]> {
    fn from(mut values: crate::vec::Vec<T>) -> Self {
        let len = values.len();
        let inner_ptr = unsafe { allocate_slice_rc::<T>(len) };
        let data_ptr = unsafe { (&raw mut (*inner_ptr).value).cast::<T>() };
        unsafe {
            ptr::copy_nonoverlapping(values.as_ptr(), data_ptr, len);
            values.set_len(0);
        }
        Self { ptr: unsafe { NonNull::new_unchecked(inner_ptr) } }
    }
}

impl<T> From<Box<[T]>> for Rc<[T]> {
    fn from(boxed: Box<[T]>) -> Self { Rc::from(Box::into_vec(boxed)) }
}

/// The bytes copied into a new allocation, read back as a `str` (they came from one, so they are UTF-8).
impl From<&str> for Rc<str> {
    fn from(value: &str) -> Self {
        let bytes: Rc<[u8]> = Rc::from(value.as_bytes());
        let raw = Rc::into_raw(bytes) as *const str;
        unsafe { Rc::from_raw(raw) }
    }
}

impl From<crate::string::String> for Rc<str> {
    fn from(value: crate::string::String) -> Self { Self::from(value.as_str()) }
}

impl From<Box<str>> for Rc<str> {
    fn from(boxed: Box<str>) -> Self { Self::from(&*boxed) }
}

/// The items collected in order into a Vec, then moved into one allocation.
impl<T> core::iter::FromIterator<T> for Rc<[T]> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let items: crate::vec::Vec<T> = iter.into_iter().collect();
        Rc::from(items)
    }
}

impl<T: ?Sized> core::marker::Unpin for Rc<T> {}
impl<T: ?Sized> core::marker::Unpin for Weak<T> {}
