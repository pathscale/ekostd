use crate::alloc;
use crate::boxed::Box;
use crate::string::String;
use crate::vec::Vec;
use core::alloc::Layout;
use core::fmt;
use core::marker::{PhantomData, PointeeSized};
use core::mem::{self, ManuallyDrop};
use core::ops::Deref;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering};

#[repr(C)]
struct ArcInner<T: ?Sized> {
    strong: AtomicUsize,
    weak: AtomicUsize,
    value: T,
}

pub struct Arc<T: ?Sized> {
    ptr: NonNull<ArcInner<T>>,
}

pub struct Weak<T: ?Sized> {
    ptr: NonNull<ArcInner<T>>,
}

impl<T: ?Sized> core::ops::LegacyReceiver for Arc<T> {}

unsafe impl<T: ?Sized + Sync + Send> Send for Arc<T> {}
unsafe impl<T: ?Sized + Sync + Send> Sync for Arc<T> {}
unsafe impl<T: ?Sized + Sync + Send> Send for Weak<T> {}
unsafe impl<T: ?Sized + Sync + Send> Sync for Weak<T> {}

#[inline]
fn header_layout() -> Layout {
    Layout::new::<ArcInner<()>>()
}

#[inline]
unsafe fn data_offset_for_ptr<T: ?Sized>(ptr: *const T) -> usize {
    let val_align = unsafe { mem::align_of_val_raw(ptr) };
    let layout = header_layout();
    layout.size() + layout.padding_needed_for(val_align)
}

#[inline]
unsafe fn layout_for_inner<T: ?Sized>(ptr: *const ArcInner<T>) -> Layout {
    let value_ptr = unsafe { &raw const (*ptr).value };
    let size = unsafe { mem::size_of_val_raw(value_ptr) };
    let align = unsafe { mem::align_of_val_raw(value_ptr) };
    let header = header_layout();
    let pad = header.padding_needed_for(align);
    let total_size = match header.size().checked_add(pad) {
        Some(n) => n.checked_add(size).unwrap(),
        None => panic!("layout_for_inner overflow"),
    };
    let total_align = if header.align() > align { header.align() } else { align };
    unsafe { Layout::from_size_align_unchecked(total_size, total_align) }
}

impl<T> Arc<T> {
    pub fn new(value: T) -> Self {
        let boxed = Box::new(ArcInner {
            strong: AtomicUsize::new(1),
            weak: AtomicUsize::new(1),
            value,
        });
        Self { ptr: unsafe { NonNull::new_unchecked(Box::into_raw(boxed)) } }
    }
}

impl<T: ?Sized> Arc<T> {
    #[inline]
    fn inner(&self) -> &ArcInner<T> {
        unsafe { self.ptr.as_ref() }
    }

    pub fn clone(this: &Self) -> Self {
        if this.inner().strong.fetch_add(1, Ordering::Relaxed) > (isize::MAX as usize) {
            panic!("Arc strong count overflow");
        }
        Self { ptr: this.ptr }
    }

    pub fn downgrade(this: &Self) -> Weak<T> {
        let mut cur = this.inner().weak.load(Ordering::Relaxed);
        loop {
            if cur == usize::MAX {
                core::hint::spin_loop();
                cur = this.inner().weak.load(Ordering::Relaxed);
                continue;
            }
            if cur > (isize::MAX as usize) {
                panic!("Arc weak count overflow");
            }
            match this.inner().weak.compare_exchange_weak(cur, cur + 1, Ordering::Acquire, Ordering::Relaxed) {
                Ok(_) => return Weak { ptr: this.ptr },
                Err(actual) => cur = actual,
            }
        }
    }

    pub fn strong_count(this: &Self) -> usize {
        this.inner().strong.load(Ordering::Acquire)
    }

    pub fn weak_count(this: &Self) -> usize {
        let cnt = this.inner().weak.load(Ordering::Acquire);
        if cnt == usize::MAX { 0 } else { cnt.saturating_sub(1) }
    }

    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        ptr::addr_eq(Self::as_ptr(this), Self::as_ptr(other))
    }

    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        if this.is_unique() { unsafe { Some(&mut (*this.ptr.as_ptr()).value) } } else { None }
    }

    #[inline]
    fn is_unique(&mut self) -> bool {
        if self.inner().weak.compare_exchange(1, usize::MAX, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            let unique = self.inner().strong.load(Ordering::Acquire) == 1;
            self.inner().weak.store(1, Ordering::Release);
            unique
        } else {
            false
        }
    }

    pub fn make_mut(this: &mut Self) -> &mut T
    where
        T: Sized + Clone,
    {
        if Self::get_mut(this).is_none() {
            *this = Self::new((**this).clone());
        }
        unsafe { &mut (*this.ptr.as_ptr()).value }
    }

    pub fn as_ptr(this: &Self) -> *const T {
        unsafe { &raw const (*this.ptr.as_ptr()).value }
    }

    pub fn into_raw(this: Self) -> *const T {
        let this = ManuallyDrop::new(this);
        Self::as_ptr(&*this)
    }

    pub unsafe fn from_raw(ptr: *const T) -> Self {
        let offset = unsafe { data_offset_for_ptr(ptr) };
        let byte_ptr = (ptr as *const u8).wrapping_sub(offset);
        let inner_ptr = byte_ptr.with_metadata_of(ptr as *const ArcInner<T>) as *mut ArcInner<T>;
        Self { ptr: unsafe { NonNull::new_unchecked(inner_ptr) } }
    }

    #[inline(never)]
    unsafe fn drop_slow(&mut self) {
        unsafe {
            ptr::drop_in_place(&raw mut (*self.ptr.as_ptr()).value);
            drop(Weak { ptr: self.ptr });
        }
    }
}

impl<T: ?Sized> Deref for Arc<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T { &self.inner().value }
}

impl<T: ?Sized> Clone for Arc<T> {
    #[inline]
    fn clone(&self) -> Self { Self::clone(self) }
}

impl<T: ?Sized> Drop for Arc<T> {
    #[inline]
    fn drop(&mut self) {
        if self.inner().strong.fetch_sub(1, Ordering::Release) != 1 { return; }
        let _ = self.inner().strong.load(Ordering::Acquire);
        unsafe { self.drop_slow() };
    }
}

impl<T: ?Sized> Weak<T> {
    #[inline]
    fn inner(&self) -> Option<&ArcInner<T>> {
        if (self.ptr.as_ptr() as *const u8).addr() == usize::MAX {
            None
        } else {
            Some(unsafe { self.ptr.as_ref() })
        }
    }

    pub fn upgrade(&self) -> Option<Arc<T>> {
        let inner = self.inner()?;
        let mut n = inner.strong.load(Ordering::Relaxed);
        loop {
            if n == 0 { return None; }
            if n > (isize::MAX as usize) { panic!("Arc strong count overflow"); }
            match inner.strong.compare_exchange_weak(n, n + 1, Ordering::Acquire, Ordering::Relaxed) {
                Ok(_) => return Some(Arc { ptr: self.ptr }),
                Err(actual) => n = actual,
            }
        }
    }

    pub fn strong_count(&self) -> usize {
        match self.inner() {
            Some(inner) => inner.strong.load(Ordering::Acquire),
            None => 0,
        }
    }

    pub fn weak_count(&self) -> usize {
        match self.inner() {
            Some(inner) => {
                let weak = inner.weak.load(Ordering::Acquire);
                if inner.strong.load(Ordering::Relaxed) == 0 { 0 } else { weak.saturating_sub(1) }
            }
            None => 0,
        }
    }

    pub fn as_ptr(&self) -> *const T {
        unsafe { &raw const (*self.ptr.as_ptr()).value }
    }

    pub fn into_raw(self) -> *const T {
        ManuallyDrop::new(self).as_ptr()
    }

    pub unsafe fn from_raw(ptr: *const T) -> Self {
        let offset = unsafe { data_offset_for_ptr(ptr) };
        let byte_ptr = (ptr as *const u8).wrapping_sub(offset);
        let inner_ptr = byte_ptr.with_metadata_of(ptr as *const ArcInner<T>) as *mut ArcInner<T>;
        Self { ptr: unsafe { NonNull::new_unchecked(inner_ptr) } }
    }

    pub fn clone(this: &Self) -> Self {
        if let Some(inner) = this.inner() {
            if inner.weak.fetch_add(1, Ordering::Relaxed) > (isize::MAX as usize) {
                panic!("Weak count overflow");
            }
        }
        Self { ptr: this.ptr }
    }
}

impl<T> Weak<T> {
    pub const fn new() -> Self {
        Self { ptr: unsafe { NonNull::new_unchecked(usize::MAX as *mut ArcInner<T>) } }
    }
}

impl<T: ?Sized> Clone for Weak<T> {
    #[inline]
    fn clone(&self) -> Self { Self::clone(self) }
}

impl<T: ?Sized> Drop for Weak<T> {
    fn drop(&mut self) {
        let Some(inner) = self.inner() else { return };
        if inner.weak.fetch_sub(1, Ordering::Release) == 1 {
            let _ = inner.weak.load(Ordering::Acquire);
            unsafe {
                let layout = layout_for_inner(self.ptr.as_ptr());
                alloc::dealloc(self.ptr.as_ptr().cast::<u8>(), layout);
            }
        }
    }
}

impl<T: ?Sized + PartialEq> PartialEq for Arc<T> {
    fn eq(&self, other: &Self) -> bool { **self == **other }
}
impl<T: ?Sized + Eq> Eq for Arc<T> {}
impl<T: ?Sized + PartialOrd> PartialOrd for Arc<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> { (**self).partial_cmp(&**other) }
}
impl<T: ?Sized + Ord> Ord for Arc<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering { (**self).cmp(&**other) }
}
impl<T: ?Sized + core::marker::Unsize<U>, U: ?Sized> core::ops::CoerceUnsized<Arc<U>> for Arc<T> {}
impl<T: ?Sized + core::marker::Unsize<U>, U: ?Sized> core::ops::DispatchFromDyn<Arc<U>> for Arc<T> {}
impl<T: ?Sized + fmt::Debug> fmt::Debug for Arc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Debug::fmt(&**self, f) }
}
impl<T: ?Sized + fmt::Display> fmt::Display for Arc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Display::fmt(&**self, f) }
}
impl<T: Default> Default for Arc<T> {
    fn default() -> Self { Self::new(T::default()) }
}
impl<T: ?Sized> AsRef<T> for Arc<T> {
    fn as_ref(&self) -> &T { &**self }
}
impl<T> From<T> for Arc<T> {
    fn from(value: T) -> Self { Self::new(value) }
}

unsafe fn allocate_slice_arc<T>(len: usize) -> *mut ArcInner<[T]> {
    let header = header_layout();
    let val_align = mem::align_of::<T>();
    let pad = header.padding_needed_for(val_align);
    let data_offset = header.size().checked_add(pad).unwrap();
    let slice_size = mem::size_of::<T>().checked_mul(len).unwrap();
    let total_size = data_offset.checked_add(slice_size).unwrap();
    let total_align = if header.align() > val_align { header.align() } else { val_align };
    let layout = Layout::from_size_align(total_size, total_align).unwrap();
    let raw = unsafe { alloc::alloc(layout) };
    let slice_ptr = ptr::slice_from_raw_parts_mut(raw.cast::<T>(), len);
    let inner_ptr = slice_ptr as *mut ArcInner<[T]>;
    unsafe {
        (&raw mut (*inner_ptr).strong).write(AtomicUsize::new(1));
        (&raw mut (*inner_ptr).weak).write(AtomicUsize::new(1));
    }
    inner_ptr
}

impl<T: Clone> From<&[T]> for Arc<[T]> {
    fn from(values: &[T]) -> Self {
        let inner_ptr = unsafe { allocate_slice_arc::<T>(values.len()) };
        let data_ptr = unsafe { (&raw mut (*inner_ptr).value).cast::<T>() };
        for (i, item) in values.iter().enumerate() {
            unsafe { data_ptr.add(i).write(item.clone()) };
        }
        Self { ptr: unsafe { NonNull::new_unchecked(inner_ptr) } }
    }
}

impl<T> From<Vec<T>> for Arc<[T]> {
    fn from(mut values: Vec<T>) -> Self {
        let len = values.len();
        let inner_ptr = unsafe { allocate_slice_arc::<T>(len) };
        let data_ptr = unsafe { (&raw mut (*inner_ptr).value).cast::<T>() };
        unsafe {
            ptr::copy_nonoverlapping(values.as_ptr(), data_ptr, len);
            values.set_len(0);
        }
        Self { ptr: unsafe { NonNull::new_unchecked(inner_ptr) } }
    }
}

impl From<&str> for Arc<str> {
    fn from(value: &str) -> Self {
        let bytes: Arc<[u8]> = Arc::from(value.as_bytes());
        let raw = Arc::into_raw(bytes) as *const str;
        unsafe { Arc::from_raw(raw) }
    }
}

impl From<String> for Arc<str> {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}
