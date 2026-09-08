//! The global allocator, over `malloc`. Named `heap` because `alloc` is the crate.
//!
//! A `no_std` binary has no allocator until something declares one, and `alloc` is useless
//! without it - `Vec`, `String` and `Box` are the compiler's whole working set. `ekod` says
//! `#[global_allocator] static A: Malloc = Malloc;` and this is what it points at.
//!
//! # It is `std`'s, deliberately
//!
//! The bodies below are `library/std/src/sys/alloc/{mod,unix}.rs` with `crate::` paths changed
//! and the platforms we do not build for dropped. Allocator alignment has three separate
//! sharp edges that are easy to miss by reading the C documentation, and std's version already
//! has each one written down:
//!
//! - **`malloc` may return less than `MIN_ALIGN` for a small allocation.** jemalloc does. So the
//!   fast path needs `align <= size` as well as `align <= MIN_ALIGN`; rust-lang/rust#45955.
//! - **Apple's `posix_memalign` returns a misaligned pointer for a very large alignment**, at
//!   least through macOS 10.14 and iOS 13.3; rust-lang/rust#30170. Over `1 << 31` we refuse
//!   rather than hand back memory that does not satisfy the layout.
//! - **`posix_memalign` requires the alignment to be a multiple of `sizeof(void*)`**, which is
//!   why `align` is raised to at least the pointer size rather than passed through. It is
//!   preferred over `aligned_alloc` because that one's supported alignments vary by libc.
//!
//! Reimplementing this from the man pages would have reproduced two of the three bugs.
//!
//! # What is not here
//!
//! No `alloc_error_handler`. `alloc`'s default is to abort, and a compiler that cannot allocate
//! has nothing useful to say; the daemon dying is the ruling for this tree anyway.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

/// The alignment `malloc` guarantees on this platform.
///
/// 16 on 64-bit, which covers every type the compiler allocates except the deliberately
/// over-aligned ones, so the `posix_memalign` path is rare.
const MIN_ALIGN: usize = if cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
    16
} else {
    8
};

/// `malloc`/`free`, as the global allocator.
pub struct Malloc;

// Safe: every method forwards to the C allocator, which satisfies `GlobalAlloc`'s contract for
// the layouts it is given - alignment is handled explicitly below, and a null return is the
// documented failure signal rather than a panic.
unsafe impl GlobalAlloc for Malloc {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() <= MIN_ALIGN && layout.align() <= layout.size() {
            unsafe { libc::malloc(layout.size()) as *mut u8 }
        } else {
            unsafe { aligned_malloc(&layout) }
        }
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if layout.align() <= MIN_ALIGN && layout.align() <= layout.size() {
            unsafe { libc::calloc(layout.size(), 1) as *mut u8 }
        } else {
            let ptr = unsafe { self.alloc(layout) };
            if !ptr.is_null() {
                unsafe { ptr::write_bytes(ptr, 0, layout.size()) };
            }
            ptr
        }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { libc::free(ptr as *mut libc::c_void) }
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if layout.align() <= MIN_ALIGN && layout.align() <= new_size {
            unsafe { libc::realloc(ptr as *mut libc::c_void, new_size) as *mut u8 }
        } else {
            // `realloc` cannot be asked to preserve an alignment it did not choose, so the
            // over-aligned case is allocate, copy, free.
            unsafe {
                let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
                let new_ptr = self.alloc(new_layout);
                if !new_ptr.is_null() {
                    ptr::copy_nonoverlapping(ptr, new_ptr, usize::min(layout.size(), new_size));
                    self.dealloc(ptr, layout);
                }
                new_ptr
            }
        }
    }
}

#[inline]
unsafe fn aligned_malloc(layout: &Layout) -> *mut u8 {
    // See the module comment: Apple's `posix_memalign` mis-answers very large alignments.
    #[cfg(target_vendor = "apple")]
    if layout.align() > (1 << 31) {
        return ptr::null_mut();
    }

    let mut out = ptr::null_mut();
    // `posix_memalign`'s one requirement is that the alignment be a multiple of `sizeof(void*)`.
    // Both are powers of two, so raising to the larger satisfies it.
    let align = layout.align().max(size_of::<usize>());
    let ret = unsafe { libc::posix_memalign(&mut out, align, layout.size()) };
    if ret != 0 { ptr::null_mut() } else { out as *mut u8 }
}
