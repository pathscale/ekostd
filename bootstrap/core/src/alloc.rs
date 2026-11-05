use crate::mem::{align_of, size_of};

pub struct Layout {
    size: usize,
    align: usize,
}

impl crate::marker::Copy for Layout {}
impl crate::clone::Clone for Layout {
    fn clone(&self) -> Self { *self }
}

pub struct LayoutError;

impl Layout {
    pub const fn from_size_align(size: usize, align: usize) -> Result<Self, LayoutError> {
        if align == 0 || (align & (align - 1)) != 0 || size > (0x7fff_ffff_ffff_ffff_usize - (align - 1)) {
            Err(LayoutError)
        } else {
            Ok(Self { size, align })
        }
    }

    pub const unsafe fn from_size_align_unchecked(size: usize, align: usize) -> Self {
        Self { size, align }
    }

    pub const fn size(&self) -> usize { self.size }
    pub const fn align(&self) -> usize { self.align }

    pub const fn new<T>() -> Self {
        // SAFETY: Type layouts satisfy the compiler's allocation layout constraints.
        unsafe { Self::from_size_align_unchecked(size_of::<T>(), align_of::<T>()) }
    }

    pub fn array<T>(n: usize) -> Result<Self, LayoutError> {
        let element_size = size_of::<T>();
        if element_size != 0 && n > (0x7fff_ffff_ffff_ffff_usize / element_size) {
            return Err(LayoutError);
        }
        Self::from_size_align(element_size * n, align_of::<T>())
    }

    pub fn for_value<T: ?Sized>(t: &T) -> Self {
        unsafe { Self::for_value_raw(t as *const T) }
    }

    pub unsafe fn for_value_raw<T: ?Sized>(t: *const T) -> Self {
        let size = unsafe { crate::mem::size_of_val(t) };
        let align = unsafe { crate::mem::align_of_val(t) };
        unsafe { Self::from_size_align_unchecked(size, align) }
    }

    pub const fn padding_needed_for(&self, align: usize) -> usize {
        if align == 0 { return 0; }
        let remainder = self.size & (align - 1);
        if remainder == 0 { 0 } else { align - remainder }
    }
}

pub struct AllocError;

pub fn handle_alloc_error(_layout: Layout) -> ! {
    crate::intrinsics::abort()
}

pub unsafe trait GlobalAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8;
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout);

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.alloc(layout) };
        if !ptr.is_null() {
            unsafe { crate::intrinsics::write_bytes(ptr, 0, layout.size()) };
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = match Layout::from_size_align(new_size, layout.align()) {
            Ok(layout) => layout,
            Err(_) => return 0 as *mut u8,
        };
        let new_ptr = unsafe { self.alloc(new_layout) };
        if new_ptr.is_null() { return new_ptr; }
        let copy_size = if layout.size() < new_size { layout.size() } else { new_size };
        unsafe { crate::intrinsics::copy_nonoverlapping(ptr, new_ptr, copy_size) };
        unsafe { self.dealloc(ptr, layout) };
        new_ptr
    }
}

pub unsafe trait Allocator {
    fn allocate(
        &self,
        layout: Layout,
    ) -> Result<crate::ptr::NonNull<[u8]>, AllocError>;

    unsafe fn deallocate(&self, ptr: crate::ptr::NonNull<u8>, layout: Layout);
}
