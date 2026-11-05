use core::alloc::{Layout, LayoutError};

unsafe extern "Rust" {
    fn __rust_alloc(size: usize, align: usize) -> *mut u8;
    fn __rust_dealloc(ptr: *mut u8, size: usize, align: usize);
    fn __rust_realloc(ptr: *mut u8, old_size: usize, align: usize, new_size: usize) -> *mut u8;
    fn __rust_alloc_zeroed(size: usize, align: usize) -> *mut u8;
}

pub fn layout_array<T>(count: usize) -> Result<Layout, LayoutError> {
    Layout::array::<T>(count)
}

pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    if layout.size() == 0 {
        return layout.align() as *mut u8;
    }
    let ptr = unsafe { __rust_alloc(layout.size(), layout.align()) };
    if ptr.is_null() {
        core::alloc::handle_alloc_error(layout);
    }
    ptr
}

pub unsafe fn alloc_zeroed(layout: Layout) -> *mut u8 {
    if layout.size() == 0 {
        return layout.align() as *mut u8;
    }
    let ptr = unsafe { __rust_alloc_zeroed(layout.size(), layout.align()) };
    if ptr.is_null() {
        core::alloc::handle_alloc_error(layout);
    }
    ptr
}

pub unsafe fn dealloc(ptr: *mut u8, layout: Layout) {
    if layout.size() != 0 {
        unsafe { __rust_dealloc(ptr, layout.size(), layout.align()) };
    }
}

pub unsafe fn realloc(ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    if layout.size() == 0 {
        let new_layout = Layout::from_size_align(new_size, layout.align())
            .unwrap_or_else(|_| core::alloc::handle_alloc_error(layout));
        return unsafe { alloc(new_layout) };
    }
    if new_size == 0 {
        unsafe { dealloc(ptr, layout) };
        return layout.align() as *mut u8;
    }
    let new_layout = Layout::from_size_align(new_size, layout.align())
        .unwrap_or_else(|_| core::alloc::handle_alloc_error(layout));
    let result = unsafe { __rust_realloc(ptr, layout.size(), layout.align(), new_size) };
    if result.is_null() {
        core::alloc::handle_alloc_error(new_layout);
    }
    result
}
