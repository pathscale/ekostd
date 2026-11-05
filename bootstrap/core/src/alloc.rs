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
        if align == 0 || !align.is_power_of_two() || size > isize::MAX as usize - (align - 1) {
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
}

pub struct AllocError;

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
