use crate::clone::Clone;
use crate::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use crate::convert::From;
use crate::intrinsics;
use crate::marker::{Copy, Destruct, Freeze, PhantomData, PointeeSized, Send, Sized, Sync, Unpin, Unsize};
use crate::option::Option::{self, None, Some};

#[lang = "pointee_trait"]
pub trait Pointee: PointeeSized {
    #[lang = "metadata_type"]
    type Metadata: Copy + Send + Sync + Ord + Unpin + Freeze;
}

pub trait Thin: Pointee<Metadata = ()> + PointeeSized {}
impl<T: Pointee<Metadata = ()> + PointeeSized> Thin for T {}

#[repr(C)]
struct FatPtr<T> {
    data: *const T,
    len: usize,
}

#[inline(always)]
pub const fn metadata<T: PointeeSized>(ptr: *const T) -> <T as Pointee>::Metadata {
    let _ = ptr;
    loop {}
}

#[inline(always)]
pub const fn from_raw_parts<T: PointeeSized>(
    data_pointer: *const impl Thin,
    metadata: <T as Pointee>::Metadata,
) -> *const T {
    let _ = (data_pointer, metadata);
    loop {}
}

#[inline(always)]
pub const fn from_raw_parts_mut<T: PointeeSized>(
    data_pointer: *mut impl Thin,
    metadata: <T as Pointee>::Metadata,
) -> *mut T {
    let _ = (data_pointer, metadata);
    loop {}
}

#[rustc_diagnostic_item = "ptr_null"]
#[inline(always)]
pub const fn null<T: Sized>() -> *const T {
    0 as *const T
}

#[rustc_diagnostic_item = "ptr_null_mut"]
#[inline(always)]
pub const fn null_mut<T: Sized>() -> *mut T {
    0 as *mut T
}

#[inline(always)]
pub const fn from_ref<T: PointeeSized>(r: &T) -> *const T {
    r
}

#[inline(always)]
pub const fn from_mut<T: PointeeSized>(r: &mut T) -> *mut T {
    r
}

#[inline(always)]
pub const fn slice_from_raw_parts<T>(data: *const T, len: usize) -> *const [T] {
    unsafe { intrinsics::transmute(FatPtr { data, len }) }
}

#[inline(always)]
pub const fn slice_from_raw_parts_mut<T>(data: *mut T, len: usize) -> *mut [T] {
    unsafe { intrinsics::transmute(FatPtr { data: data as *const T, len }) }
}

#[inline(always)]
pub const unsafe fn read<T>(src: *const T) -> T {
    unsafe { intrinsics::read_via_copy(src) }
}

#[inline(always)]
pub const unsafe fn write<T>(dst: *mut T, src: T) {
    unsafe { intrinsics::write_via_move(dst, src) }
}

#[inline(always)]
pub const unsafe fn copy<T>(src: *const T, dst: *mut T, count: usize) {
    unsafe { intrinsics::copy(src, dst, count) }
}

#[inline(always)]
pub const unsafe fn copy_nonoverlapping<T>(src: *const T, dst: *mut T, count: usize) {
    unsafe { intrinsics::copy_nonoverlapping(src, dst, count) }
}

#[inline(always)]
pub const unsafe fn write_bytes<T>(dst: *mut T, val: u8, count: usize) {
    unsafe { intrinsics::write_bytes(dst, val, count) }
}

#[inline(always)]
pub unsafe fn read_volatile<T>(src: *const T) -> T {
    unsafe { intrinsics::volatile_load(src) }
}

#[inline(always)]
pub unsafe fn write_volatile<T>(dst: *mut T, src: T) {
    unsafe { intrinsics::volatile_store(dst, src) }
}

#[rustc_diagnostic_item = "ptr_drop_in_place"]
#[inline(always)]
pub const unsafe fn drop_in_place<T: PointeeSized>(to_drop: *mut T)
where
    T: [const] Destruct,
{
    unsafe { drop_glue(&mut *to_drop) }
}

#[lang = "drop_glue"]
const unsafe fn drop_glue<T: PointeeSized>(_: &mut T)
where
    T: [const] Destruct,
{
}

#[inline(always)]
pub fn eq<T: PointeeSized>(a: *const T, b: *const T) -> bool {
    a == b
}

#[inline(always)]
pub fn addr_eq<T: PointeeSized, U: PointeeSized>(p: *const T, q: *const U) -> bool {
    (p as *const ()) == (q as *const ())
}

#[repr(transparent)]
pub struct NonNull<T: PointeeSized> {
    pointer: *const T,
}

#[lang = "dyn_metadata"]
#[repr(transparent)]
pub struct DynMetadata<Dyn: PointeeSized> {
    _vtable_ptr: *const (),
    _phantom: PhantomData<Dyn>,
}

impl<Dyn: PointeeSized> Copy for DynMetadata<Dyn> {}
impl<Dyn: PointeeSized> Clone for DynMetadata<Dyn> {
    fn clone(&self) -> Self { *self }
}
impl<Dyn: PointeeSized> PartialEq for DynMetadata<Dyn> {
    fn eq(&self, _: &Self) -> bool { true }
}
impl<Dyn: PointeeSized> Eq for DynMetadata<Dyn> {}
impl<Dyn: PointeeSized> PartialOrd for DynMetadata<Dyn> {
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> { Some(Ordering::Equal) }
}
impl<Dyn: PointeeSized> Ord for DynMetadata<Dyn> {
    fn cmp(&self, _: &Self) -> Ordering { Ordering::Equal }
}
unsafe impl<Dyn: PointeeSized> Send for DynMetadata<Dyn> {}
unsafe impl<Dyn: PointeeSized> Sync for DynMetadata<Dyn> {}
unsafe impl<Dyn: PointeeSized> Freeze for DynMetadata<Dyn> {}
impl<Dyn: PointeeSized> Unpin for DynMetadata<Dyn> {}

impl<T: PointeeSized, U: PointeeSized> crate::ops::CoerceUnsized<NonNull<U>> for NonNull<T>
where
    T: Unsize<U>,
{}

impl<T: PointeeSized, U: PointeeSized> crate::ops::DispatchFromDyn<NonNull<U>> for NonNull<T>
where
    T: Unsize<U>,
{}

impl<T: Sized> NonNull<T> {
    #[inline(always)]
    pub const fn dangling() -> Self {
        unsafe { NonNull::new_unchecked(crate::mem::align_of::<T>() as *mut T) }
    }
}

impl<T: PointeeSized> NonNull<T> {
    #[inline(always)]
    pub const unsafe fn new_unchecked(ptr: *mut T) -> Self {
        unsafe { NonNull { pointer: ptr as *const T } }
    }

    #[inline(always)]
    pub fn new(ptr: *mut T) -> Option<Self> {
        if (ptr as *const u8 as usize) != 0 {
            Some(unsafe { NonNull::new_unchecked(ptr) })
        } else {
            None
        }
    }

    #[inline(always)]
    pub const fn from_ref(r: &T) -> Self {
        NonNull { pointer: r as *const T }
    }

    #[inline(always)]
    pub const fn from_mut(r: &mut T) -> Self {
        NonNull { pointer: r as *mut T as *const T }
    }

    #[inline(always)]
    pub const fn as_ptr(self) -> *mut T {
        self.pointer as *mut T
    }

    #[inline(always)]
    pub const unsafe fn as_ref<'a>(&self) -> &'a T {
        unsafe { &*self.as_ptr() }
    }

    #[inline(always)]
    pub const unsafe fn as_mut<'a>(&mut self) -> &'a mut T {
        unsafe { &mut *self.as_ptr() }
    }

    #[inline(always)]
    pub const fn cast<U>(self) -> NonNull<U> {
        unsafe { NonNull::new_unchecked(self.as_ptr() as *mut U) }
    }

    #[inline(always)]
    pub const unsafe fn offset(self, count: isize) -> Self
    where
        T: Sized,
    {
        unsafe { NonNull::new_unchecked(intrinsics::arith_offset(self.pointer, count) as *mut T) }
    }

    #[inline(always)]
    pub const unsafe fn add(self, count: usize) -> Self
    where
        T: Sized,
    {
        unsafe { self.offset(count as isize) }
    }

    #[inline(always)]
    pub const unsafe fn sub(self, count: usize) -> Self
    where
        T: Sized,
    {
        unsafe { self.offset(0isize.wrapping_sub(count as isize)) }
    }
}

impl<T> NonNull<[T]> {
    #[inline(always)]
    pub const fn slice_from_raw_parts(data: NonNull<T>, len: usize) -> Self {
        unsafe { NonNull::new_unchecked(slice_from_raw_parts_mut(data.as_ptr(), len)) }
    }

    #[inline(always)]
    pub const fn len(self) -> usize {
        self.as_ptr().len()
    }

    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub const fn as_non_null_ptr(self) -> NonNull<T> {
        self.cast()
    }
}

impl<T: PointeeSized> Copy for NonNull<T> {}
impl<T: PointeeSized> Clone for NonNull<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: PointeeSized> !Send for NonNull<T> {}
impl<T: PointeeSized> !Sync for NonNull<T> {}
impl<T: PointeeSized> Eq for NonNull<T> {}
impl<T: PointeeSized> PartialEq for NonNull<T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}
impl<T: PointeeSized> Ord for NonNull<T> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}
impl<T: PointeeSized> PartialOrd for NonNull<T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_ptr().partial_cmp(&other.as_ptr())
    }
}
impl<T: PointeeSized> From<&mut T> for NonNull<T> {
    #[inline(always)]
    fn from(r: &mut T) -> Self {
        NonNull::from_mut(r)
    }
}
impl<T: PointeeSized> From<&T> for NonNull<T> {
    #[inline(always)]
    fn from(r: &T) -> Self {
        NonNull::from_ref(r)
    }
}

#[repr(transparent)]
pub struct Unique<T: PointeeSized> {
    pointer: NonNull<T>,
    _marker: PhantomData<T>,
}

unsafe impl<T: Send + PointeeSized> Send for Unique<T> {}
unsafe impl<T: Sync + PointeeSized> Sync for Unique<T> {}

impl<T: Sized> Unique<T> {
    #[inline(always)]
    pub const fn dangling() -> Self {
        Unique { pointer: NonNull::dangling(), _marker: PhantomData }
    }
}

impl<T: PointeeSized> Unique<T> {
    #[inline(always)]
    pub const unsafe fn new_unchecked(ptr: *mut T) -> Self {
        unsafe { Unique { pointer: NonNull::new_unchecked(ptr), _marker: PhantomData } }
    }

    #[inline(always)]
    pub fn new(ptr: *mut T) -> Option<Self> {
        match NonNull::new(ptr) {
            Some(pointer) => Some(Unique { pointer, _marker: PhantomData }),
            None => None,
        }
    }

    #[inline(always)]
    pub const fn from_non_null(pointer: NonNull<T>) -> Self {
        Unique { pointer, _marker: PhantomData }
    }

    #[inline(always)]
    pub const fn as_ptr(self) -> *mut T {
        self.pointer.as_ptr()
    }

    #[inline(always)]
    pub const fn as_non_null_ptr(self) -> NonNull<T> {
        self.pointer
    }

    #[inline(always)]
    pub const unsafe fn as_ref(&self) -> &T {
        unsafe { self.pointer.as_ref() }
    }

    #[inline(always)]
    pub const unsafe fn as_mut(&mut self) -> &mut T {
        unsafe { self.pointer.as_mut() }
    }

    #[inline(always)]
    pub const fn cast<U>(self) -> Unique<U> {
        Unique { pointer: self.pointer.cast(), _marker: PhantomData }
    }
}

impl<T: PointeeSized> Copy for Unique<T> {}
impl<T: PointeeSized> Clone for Unique<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: PointeeSized> From<NonNull<T>> for Unique<T> {
    #[inline(always)]
    fn from(pointer: NonNull<T>) -> Self {
        Unique::from_non_null(pointer)
    }
}

impl<T: PointeeSized> *const T {
    #[rustc_diagnostic_item = "ptr_const_is_null"]
    #[inline(always)]
    pub fn is_null(self) -> bool {
        (self as *const u8 as usize) == 0
    }

    #[inline(always)]
    pub fn addr(self) -> usize {
        self as *const () as usize
    }

    #[inline(always)]
    pub const fn with_metadata_of<U: PointeeSized>(self, meta: *const U) -> *const U {
        from_raw_parts(self as *const (), metadata(meta))
    }

    #[inline(always)]
    pub const fn cast<U>(self) -> *const U {
        self as *const U
    }

    #[inline(always)]
    pub const fn cast_mut(self) -> *mut T {
        self as *mut T
    }

    #[inline(always)]
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        if self.is_null() { None } else { unsafe { Some(&*self) } }
    }

    #[inline(always)]
    pub const unsafe fn offset(self, count: isize) -> *const T
    where
        T: Sized,
    {
        unsafe { intrinsics::arith_offset(self, count) }
    }

    #[inline(always)]
    pub const fn wrapping_offset(self, count: isize) -> *const T
    where
        T: Sized,
    {
        unsafe { intrinsics::arith_offset(self, count) }
    }

    #[inline(always)]
    pub const unsafe fn add(self, count: usize) -> *const T
    where
        T: Sized,
    {
        unsafe { self.offset(count as isize) }
    }

    #[inline(always)]
    pub const unsafe fn sub(self, count: usize) -> *const T
    where
        T: Sized,
    {
        let neg = 0isize.wrapping_sub(count as isize);
        unsafe { self.offset(neg) }
    }

    #[inline(always)]
    pub const fn wrapping_add(self, count: usize) -> *const T
    where
        T: Sized,
    {
        self.wrapping_offset(count as isize)
    }

    #[inline(always)]
    pub const fn wrapping_sub(self, count: usize) -> *const T
    where
        T: Sized,
    {
        let neg = 0isize.wrapping_sub(count as isize);
        self.wrapping_offset(neg)
    }

    #[inline(always)]
    pub const unsafe fn read(self) -> T
    where
        T: Sized,
    {
        unsafe { read(self) }
    }

    #[inline(always)]
    pub const unsafe fn copy_to(self, dst: *mut T, count: usize)
    where
        T: Sized,
    {
        unsafe { copy(self, dst, count) }
    }

    #[inline(always)]
    pub const unsafe fn copy_to_nonoverlapping(self, dst: *mut T, count: usize)
    where
        T: Sized,
    {
        unsafe { copy_nonoverlapping(self, dst, count) }
    }
}

impl<T: PointeeSized> *mut T {
    #[rustc_diagnostic_item = "ptr_is_null"]
    #[inline(always)]
    pub fn is_null(self) -> bool {
        (self as *const T).is_null()
    }

    #[inline(always)]
    pub fn addr(self) -> usize {
        self as *const () as usize
    }

    #[inline(always)]
    pub const fn with_metadata_of<U: PointeeSized>(self, meta: *const U) -> *mut U {
        from_raw_parts_mut(self as *mut (), metadata(meta))
    }

    #[inline(always)]
    pub const fn cast<U>(self) -> *mut U {
        self as *mut U
    }

    #[inline(always)]
    pub const fn cast_const(self) -> *const T {
        self as *const T
    }

    #[inline(always)]
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        if self.is_null() { None } else { unsafe { Some(&*self) } }
    }

    #[inline(always)]
    pub unsafe fn as_mut<'a>(self) -> Option<&'a mut T> {
        if self.is_null() { None } else { unsafe { Some(&mut *self) } }
    }

    #[inline(always)]
    pub const unsafe fn offset(self, count: isize) -> *mut T
    where
        T: Sized,
    {
        unsafe { intrinsics::arith_offset(self, count) as *mut T }
    }

    #[inline(always)]
    pub const fn wrapping_offset(self, count: isize) -> *mut T
    where
        T: Sized,
    {
        unsafe { intrinsics::arith_offset(self, count) as *mut T }
    }

    #[inline(always)]
    pub const unsafe fn add(self, count: usize) -> *mut T
    where
        T: Sized,
    {
        unsafe { self.offset(count as isize) }
    }

    #[inline(always)]
    pub const unsafe fn sub(self, count: usize) -> *mut T
    where
        T: Sized,
    {
        let neg = 0isize.wrapping_sub(count as isize);
        unsafe { self.offset(neg) }
    }

    #[inline(always)]
    pub const fn wrapping_add(self, count: usize) -> *mut T
    where
        T: Sized,
    {
        self.wrapping_offset(count as isize)
    }

    #[inline(always)]
    pub const fn wrapping_sub(self, count: usize) -> *mut T
    where
        T: Sized,
    {
        let neg = 0isize.wrapping_sub(count as isize);
        self.wrapping_offset(neg)
    }

    #[inline(always)]
    pub const unsafe fn read(self) -> T
    where
        T: Sized,
    {
        unsafe { read(self) }
    }

    #[inline(always)]
    pub const unsafe fn write(self, val: T)
    where
        T: Sized,
    {
        unsafe { write(self, val) }
    }

    #[inline(always)]
    pub const unsafe fn copy_to(self, dst: *mut T, count: usize)
    where
        T: Sized,
    {
        unsafe { copy(self, dst, count) }
    }

    #[inline(always)]
    pub const unsafe fn copy_to_nonoverlapping(self, dst: *mut T, count: usize)
    where
        T: Sized,
    {
        unsafe { copy_nonoverlapping(self, dst, count) }
    }

    #[inline(always)]
    pub const unsafe fn copy_from(self, src: *const T, count: usize)
    where
        T: Sized,
    {
        unsafe { copy(src, self, count) }
    }

    #[inline(always)]
    pub const unsafe fn copy_from_nonoverlapping(self, src: *const T, count: usize)
    where
        T: Sized,
    {
        unsafe { copy_nonoverlapping(src, self, count) }
    }

    #[inline(always)]
    pub const unsafe fn drop_in_place(self)
    where
        T: [const] Destruct,
    {
        unsafe { drop_in_place(self) }
    }

    #[inline(always)]
    pub const unsafe fn write_bytes(self, val: u8, count: usize)
    where
        T: Sized,
    {
        unsafe { write_bytes(self, val, count) }
    }
}

impl<T> *const [T] {
    #[inline(always)]
    pub const fn len(self) -> usize {
        unsafe { intrinsics::transmute::<*const [T], FatPtr<T>>(self).len }
    }

    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub const fn as_ptr(self) -> *const T {
        self as *const T
    }
}

impl<T> *mut [T] {
    #[inline(always)]
    pub const fn len(self) -> usize {
        (self as *const [T]).len()
    }

    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub const fn as_ptr(self) -> *const T {
        self as *const T
    }

    #[inline(always)]
    pub const fn as_mut_ptr(self) -> *mut T {
        self as *mut T
    }
}

impl<T: PointeeSized> PartialEq for *const T {
    #[inline(always)]
    fn eq(&self, other: &*const T) -> bool {
        (*self as *const ()) == (*other as *const ())
    }
}
impl<T: PointeeSized> Eq for *const T {}
impl<T: PointeeSized> PartialOrd for *const T {
    #[inline(always)]
    fn partial_cmp(&self, other: &*const T) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T: PointeeSized> Ord for *const T {
    #[inline(always)]
    fn cmp(&self, other: &*const T) -> Ordering {
        let a = *self as *const () as usize;
        let b = *other as *const () as usize;
        if a < b { Ordering::Less } else if a == b { Ordering::Equal } else { Ordering::Greater }
    }
}

impl<T: PointeeSized> PartialEq for *mut T {
    #[inline(always)]
    fn eq(&self, other: &*mut T) -> bool {
        (*self as *const T) == (*other as *const T)
    }
}
impl<T: PointeeSized> Eq for *mut T {}
impl<T: PointeeSized> PartialOrd for *mut T {
    #[inline(always)]
    fn partial_cmp(&self, other: &*mut T) -> Option<Ordering> {
        (*self as *const T).partial_cmp(&(*other as *const T))
    }
}
impl<T: PointeeSized> Ord for *mut T {
    #[inline(always)]
    fn cmp(&self, other: &*mut T) -> Ordering {
        (*self as *const T).cmp(&(*other as *const T))
    }
}

#[macro_export]
macro_rules! addr_of {
    ($place:expr) => {
        &raw const $place
    };
}

#[macro_export]
macro_rules! addr_of_mut {
    ($place:expr) => {
        &raw mut $place
    };
}

pub use addr_of;
pub use addr_of_mut;
