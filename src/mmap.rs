//! Memory-mapped files.
//!
//! `memmap2` was the only thing left pulling a std dependency into `rustc_data_structures`, and
//! what it provides here is two calls: a private read-only mapping of a file, and an anonymous
//! mapping used as a big allocation.
//!
//! **Private, not shared.** A shared mapping would let a writer change the bytes under a reader,
//! and the whole precondition of `Mmap::map` is that the file does not change while mapped.
//! `MAP_PRIVATE` makes that unenforceable precondition harmless: the mapping is copy-on-write, so
//! a later write to the file cannot be seen through it. This is what rust-lang/rust#122262
//! settled on, and it also fixes cacheless virtiofs.

use core::ops::{Deref, DerefMut};

use crate::Errno;
use crate::file::{Error, File, Result};

/// A read-only private mapping.
pub struct Mmap {
    ptr: *mut libc::c_void,
    len: usize,
}

// The mapping is read-only and owned; nothing in it is thread-affine.
unsafe impl Send for Mmap {}
unsafe impl Sync for Mmap {}

impl Mmap {
    /// Map a file read-only and privately.
    ///
    /// # Safety
    ///
    /// The file must not be truncated while the mapping is live. Writes to it are made harmless
    /// by `MAP_PRIVATE`, but shrinking it leaves the mapping over pages that no longer exist and
    /// touching them raises `SIGBUS`.
    pub unsafe fn map(file: File) -> Result<Mmap> {
        let len = file.metadata()?.len() as usize;
        if len == 0 {
            // `mmap` rejects a zero length. An empty file maps to an empty slice, which is what
            // the caller means and what a zero-length `Vec` would give it.
            return Ok(Mmap { ptr: core::ptr::null_mut(), len: 0 });
        }
        let ptr = unsafe {
            libc::mmap(
                core::ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                file.raw(),
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(Error::from_errno(Errno::current(), "mmap"));
        }
        // The descriptor is not needed once mapped: the mapping holds its own reference to the
        // file. Dropping `file` here closes it, which is why `map` takes it by value.
        Ok(Mmap { ptr, len })
    }
}

impl Deref for Mmap {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        if self.len == 0 {
            return &[];
        }
        unsafe { core::slice::from_raw_parts(self.ptr.cast(), self.len) }
    }
}

impl AsRef<[u8]> for Mmap {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {
        if self.len != 0 {
            unsafe { libc::munmap(self.ptr, self.len) };
        }
    }
}

/// A writable anonymous mapping.
///
/// Used as an allocation rather than as a view of a file: the caller wants `len` bytes it can
/// write and then freeze. Anonymous pages arrive zeroed from the kernel, which the callers rely
/// on and a `Vec::with_capacity` would not give.
pub struct MmapMut {
    ptr: *mut libc::c_void,
    len: usize,
}

unsafe impl Send for MmapMut {}
unsafe impl Sync for MmapMut {}

impl MmapMut {
    /// Map `len` zeroed, writable bytes backed by nothing.
    pub fn map_anon(len: usize) -> Result<MmapMut> {
        if len == 0 {
            return Ok(MmapMut { ptr: core::ptr::null_mut(), len: 0 });
        }
        let ptr = unsafe {
            libc::mmap(
                core::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(Error::from_errno(Errno::current(), "mmap"));
        }
        Ok(MmapMut { ptr, len })
    }

    /// A no-op, kept because callers call it.
    ///
    /// `msync` on an anonymous mapping has nothing to flush to - there is no file behind it. The
    /// call exists in the `memmap2` shape this replaces, and removing it would be a change to
    /// every caller for no effect.
    pub fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    /// Drop write permission, turning this into a [`Mmap`].
    pub fn make_read_only(self) -> Result<Mmap> {
        if self.len != 0
            && unsafe { libc::mprotect(self.ptr, self.len, libc::PROT_READ) } != 0
        {
            return Err(Error::from_errno(Errno::current(), "mprotect"));
        }
        let out = Mmap { ptr: self.ptr, len: self.len };
        // The mapping is now owned by `out`; this one must not unmap it.
        core::mem::forget(self);
        Ok(out)
    }
}

impl Deref for MmapMut {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        if self.len == 0 {
            return &[];
        }
        unsafe { core::slice::from_raw_parts(self.ptr.cast(), self.len) }
    }
}

impl DerefMut for MmapMut {
    fn deref_mut(&mut self) -> &mut [u8] {
        if self.len == 0 {
            return &mut [];
        }
        unsafe { core::slice::from_raw_parts_mut(self.ptr.cast(), self.len) }
    }
}

impl Drop for MmapMut {
    fn drop(&mut self) {
        if self.len != 0 {
            unsafe { libc::munmap(self.ptr, self.len) };
        }
    }
}

// -------------------------------------------------------------------------------------------
// Page protection.
//
// This is the half of `mmap` that the JIT needs and a file reader does not: writing code into a
// page and then taking the write permission away before executing it. It was the `region` crate,
// which is a thin wrapper over `mprotect` and `VirtualProtect` plus a page-size lookup - the same
// two calls, one layer further away.
// -------------------------------------------------------------------------------------------

/// What a page may be used for.
///
/// A set of `PROT_*` bits rather than an enum of the four useful combinations, because that is
/// what `mprotect` takes and an enum would have to be translated back at every call.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Protection(libc::c_int);

impl Protection {
    /// No access at all. Reserving address space without committing to it.
    pub const NONE: Protection = Protection(libc::PROT_NONE);
    /// Readable.
    pub const READ: Protection = Protection(libc::PROT_READ);
    /// Readable and writable.
    pub const READ_WRITE: Protection = Protection(libc::PROT_READ | libc::PROT_WRITE);
    /// Readable and executable. Not writable: a page that is both is what W^X forbids, and on
    /// Apple Silicon the kernel refuses it outright outside a JIT entitlement.
    pub const READ_EXECUTE: Protection = Protection(libc::PROT_READ | libc::PROT_EXEC);

    /// The raw `PROT_*` bits, for a caller that has to add one this type does not name.
    pub fn bits(self) -> libc::c_int {
        self.0
    }

    /// This protection with extra `PROT_*` bits set.
    pub fn with(self, extra: libc::c_int) -> Protection {
        Protection(self.0 | extra)
    }
}

/// The size of a page on this system.
///
/// Cached after the first call. `sysconf` is not expensive, but the JIT's arena asks this per
/// allocation and a relaxed load is free.
pub fn page_size() -> usize {
    use core::sync::atomic::{AtomicUsize, Ordering};
    static CACHED: AtomicUsize = AtomicUsize::new(0);

    // Relaxed on both sides: the value is a constant for the life of the process, so two threads
    // racing here compute the same number and neither publishes anything the other must see.
    let cached = CACHED.load(Ordering::Relaxed);
    if cached != 0 {
        return cached;
    }
    let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    // `sysconf` returning -1 for `_SC_PAGESIZE` would mean a system with no page size, which no
    // caller here could do anything about. 4 KiB is the answer everywhere it could happen.
    let size = if size > 0 { size as usize } else { 4096 };
    CACHED.store(size, Ordering::Relaxed);
    size
}

/// Round `n` up to a whole number of pages.
pub fn page_ceil(n: usize) -> usize {
    let page = page_size();
    n.div_ceil(page) * page
}

/// Change the protection of an already-mapped range.
///
/// # Safety
///
/// `ptr` must be page-aligned and `ptr..ptr + len` must be inside a live mapping the caller owns.
/// Removing an access the caller still relies on turns the next use into `SIGSEGV`, and granting
/// execute to a page some other code can still write is the W^X hole this type exists to make
/// visible.
pub unsafe fn protect(ptr: *mut u8, len: usize, prot: Protection) -> Result<()> {
    if len == 0 {
        return Ok(());
    }
    if unsafe { libc::mprotect(ptr.cast(), len, prot.0) } != 0 {
        return Err(Error::from_errno(Errno::current(), "mprotect"));
    }
    Ok(())
}

/// A reserved range of address space.
///
/// Distinct from [`MmapMut`] because a reservation is not readable: it is mapped `PROT_NONE` and
/// exists so that later allocations get stable addresses without committing memory for them up
/// front. Handing it out as a `&[u8]` would be handing out a slice that faults on the first read,
/// which is why this type has no `Deref`.
pub struct Reservation {
    ptr: *mut libc::c_void,
    len: usize,
}

// The reservation is owned and its pages carry no thread affinity.
unsafe impl Send for Reservation {}
unsafe impl Sync for Reservation {}

impl Reservation {
    /// Reserve `len` bytes of address space, rounded up to a page, with no access.
    ///
    /// `MAP_PRIVATE | MAP_ANON` and nothing else, which is exactly what `region::alloc` passed.
    /// In particular no `MAP_NORESERVE`: `PROT_NONE` already means nothing is committed, and
    /// `region` did not pass it either, so a caller reserving a terabyte gets the behaviour it
    /// had before. `region` also added `MAP_JIT` on Apple silicon, but only for a
    /// write-and-execute protection, which nothing here ever asks for.
    pub fn new(len: usize) -> Result<Reservation> {
        if len == 0 {
            return Ok(Reservation { ptr: core::ptr::null_mut(), len: 0 });
        }
        let len = page_ceil(len);
        let ptr = unsafe {
            libc::mmap(
                core::ptr::null_mut(),
                len,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(Error::from_errno(Errno::current(), "mmap"));
        }
        Ok(Reservation { ptr, len })
    }

    /// The start of the reserved range.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.cast()
    }

    /// How much was reserved, after rounding up to a page.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether nothing was reserved.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        if self.len != 0 {
            unsafe { libc::munmap(self.ptr, self.len) };
        }
    }
}

impl MmapMut {
    /// The start of the mapping.
    ///
    /// `DerefMut` gives the same pointer through a slice, but a caller that only wants the
    /// address should not have to materialise a `&mut [u8]` over pages it is about to make
    /// read-execute.
    pub fn as_ptr_mut(&mut self) -> *mut u8 {
        self.ptr.cast()
    }

    /// How many bytes were mapped.
    pub fn mapped_len(&self) -> usize {
        self.len
    }
}
